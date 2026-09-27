use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use winit as W;
use winit::event as WE;
use winit::event_loop as WEL;
use winit::window as WW;

enum Inner {
    Suspended(Option<Arc<WW::Window>>),
    Active {
        window: Arc<WW::Window>,
        webview: wry::WebView,
        _watcher: notify::FsEventWatcher,
    },
}

struct App {
    inner: Inner,
    proxy: WEL::EventLoopProxy<UserEvent>,
    init_script: Vec<String>,
    watch_file: Vec<std::path::PathBuf>,
    config: Config,
}

enum UserEvent {
    Ipc(wry::http::Request<String>),
    Notify(notify::Result<notify::Event>),
    EvalJs(EvalJsResult),
}

impl W::application::ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &WEL::ActiveEventLoop) {
        let Inner::Suspended(cached_window) = &mut self.inner else {
            return;
        };

        let window = cached_window.take().unwrap_or_else(|| {
            Arc::new(
                event_loop
                    .create_window(self.config.window.attributes())
                    .unwrap(),
            )
        });

        let logical_size = window.inner_size().to_logical::<f64>(window.scale_factor());

        let proxy = self.proxy.clone();
        let webview = wry::WebViewBuilder::new()
            .with_html(include_str!("../web/dist/index.html"))
            .with_ipc_handler(move |request| {
                let _ = proxy.send_event(UserEvent::Ipc(request));
            })
            .with_bounds(wry::Rect {
                position: W::dpi::LogicalPosition::new(0.0, 0.0).into(),
                size: logical_size.into(),
            })
            .build_as_child(&window)
            .unwrap();

        let proxy = self.proxy.clone();
        let mut last_change = Instant::now() - Duration::from_secs(1);
        let mut last_kind = None;

        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                let Ok(event) = result else {
                    let _ = proxy.send_event(UserEvent::Notify(result));
                    return;
                };

                std::thread::sleep(Duration::from_millis(2));

                if last_kind.as_ref() != Some(&event.kind) {
                    last_kind = Some(event.kind.clone());
                    let _ = proxy.send_event(UserEvent::Notify(Ok(event)));
                    last_change = Instant::now();
                    return;
                }

                if last_change.elapsed() < Duration::from_millis(10) {
                    return;
                }

                last_change = Instant::now();
                let _ = proxy.send_event(UserEvent::Notify(Ok(event)));
            })
            .unwrap();

        use notify::Watcher;
        self.watch_file.iter().for_each(|p| {
            watcher
                .watch(p, notify::RecursiveMode::NonRecursive)
                .unwrap();
        });

        self.inner = Inner::Active {
            window,
            webview,
            _watcher: watcher,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &WEL::ActiveEventLoop,
        window_id: WW::WindowId,
        event: WE::WindowEvent,
    ) {
        let (window, webview) = match &mut self.inner {
            Inner::Active {
                window, webview, ..
            } if window.id() == window_id => (window, webview),
            _ => return,
        };

        match event {
            WE::WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WE::WindowEvent::Resized(size) => {
                let logical_size = size.to_logical::<f64>(window.scale_factor());
                webview
                    .set_bounds(wry::Rect {
                        position: W::dpi::LogicalPosition::new(0.0, 0.0).into(),
                        size: logical_size.into(),
                    })
                    .unwrap();
            }

            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &WEL::ActiveEventLoop, event: UserEvent) {
        let (_window, webview) = match &mut self.inner {
            Inner::Active {
                window, webview, ..
            } => (window, webview),
            _ => return,
        };

        match event {
            UserEvent::Ipc(req) => {
                let body = req.into_body();
                match body.as_str() {
                    "init" => {
                        self.init_script.iter().for_each(|s| {
                            let proxy = self.proxy.clone();
                            let _ = eval_js(webview, s, move |res| {
                                let _ = proxy.send_event(UserEvent::EvalJs(res));
                            });
                        });
                    }
                    _ => {}
                }
            }
            UserEvent::EvalJs(res) => {
                if let Err(ref e) = res {
                    eprintln!("{:?}", e);
                }
            }
            UserEvent::Notify(res) => {
                let Ok(e) = res else {
                    return;
                };

                if matches!(e.kind, notify::EventKind::Modify(_)) {
                    let Some(path) = e.paths.first() else { return };
                    let Ok(opt) = options_from_file(path) else {
                        return;
                    };
                    let _ = webview.evaluate_script(
                        &PCall {
                            r#type: SET_CHART_OPTION,
                            payload: &opt,
                        }
                        .into_js(),
                    );
                }
            }
        }
    }

    fn suspended(&mut self, _el: &WEL::ActiveEventLoop) {
        if let Inner::Active { window, .. } = &mut self.inner {
            self.inner = Inner::Suspended(Some(window.clone()));
        }
    }
}

const SET_CHART_OPTION: &str = "setChartOption";

#[derive(Debug, Clone, Copy)]
struct PCall<'a> {
    r#type: &'static str,
    payload: &'a str,
}

impl<'a> PCall<'a> {
    fn into_js(self) -> String {
        format!(
            "window.__PORTAL__.dispatch({{ type: '{}', payload: {} }});",
            self.r#type, self.payload
        )
    }
}

use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct EvalJsError {
    pub name: String,
    pub message: String,
    pub stack: Option<String>,
}

pub type EvalJsResult = Result<Option<serde_json::Value>, EvalJsError>;

fn eval_js(
    webview: &wry::WebView,
    script: &str,
    callback: impl Fn(EvalJsResult) + Send + 'static,
) -> wry::Result<()> {
    let wrapped = format!(
        r#"
        (() => {{
            try {{
                const value = eval({:?});

                return JSON.stringify({{
                    ok: true,
                    value
                }});
            }} catch (e) {{
                return JSON.stringify({{
                    ok: false,
                    error: {{
                        name: e.name,
                        message: e.message,
                        stack: e.stack
                    }}
                }});
            }}
        }})()
        "#,
        script
    );

    webview.evaluate_script_with_callback(&wrapped, move |result| {
        let Ok(value) = serde_json::from_str::<String>(&result) else {
            return;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&value) else {
            return;
        };
        if json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            callback(Ok(json.get("value").cloned()));
            return;
        }
        if let Some(e) = json
            .get("error")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
        {
            callback(Err(e));
        }
    })
}

fn options_from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<String> {
    let path = path.as_ref();

    if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
        let file = std::fs::File::open(path)?;
        dataset_from_csv(file)
    } else {
        Ok(std::fs::read_to_string(path)?)
    }
}

fn dataset_from_csv(reader: impl std::io::Read) -> anyhow::Result<String> {
    #[inline]
    fn write_row(out: &mut String, record: &csv::StringRecord) {
        out.push('[');
        let mut first = true;
        for field in record.iter() {
            if !first {
                out.push(',');
            }
            first = false;
            write_field(out, field);
        }
        out.push(']');
    }

    #[inline]
    fn write_field(out: &mut String, field: &str) {
        let s = field.trim();

        if !s.is_empty() && s.parse::<f64>().is_ok() {
            out.push_str(s);
            return;
        }

        out.push('\'');
        if s.as_bytes().contains(&b'\'') {
            for c in s.chars() {
                if c == '\'' {
                    out.push('\\');
                }
                out.push(c);
            }
        } else {
            out.push_str(s);
        }
        out.push('\'');
    }

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);

    let mut out = String::with_capacity(8192);
    out.push_str("{dataset:{source:[");

    let mut record = csv::StringRecord::new();

    if !csv_reader.read_record(&mut record)? {
        anyhow::bail!("CSV is empty");
    }
    write_row(&mut out, &record);

    while csv_reader.read_record(&mut record)? {
        out.push(',');
        write_row(&mut out, &record);
    }

    out.push_str("]}}");
    Ok(out)
}

pub fn run(args: crate::cli::Args, config: Option<crate::config::Config>) -> anyhow::Result<()> {
    let config = config.unwrap_or_default();

    let event_loop = WEL::EventLoop::<UserEvent>::with_user_event()
        .build()
        .unwrap();

    let mut init_script = Vec::new();
    for (path, _) in args.opt_paths.iter() {
        let script = options_from_file(path)?;
        init_script.push(
            PCall {
                r#type: SET_CHART_OPTION,
                payload: &script,
            }
            .into_js(),
        );
    }

    let mut app = App {
        inner: Inner::Suspended(None),
        proxy: event_loop.create_proxy(),
        init_script,
        watch_file: args
            .opt_paths
            .iter()
            .filter(|(_, f)| *f)
            .map(|v| v.0.clone())
            .collect(),
        config,
    };

    Ok(event_loop.run_app(&mut app)?)
}
