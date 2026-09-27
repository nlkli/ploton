use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use winit as W;
use winit::event as WE;
use winit::event_loop as WEL;
use winit::keyboard as WK;
use winit::window as WW;

mod app;
mod cli;
mod config;

// type Result<T> = anyhow::Result<T>;

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
    init_script: Option<String>,
    watch_file: Option<std::path::PathBuf>,
}

impl W::application::ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &WEL::ActiveEventLoop) {
        let Inner::Suspended(cached_window) = &mut self.inner else {
            return;
        };

        let window = cached_window.take().unwrap_or_else(|| {
            Arc::new(
                event_loop
                    .create_window(
                        WW::Window::default_attributes()
                            .with_title("app")
                            .with_inner_size(W::dpi::LogicalSize::new(800, 600)),
                    )
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

                std::thread::sleep(Duration::from_millis(5));

                if last_kind.as_ref() != Some(&event.kind) {
                    last_kind = Some(event.kind.clone());
                    let _ = proxy.send_event(UserEvent::Notify(Ok(event)));
                    last_change = Instant::now();
                    return;
                }

                if last_change.elapsed() < Duration::from_millis(200) {
                    return;
                }

                last_change = Instant::now();
                let _ = proxy.send_event(UserEvent::Notify(Ok(event)));
            })
            .unwrap();

        use notify::Watcher;
        if let Some(ref path) = self.watch_file {
            watcher
                .watch(path, notify::RecursiveMode::NonRecursive)
                .unwrap();
        }

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

            WE::WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                WK::PhysicalKey::Code(key_code) => match key_code {
                    WK::KeyCode::KeyH => {
                        let r = std::fs::File::open("./resources/line-2.csv").unwrap();
                        let ds = dataset_from_csv(r).unwrap();
                        let _ = webview.evaluate_script(
                            &PCall {
                                r#type: SET_CHART_OPTION,
                                payload: &ds,
                            }
                            .into_js(),
                        );
                    }
                    _ => {}
                },

                WK::PhysicalKey::Unidentified(_native_key_code) => {}
            },

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
                        if let Some(script) = self.init_script.as_deref() {
                            let proxy = self.proxy.clone();
                            let _ = eval_js(webview, script, move |res| {
                                let _ = proxy.send_event(UserEvent::EvalJs(res));
                            });
                        }
                    }
                    _ => {}
                }
            }
            UserEvent::EvalJs(res) => {
                println!("eval result: {:?}", res);
            }
            UserEvent::Notify(res) => {
                let Ok(e) = res else {
                    return;
                };

                if matches!(e.kind, notify::EventKind::Modify(_)) {
                    let Some(path) = e.paths.first() else { return };
                    let Ok(file) = std::fs::File::open(path) else {
                        return;
                    };
                    let Some(ds) = dataset_from_csv(file) else {
                        return;
                    };
                    let _ = webview.evaluate_script(
                        &PCall {
                            r#type: SET_CHART_OPTION,
                            payload: &ds,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct JsError {
    pub name: String,
    pub message: String,
    pub stack: Option<String>,
}

pub type EvalResult = Result<Option<serde_json::Value>, JsError>;

fn eval_js(
    webview: &wry::WebView,
    script: &str,
    callback: impl Fn(EvalResult) + Send + 'static,
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

enum UserEvent {
    Ipc(wry::http::Request<String>),
    Notify(notify::Result<notify::Event>),
    EvalJs(EvalResult),
}

fn options_from_file(path: impl AsRef<std::path::Path>) {
    todo!()
}

fn dataset_from_csv(reader: impl std::io::Read) -> Option<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    let mut records = reader.records();

    let header = records.next()?.ok()?;
    let hs = header
        .iter()
        .map(|s| format!("'{}'", s.trim()))
        .collect::<Vec<_>>()
        .join(",");

    let mut rows = Vec::new();
    rows.push(format!("[{hs}]"));

    for result in records {
        let record = result.ok()?;
        let row = record
            .iter()
            .map(|s| {
                let s = s.trim();
                if s.parse::<f64>().is_ok() {
                    s.to_string()
                } else {
                    format!("'{s}'")
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        rows.push(format!("[{row}]"));
    }

    Some(format!("{{dataset:{{source:[{}]}}}}", rows.join(",")))
}

fn main() {
    let event_loop = WEL::EventLoop::<UserEvent>::with_user_event()
        .build()
        .unwrap();

    let mut app = App {
        inner: Inner::Suspended(None),
        proxy: event_loop.create_proxy(),
        init_script: None,
        watch_file: None,
    };
    app.init_script.replace(
        PCall {
            r#type: SET_CHART_OPTION,
            payload: r#"function () {return {
grid: { left: '3%', right: '3%' },
  xAxis: { type: 'category' },
  yAxis: { },
  series: [{ type: 'bar' }]
};}()"#,
        }
        .into_js(),
    );

    app.watch_file
        .replace(std::path::PathBuf::from("./resources/line-2.csv"));

    event_loop.run_app(&mut app).unwrap();
}
