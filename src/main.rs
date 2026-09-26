use std::sync::Arc;

use winit as W;
use winit::event as WE;
use winit::event_loop as WEL;
use winit::window as WW;

enum Inner {
    Suspended(Option<Arc<WW::Window>>),
    Active {
        window: Arc<WW::Window>,
        webview: wry::WebView,
    },
}

struct App {
    inner: Inner,
    proxy: WEL::EventLoopProxy<UserEvent>,
    init: Option<String>,
}

// fn ipc_send(msg: &str, wv: &wry::WebView) -> wry::Result<()> {
//     wv.evaluate_script(&format!("window.recvIpcMessage({msg});"))
// }

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
            .with_html(include_str!("../app/dist/index.html"))
            .with_ipc_handler(move |request| {
                let _ = proxy.send_event(UserEvent::Ipc(request));
            })
            .with_bounds(wry::Rect {
                position: W::dpi::LogicalPosition::new(0.0, 0.0).into(),
                size: logical_size.into(),
            })
            .build_as_child(&window)
            .unwrap();

        self.inner = Inner::Active { window, webview };
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

            WE::WindowEvent::KeyboardInput { event, .. } => {
                let _event = event;
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
            UserEvent::Ipc(_req) => {
                if let Some(opt) = self.init.take() {
                    let _ = webview.evaluate_script(&format!("window.setChartOption({opt});"));
                    return;
                }
            } // _ => {}
        }
    }

    fn suspended(&mut self, _el: &WEL::ActiveEventLoop) {
        if let Inner::Active { window, .. } = &mut self.inner {
            self.inner = Inner::Suspended(Some(window.clone()));
        }
    }
}

enum UserEvent {
    Ipc(wry::http::Request<String>),
}

fn main() {
    let event_loop = WEL::EventLoop::<UserEvent>::with_user_event()
        .build()
        .unwrap();

    let mut app = App {
        inner: Inner::Suspended(None),
        proxy: event_loop.create_proxy(),
        init: None,
    };
    app.init.replace(
        "{
  xAxis: {
    type: 'category',
    data: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
  },
  yAxis: {
    type: 'value'
  },
  series: [
    {
      data: [150, 230, 224, 218, 135, 147, 260],
      type: 'line'
    }
  ]
}"
        .into(),
    );

    event_loop.run_app(&mut app).unwrap();
}
