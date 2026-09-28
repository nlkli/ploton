use std::{
    fmt::Write as _,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::{cli::Args, config::Config};

/// Repeated file events of the same kind within this interval are dropped.
const DEBOUNCE: Duration = Duration::from_millis(10);
/// Gives the writer a moment to finish before the file is read.
const SETTLE_DELAY: Duration = Duration::from_millis(2);

enum Inner {
    /// No webview; keeps the window so it can be reused on resume.
    Suspended(Option<Arc<Window>>),
    Active {
        window: Arc<Window>,
        webview: wry::WebView,
        _watcher: RecommendedWatcher,
    },
}

enum UserEvent {
    Ipc(wry::http::Request<String>),
    Notify(notify::Result<notify::Event>),
    EvalJs(EvalJsResult),
}

struct App {
    inner: Inner,
    proxy: EventLoopProxy<UserEvent>,
    /// Scripts sent to the page once it reports it is ready.
    init_scripts: Vec<String>,
    watch_paths: Vec<PathBuf>,
    config: Config,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Inner::Suspended(cached_window) = &mut self.inner else {
            return;
        };

        let window = cached_window.take().unwrap_or_else(|| {
            Arc::new(
                event_loop
                    .create_window(self.config.window.attributes())
                    .expect("failed to create window"),
            )
        });

        let proxy = self.proxy.clone();
        let webview = wry::WebViewBuilder::new()
            .with_html(include_str!("../web/dist/index.html"))
            .with_ipc_handler(move |request| {
                let _ = proxy.send_event(UserEvent::Ipc(request));
            })
            .with_bounds(bounds(window.inner_size(), window.scale_factor()))
            .build_as_child(&window)
            .expect("failed to create webview");

        let watcher = create_watcher(self.proxy.clone(), &self.watch_paths);

        self.inner = Inner::Active {
            window,
            webview,
            _watcher: watcher,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Inner::Active {
            window, webview, ..
        } = &self.inner
        else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => webview
                .set_bounds(bounds(size, window.scale_factor()))
                .expect("failed to resize webview"),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let Inner::Active { webview, .. } = &self.inner else {
            return;
        };

        match event {
            UserEvent::Ipc(request) => {
                // The page asks for its options once it has loaded.
                if request.body().as_str() == "init" {
                    for script in &self.init_scripts {
                        let proxy = self.proxy.clone();
                        let _ = eval_js(webview, script, move |result| {
                            let _ = proxy.send_event(UserEvent::EvalJs(result));
                        });
                    }
                }
            }
            UserEvent::EvalJs(Err(error)) => {
                eprintln!("{error:?}");
                event_loop.exit();
            }
            UserEvent::EvalJs(Ok(_)) => {}
            UserEvent::Notify(Ok(event)) if matches!(event.kind, EventKind::Modify(_)) => {
                let Some(path) = event.paths.first() else {
                    return;
                };
                // The file may be mid-write or invalid: keep the last good chart.
                let Ok(option) = options_from_file(path) else {
                    return;
                };
                let _ = webview.evaluate_script(&set_chart_option_js(&option));
            }
            UserEvent::Notify(_) => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        let Inner::Active { window, .. } = &self.inner else {
            return;
        };
        let window = window.clone();
        self.inner = Inner::Suspended(Some(window));
    }
}

/// Bounds that make the webview fill the whole window.
fn bounds(size: PhysicalSize<u32>, scale_factor: f64) -> wry::Rect {
    wry::Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: size.to_logical::<f64>(scale_factor).into(),
    }
}

/// Watches `paths` and forwards de-duplicated events to the event loop.
fn create_watcher(proxy: EventLoopProxy<UserEvent>, paths: &[PathBuf]) -> RecommendedWatcher {
    let mut last_kind: Option<EventKind> = None;
    let mut last_change = Instant::now();

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let event = match result {
            Ok(event) => event,
            Err(error) => {
                let _ = proxy.send_event(UserEvent::Notify(Err(error)));
                return;
            }
        };

        std::thread::sleep(SETTLE_DELAY);

        let same_kind = last_kind.as_ref() == Some(&event.kind);
        if same_kind && last_change.elapsed() < DEBOUNCE {
            return;
        }
        last_kind = Some(event.kind.clone());
        last_change = Instant::now();
        let _ = proxy.send_event(UserEvent::Notify(Ok(event)));
    })
    .expect("failed to create file watcher");

    for path in paths {
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .expect("failed to watch file");
    }
    watcher
}

/// JS that hands a chart option (a JS expression) to the page.
/// Must match the `setChartOption` handler in `web/src/main.ts`.
fn set_chart_option_js(option: &str) -> String {
    format!("window.__PORTAL__.dispatch({{ type: 'setChartOption', payload: {option} }});")
}

#[derive(Debug, Deserialize)]
struct EvalJsError {
    name: String,
    message: String,
    stack: Option<String>,
}

type EvalJsResult = Result<Option<serde_json::Value>, EvalJsError>;

/// Evaluates `script` in the webview and reports the value or the JS error.
fn eval_js(
    webview: &wry::WebView,
    script: &str,
    callback: impl Fn(EvalJsResult) + Send + 'static,
) -> wry::Result<()> {
    // A JSON string is a valid JS string literal.
    let literal = serde_json::to_string(script).expect("a string always serializes");
    let wrapped = format!(
        r#"
        (() => {{
            try {{
                const value = eval({literal});
                return JSON.stringify({{ ok: true, value }});
            }} catch (e) {{
                return JSON.stringify({{
                    ok: false,
                    error: {{ name: e.name, message: e.message, stack: e.stack }}
                }});
            }}
        }})()
        "#
    );

    webview.evaluate_script_with_callback(&wrapped, move |result| {
        // `result` is a JSON string holding our own JSON payload.
        let Ok(payload) = serde_json::from_str::<String>(&result) else {
            return;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) else {
            return;
        };

        if json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            callback(Ok(json.get("value").cloned()));
        } else if let Some(error) = json
            .get("error")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
        {
            callback(Err(error));
        }
    })
}

/// Loads a chart option: CSV files become a dataset, anything else is used as-is.
fn options_from_file(path: &Path) -> Result<String> {
    if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
        dataset_from_csv(File::open(path)?)
    } else {
        Ok(fs::read_to_string(path)?)
    }
}

/// Converts CSV into `{dataset:{source:[[...],...]}}`; the first row is kept as the header.
fn dataset_from_csv(reader: impl Read) -> Result<String> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);

    let mut out = String::with_capacity(8192);
    out.push_str("{dataset:{source:[");

    let mut record = csv::StringRecord::new();
    let mut first_row = true;
    while csv_reader.read_record(&mut record)? {
        if !first_row {
            out.push(',');
        }
        first_row = false;
        push_row(&mut out, &record);
    }

    if first_row {
        anyhow::bail!("CSV is empty");
    }
    out.push_str("]}}");
    Ok(out)
}

fn push_row(out: &mut String, record: &csv::StringRecord) {
    out.push('[');
    for (i, field) in record.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_field(out, field);
    }
    out.push(']');
}

fn push_field(out: &mut String, field: &str) {
    let field = field.trim();
    match field.parse::<f64>() {
        // Re-format numbers so "007" is not read as an octal literal by JS.
        // NaN and infinity are not valid JS literals, so they stay strings.
        Ok(n) if n.is_finite() => {
            let _ = write!(out, "{n}");
        }
        _ => push_js_string(out, field),
    }
}

/// Appends `s` as a single-quoted JS string literal.
fn push_js_string(out: &mut String, s: &str) {
    out.push('\'');
    for c in s.chars() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('\'');
}

pub fn run(args: Args, config: Config) -> Result<()> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;

    let init_scripts = args
        .files
        .iter()
        .map(|file| options_from_file(&file.path).map(|option| set_chart_option_js(&option)))
        .collect::<Result<Vec<_>>>()?;

    let watch_paths = args
        .files
        .iter()
        .filter(|file| file.watch)
        .map(|file| file.path.clone())
        .collect();

    let mut app = App {
        inner: Inner::Suspended(None),
        proxy: event_loop.create_proxy(),
        init_scripts,
        watch_paths,
        config,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}
