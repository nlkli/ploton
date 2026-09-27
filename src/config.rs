use anyhow::Result;
use configparser::ini::Ini;
use winit::window as WW;

macro_rules! parse_string {
    ($ini:expr, $section:expr, $key:expr, $target:expr) => {
        if let Some(v) = $ini.get($section, $key) {
            $target = v;
        }
    };
}

macro_rules! parse_number {
    ($ini:expr, $section:expr, $key:expr, $target:expr) => {
        $target = $ini
            .get($section, $key)
            .and_then(|v| v.parse().ok())
            .unwrap_or($target);
    };
}

macro_rules! parse_bool {
    ($ini:expr, $section:expr, $key:expr, $target:expr) => {
        if let Ok(Some(v)) = $ini.getboolcoerce($section, $key) {
            $target = v;
        }
    };
}

#[derive(Clone, Debug)]
pub struct Window {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
    pub fullscreen: bool,
    pub resizable: bool,
    pub active: bool,
    pub visible: bool,
    pub level: String,
    pub decorations: bool,
    pub transparent: bool,
    pub blur: bool,
    pub maximized: bool,
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,

    // macOS
    pub macos_titlebar_transparent: bool,
    pub macos_titlebar_hidden: bool,
    pub macos_titlebar_buttons_hidden: bool,
    pub macos_title_hidden: bool,
    pub macos_fullsize_content_view: bool,
    pub macos_has_shadow: bool,
    // // Windows
    // pub windows_skip_taskbar: bool,
    // pub windows_undecorated_shadow: bool,
    // pub windows_corner_preference: Option<String>,
    // pub windows_class_name: Option<String>,
    // pub windows_border_color: Option<String>,
    // pub windows_title_background_color: Option<String>,
    // pub windows_title_text_color: Option<String>,

    // // X11
    // pub x11_general_name: Option<String>,
    // pub x11_instance_name: Option<String>,
    // pub x11_override_redirect: bool,
    // pub x11_window_type: Option<String>,
    // pub x11_visual_id: Option<u32>,
    // pub x11_screen_id: Option<i32>,
    // pub x11_embed_parent_window: Option<u32>,

    // // Wayland
    // pub wayland_general_name: Option<String>,
    // pub wayland_instance_name: Option<String>,
    // pub wayland_activation_token: Option<String>,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: "ploton".into(),
            width: 800,
            height: 600,
            x: 0,
            y: 0,
            fullscreen: false,
            resizable: true,
            active: true,
            visible: true,
            level: "normal".into(),
            decorations: true,
            transparent: false,
            blur: false,
            maximized: false,
            min_width: 0,
            min_height: 0,
            max_width: 0,
            max_height: 0,

            // macOS
            macos_titlebar_transparent: false,
            macos_titlebar_hidden: false,
            macos_titlebar_buttons_hidden: false,
            macos_title_hidden: false,
            macos_fullsize_content_view: false,
            macos_has_shadow: true,
            // // Windows
            // windows_skip_taskbar: false,
            // windows_undecorated_shadow: true,
            // windows_corner_preference: None,
            // windows_class_name: None,
            // windows_border_color: None,
            // windows_title_background_color: None,
            // windows_title_text_color: None,

            // // X11
            // x11_general_name: None,
            // x11_instance_name: None,
            // x11_override_redirect: false,
            // x11_window_type: None,
            // x11_visual_id: None,
            // x11_screen_id: None,
            // x11_embed_parent_window: None,

            // // Wayland
            // wayland_general_name: None,
            // wayland_instance_name: None,
            // wayland_activation_token: None,
        }
    }
}

impl Window {
    pub fn attributes(&self) -> WW::WindowAttributes {
        let w = &self;

        let mut attr = WW::Window::default_attributes()
            .with_title(&w.title)
            .with_inner_size(winit::dpi::LogicalSize::new(w.width, w.height))
            .with_resizable(w.resizable)
            .with_visible(w.visible)
            .with_decorations(w.decorations)
            .with_transparent(w.transparent)
            .with_active(w.active)
            .with_blur(w.blur)
            .with_maximized(w.maximized)
            .with_fullscreen(if w.fullscreen {
                Some(WW::Fullscreen::Borderless(None))
            } else {
                None
            });

        if w.x > 0 || w.y > 0 {
            attr = attr.with_position(winit::dpi::LogicalPosition::new(w.x, w.y));
        }

        let mut min_inner_size = winit::dpi::LogicalSize::new(1, 1);
        if w.min_width > 0 {
            let min_width = w.min_width.max(1);
            min_inner_size.width = min_width;
        }
        if w.min_height > 0 {
            let min_height = w.min_height.max(1);
            min_inner_size.height = min_height;
        }
        attr = attr.with_min_inner_size(min_inner_size);

        if w.max_width > 0 || w.max_height > 0 {
            let mut max_inner_size = winit::dpi::LogicalSize::new(u32::MAX, u32::MAX);
            if w.max_width > 0 {
                max_inner_size.width = w.max_width.max(2);
            }
            if w.max_height > 0 {
                max_inner_size.height = w.max_height.max(2);
            }
            attr = attr.with_max_inner_size(max_inner_size);
        }

        if !w.level.is_empty() {
            match w.level.to_lowercase().trim() {
                "normal" => attr = attr.with_window_level(WW::WindowLevel::Normal),
                "alwaysonbottom" => attr = attr.with_window_level(WW::WindowLevel::AlwaysOnBottom),
                "alwaysontop" => attr = attr.with_window_level(WW::WindowLevel::AlwaysOnTop),
                _ => {}
            }
        }

        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowAttributesExtMacOS;

            attr = attr
                .with_titlebar_transparent(w.macos_titlebar_transparent)
                .with_titlebar_hidden(w.macos_titlebar_hidden)
                .with_titlebar_buttons_hidden(w.macos_titlebar_buttons_hidden)
                .with_title_hidden(w.macos_title_hidden)
                .with_fullsize_content_view(w.macos_fullsize_content_view)
                .with_has_shadow(w.macos_has_shadow);
        }

        attr
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub path: Option<std::path::PathBuf>,
    pub window: Window,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: None,
            window: Window::default(),
        }
    }
}

impl Config {
    pub fn from_ini_file(
        path: impl AsRef<std::path::Path>,
        default: Option<Config>,
    ) -> Result<Self> {
        let mut ini = Ini::new();
        ini.load(&path).map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut config = default.unwrap_or_default();
        config.path.replace(path.as_ref().to_path_buf());

        parse_string!(ini, "window", "title", config.window.title);
        parse_number!(ini, "window", "width", config.window.width);
        parse_number!(ini, "window", "height", config.window.height);
        parse_bool!(ini, "window", "fullscreen", config.window.fullscreen);
        parse_bool!(ini, "window", "resizable", config.window.resizable);
        parse_bool!(ini, "window", "active", config.window.active);
        parse_bool!(ini, "window", "visible", config.window.visible);
        parse_bool!(ini, "window", "decorations", config.window.decorations);
        parse_bool!(ini, "window", "transparent", config.window.transparent);
        parse_bool!(ini, "window", "blur", config.window.blur);
        parse_bool!(ini, "window", "maximized", config.window.maximized);
        parse_number!(ini, "window", "min_width", config.window.min_width);
        parse_number!(ini, "window", "min_height", config.window.min_height);
        parse_number!(ini, "window", "max_width", config.window.max_width);
        parse_number!(ini, "window", "max_height", config.window.max_height);

        parse_bool!(
            ini,
            "window",
            "macos_titlebar_transparent",
            config.window.macos_titlebar_transparent
        );
        parse_bool!(
            ini,
            "window",
            "macos_titlebar_hidden",
            config.window.macos_titlebar_hidden
        );
        parse_bool!(
            ini,
            "window",
            "macos_titlebar_buttons_hidden",
            config.window.macos_titlebar_buttons_hidden
        );
        parse_bool!(
            ini,
            "window",
            "macos_title_hidden",
            config.window.macos_title_hidden
        );
        parse_bool!(
            ini,
            "window",
            "macos_fullsize_content_view",
            config.window.macos_fullsize_content_view
        );
        parse_bool!(
            ini,
            "window",
            "macos_has_shadow",
            config.window.macos_has_shadow
        );

        Ok(config)
    }
}
