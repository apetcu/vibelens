use ratatui::style::Color;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Default,
    Dracula,
    Solarized,
    Nord,
    Monokai,
    Gruvbox,
}

impl Theme {
    pub fn all() -> &'static [Theme] {
        &[
            Theme::Default,
            Theme::Dracula,
            Theme::Solarized,
            Theme::Nord,
            Theme::Monokai,
            Theme::Gruvbox,
        ]
    }

    pub fn next(self) -> Theme {
        let all = Self::all();
        let idx = all.iter().position(|&t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn colors(self) -> ThemeColors {
        match self {
            Theme::Default => ThemeColors::default_theme(),
            Theme::Dracula => ThemeColors::dracula(),
            Theme::Solarized => ThemeColors::solarized(),
            Theme::Nord => ThemeColors::nord(),
            Theme::Monokai => ThemeColors::monokai(),
            Theme::Gruvbox => ThemeColors::gruvbox(),
        }
    }

    pub fn from_name(name: &str) -> Option<Theme> {
        match name.to_lowercase().as_str() {
            "default" => Some(Theme::Default),
            "dracula" => Some(Theme::Dracula),
            "solarized" => Some(Theme::Solarized),
            "nord" => Some(Theme::Nord),
            "monokai" => Some(Theme::Monokai),
            "gruvbox" => Some(Theme::Gruvbox),
            _ => None,
        }
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Theme::Default => write!(f, "Default"),
            Theme::Dracula => write!(f, "Dracula"),
            Theme::Solarized => write!(f, "Solarized"),
            Theme::Nord => write!(f, "Nord"),
            Theme::Monokai => write!(f, "Monokai"),
            Theme::Gruvbox => write!(f, "Gruvbox"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub title: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    // Model colors
    pub opus: Color,
    pub sonnet: Color,
    pub haiku: Color,
    // Token colors
    pub token_input: Color,
    pub token_output: Color,
    pub token_cache: Color,
    // Bar chart
    pub bar: Color,
    pub bar_alt: Color,
    // Source badges
    pub claude_badge: Color,
    pub cursor_badge: Color,
    // XML tag highlighting
    pub xml_tag: Color,
}

impl ThemeColors {
    fn default_theme() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            muted: Color::DarkGray,
            border: Color::DarkGray,
            accent: Color::Cyan,
            title: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            danger: Color::Red,
            highlight_bg: Color::Cyan,
            highlight_fg: Color::Black,
            opus: Color::Yellow,
            sonnet: Color::Blue,
            haiku: Color::Green,
            token_input: Color::Cyan,
            token_output: Color::Magenta,
            token_cache: Color::DarkGray,
            bar: Color::Cyan,
            bar_alt: Color::Blue,
            claude_badge: Color::Cyan,
            cursor_badge: Color::Yellow,
            xml_tag: Color::Rgb(180, 140, 220),
        }
    }

    fn dracula() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(98, 114, 164),
            border: Color::Rgb(68, 71, 90),
            accent: Color::Rgb(139, 233, 253),
            title: Color::Rgb(189, 147, 249),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            danger: Color::Rgb(255, 85, 85),
            highlight_bg: Color::Rgb(68, 71, 90),
            highlight_fg: Color::Rgb(248, 248, 242),
            opus: Color::Rgb(255, 184, 108),
            sonnet: Color::Rgb(139, 233, 253),
            haiku: Color::Rgb(80, 250, 123),
            token_input: Color::Rgb(139, 233, 253),
            token_output: Color::Rgb(255, 121, 198),
            token_cache: Color::Rgb(98, 114, 164),
            bar: Color::Rgb(189, 147, 249),
            bar_alt: Color::Rgb(139, 233, 253),
            claude_badge: Color::Rgb(139, 233, 253),
            cursor_badge: Color::Rgb(241, 250, 140),
            xml_tag: Color::Rgb(189, 147, 249),
        }
    }

    fn solarized() -> Self {
        Self {
            bg: Color::Rgb(0, 43, 54),
            fg: Color::Rgb(131, 148, 150),
            muted: Color::Rgb(88, 110, 117),
            border: Color::Rgb(7, 54, 66),
            accent: Color::Rgb(38, 139, 210),
            title: Color::Rgb(181, 137, 0),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(203, 75, 22),
            danger: Color::Rgb(220, 50, 47),
            highlight_bg: Color::Rgb(7, 54, 66),
            highlight_fg: Color::Rgb(238, 232, 213),
            opus: Color::Rgb(203, 75, 22),
            sonnet: Color::Rgb(38, 139, 210),
            haiku: Color::Rgb(133, 153, 0),
            token_input: Color::Rgb(38, 139, 210),
            token_output: Color::Rgb(211, 54, 130),
            token_cache: Color::Rgb(88, 110, 117),
            bar: Color::Rgb(42, 161, 152),
            bar_alt: Color::Rgb(38, 139, 210),
            claude_badge: Color::Rgb(38, 139, 210),
            cursor_badge: Color::Rgb(181, 137, 0),
            xml_tag: Color::Rgb(108, 113, 196),
        }
    }

    fn nord() -> Self {
        Self {
            bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            muted: Color::Rgb(76, 86, 106),
            border: Color::Rgb(59, 66, 82),
            accent: Color::Rgb(136, 192, 208),
            title: Color::Rgb(129, 161, 193),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            danger: Color::Rgb(191, 97, 106),
            highlight_bg: Color::Rgb(67, 76, 94),
            highlight_fg: Color::Rgb(236, 239, 244),
            opus: Color::Rgb(235, 203, 139),
            sonnet: Color::Rgb(129, 161, 193),
            haiku: Color::Rgb(163, 190, 140),
            token_input: Color::Rgb(136, 192, 208),
            token_output: Color::Rgb(180, 142, 173),
            token_cache: Color::Rgb(76, 86, 106),
            bar: Color::Rgb(136, 192, 208),
            bar_alt: Color::Rgb(129, 161, 193),
            claude_badge: Color::Rgb(136, 192, 208),
            cursor_badge: Color::Rgb(235, 203, 139),
            xml_tag: Color::Rgb(180, 142, 173),
        }
    }

    fn monokai() -> Self {
        Self {
            bg: Color::Rgb(39, 40, 34),
            fg: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(117, 113, 94),
            border: Color::Rgb(62, 61, 50),
            accent: Color::Rgb(102, 217, 239),
            title: Color::Rgb(249, 38, 114),
            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(230, 219, 116),
            danger: Color::Rgb(249, 38, 114),
            highlight_bg: Color::Rgb(62, 61, 50),
            highlight_fg: Color::Rgb(248, 248, 242),
            opus: Color::Rgb(253, 151, 31),
            sonnet: Color::Rgb(102, 217, 239),
            haiku: Color::Rgb(166, 226, 46),
            token_input: Color::Rgb(102, 217, 239),
            token_output: Color::Rgb(174, 129, 255),
            token_cache: Color::Rgb(117, 113, 94),
            bar: Color::Rgb(249, 38, 114),
            bar_alt: Color::Rgb(174, 129, 255),
            claude_badge: Color::Rgb(102, 217, 239),
            cursor_badge: Color::Rgb(230, 219, 116),
            xml_tag: Color::Rgb(174, 129, 255),
        }
    }

    fn gruvbox() -> Self {
        Self {
            bg: Color::Rgb(40, 40, 40),
            fg: Color::Rgb(235, 219, 178),
            muted: Color::Rgb(146, 131, 116),
            border: Color::Rgb(80, 73, 69),
            accent: Color::Rgb(131, 165, 152),
            title: Color::Rgb(250, 189, 47),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(254, 128, 25),
            danger: Color::Rgb(251, 73, 52),
            highlight_bg: Color::Rgb(80, 73, 69),
            highlight_fg: Color::Rgb(253, 244, 193),
            opus: Color::Rgb(254, 128, 25),
            sonnet: Color::Rgb(131, 165, 152),
            haiku: Color::Rgb(184, 187, 38),
            token_input: Color::Rgb(131, 165, 152),
            token_output: Color::Rgb(211, 134, 155),
            token_cache: Color::Rgb(146, 131, 116),
            bar: Color::Rgb(250, 189, 47),
            bar_alt: Color::Rgb(254, 128, 25),
            claude_badge: Color::Rgb(131, 165, 152),
            cursor_badge: Color::Rgb(250, 189, 47),
            xml_tag: Color::Rgb(211, 134, 155),
        }
    }

    pub fn model_color(&self, model: &str) -> Color {
        let m = model.to_lowercase();
        if m.contains("opus") {
            self.opus
        } else if m.contains("haiku") {
            self.haiku
        } else {
            self.sonnet
        }
    }
}

/// Load saved theme from config
pub fn load_saved_theme() -> Theme {
    let config_path = dirs::config_dir()
        .unwrap_or_default()
        .join("claude-tracker")
        .join("theme");
    if let Ok(name) = std::fs::read_to_string(&config_path) {
        Theme::from_name(name.trim()).unwrap_or(Theme::Default)
    } else {
        Theme::Default
    }
}

/// Save theme to config
pub fn save_theme(theme: Theme) {
    let config_dir = dirs::config_dir()
        .unwrap_or_default()
        .join("claude-tracker");
    let _ = std::fs::create_dir_all(&config_dir);
    let _ = std::fs::write(config_dir.join("theme"), theme.to_string());
}
