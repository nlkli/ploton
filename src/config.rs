use std::{path::Path, str::FromStr};

use anyhow::{Result, anyhow};
use configparser::ini::Ini;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{Fullscreen, Window as WinitWindow, WindowAttributes, WindowLevel};

/// Reads typed values from one INI section, leaving the target untouched
/// when the key is missing or invalid.
struct Section<'a> {
    ini: &'a Ini,
    name: &'a str,
}

impl Section<'_> {
    fn string(&self, key: &str, target: &mut String) {
        if let Some(v) = self.ini.get(self.name, key) {
            *target = v;
        }
    }

    fn number<T: FromStr>(&self, key: &str, target: &mut T) {
        if let Some(v) = self
            .ini
            .get(self.name, key)
            .and_then(|v| v.parse::<T>().ok())
        {
            *target = v;
        }
    }

    fn boolean(&self, key: &str, target: &mut bool) {
        if let Ok(Some(v)) = self.ini.getboolcoerce(self.name, key) {
            *target = v;
        }
    }
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
    /// "normal", "alwaysontop" or "alwaysonbottom".
    pub level: String,
    pub decorations: bool,
    pub transparent: bool,
    pub blur: bool,
    pub maximized: bool,
    /// Size limits; 0 means "no limit".
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,

    // macOS only
    pub macos_titlebar_transparent: bool,
    pub macos_titlebar_hidden: bool,
    pub macos_titlebar_buttons_hidden: bool,
    pub macos_title_hidden: bool,
    pub macos_fullsize_content_view: bool,
    pub macos_has_shadow: bool,
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

            macos_titlebar_transparent: false,
            macos_titlebar_hidden: false,
            macos_titlebar_buttons_hidden: false,
            macos_title_hidden: false,
            macos_fullsize_content_view: false,
            macos_has_shadow: true,
        }
    }
}

impl Window {
    pub fn attributes(&self) -> WindowAttributes {
        let mut attr = WinitWindow::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.width, self.height))
            .with_resizable(self.resizable)
            .with_visible(self.visible)
            .with_decorations(self.decorations)
            .with_transparent(self.transparent)
            .with_active(self.active)
            .with_blur(self.blur)
            .with_maximized(self.maximized)
            .with_fullscreen(self.fullscreen.then(|| Fullscreen::Borderless(None)))
            .with_min_inner_size(LogicalSize::new(
                self.min_width.max(1),
                self.min_height.max(1),
            ));

        // (0, 0) means "let the OS decide".
        if self.x > 0 || self.y > 0 {
            attr = attr.with_position(LogicalPosition::new(self.x, self.y));
        }

        if self.max_width > 0 || self.max_height > 0 {
            let limit = |value: u32| if value > 0 { value.max(2) } else { u32::MAX };
            attr = attr.with_max_inner_size(LogicalSize::new(
                limit(self.max_width),
                limit(self.max_height),
            ));
        }

        let level = match self.level.trim().to_lowercase().as_str() {
            "normal" => Some(WindowLevel::Normal),
            "alwaysonbottom" => Some(WindowLevel::AlwaysOnBottom),
            "alwaysontop" => Some(WindowLevel::AlwaysOnTop),
            _ => None,
        };
        if let Some(level) = level {
            attr = attr.with_window_level(level);
        }

        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowAttributesExtMacOS;

            attr = attr
                .with_titlebar_transparent(self.macos_titlebar_transparent)
                .with_titlebar_hidden(self.macos_titlebar_hidden)
                .with_titlebar_buttons_hidden(self.macos_titlebar_buttons_hidden)
                .with_title_hidden(self.macos_title_hidden)
                .with_fullsize_content_view(self.macos_fullsize_content_view)
                .with_has_shadow(self.macos_has_shadow);
        }

        attr
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub window: Window,
}

impl Config {
    /// Loads the `[window]` section of an INI file over the defaults.
    #[allow(dead_code)] // Not wired to the CLI yet.
    pub fn from_ini_file(path: impl AsRef<Path>) -> Result<Self> {
        let mut ini = Ini::new();
        ini.load(path.as_ref()).map_err(|e| anyhow!("{e}"))?;

        let mut config = Self::default();
        let section = Section {
            ini: &ini,
            name: "window",
        };
        let w = &mut config.window;

        section.string("title", &mut w.title);
        section.number("width", &mut w.width);
        section.number("height", &mut w.height);
        section.boolean("fullscreen", &mut w.fullscreen);
        section.boolean("resizable", &mut w.resizable);
        section.boolean("active", &mut w.active);
        section.boolean("visible", &mut w.visible);
        section.boolean("decorations", &mut w.decorations);
        section.boolean("transparent", &mut w.transparent);
        section.boolean("blur", &mut w.blur);
        section.boolean("maximized", &mut w.maximized);
        section.number("min_width", &mut w.min_width);
        section.number("min_height", &mut w.min_height);
        section.number("max_width", &mut w.max_width);
        section.number("max_height", &mut w.max_height);

        section.boolean("macos_titlebar_transparent", &mut w.macos_titlebar_transparent);
        section.boolean("macos_titlebar_hidden", &mut w.macos_titlebar_hidden);
        section.boolean("macos_titlebar_buttons_hidden", &mut w.macos_titlebar_buttons_hidden);
        section.boolean("macos_title_hidden", &mut w.macos_title_hidden);
        section.boolean("macos_fullsize_content_view", &mut w.macos_fullsize_content_view);
        section.boolean("macos_has_shadow", &mut w.macos_has_shadow);

        Ok(config)
    }
}
