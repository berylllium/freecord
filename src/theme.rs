pub mod button;
pub mod container;
pub mod context_menu;
pub mod pane_grid;
pub mod rule;
pub mod scrollable;
pub mod text;

use iced::Color;
use serde::{Deserialize, Serialize};

const DEFAULT_THEME_NAME: &str = "GruvBoxMedium";
const DEFAULT_THEME_CONTENT: &str = include_str!("../assets/theme/gruvboxmedium.toml");

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub styles: Styles,
}

impl Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: DEFAULT_THEME_NAME.to_string(),
            styles: toml::from_str(DEFAULT_THEME_CONTENT).expect("expected valid default theme"),
        }
    }
}

impl iced::theme::Base for Theme {
    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: self.styles.general.background,
            text_color: self.styles.text.primary,
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Styles {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub buffer: Buffer,
    #[serde(default)]
    pub text: Text,
    #[serde(default)]
    pub button: Button,
}

impl Styles {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    #[serde(with = "color_serde")]
    pub background: Color,
    #[serde(with = "color_serde")]
    pub background_success: Color,
    #[serde(with = "color_serde")]
    pub background_failure: Color,
    #[serde(with = "color_serde")]
    pub border: Color,
    #[serde(with = "color_serde")]
    pub scrollbar: Color,
    #[serde(with = "color_serde")]
    pub horizontal_rule: Color,
}

impl Default for General {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            background_success: Color::TRANSPARENT,
            background_failure: Color::TRANSPARENT,
            border: Color::TRANSPARENT,
            scrollbar: Color::TRANSPARENT,
            horizontal_rule: Color::TRANSPARENT,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Buffer {
    #[serde(with = "color_serde")]
    pub background: Color,
    #[serde(with = "color_serde")]
    pub background_title_bar: Color,
    #[serde(with = "color_serde")]
    pub border: Color,
    #[serde(with = "color_serde")]
    pub border_focus: Color,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            background_title_bar: Color::TRANSPARENT,
            border: Color::TRANSPARENT,
            border_focus: Color::TRANSPARENT,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Text {
    #[serde(with = "color_serde")]
    pub primary: Color,
    #[serde(with = "color_serde")]
    pub secondary: Color,
    #[serde(with = "color_serde")]
    pub tertiary: Color,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            primary: Color::TRANSPARENT,
            secondary: Color::TRANSPARENT,
            tertiary: Color::TRANSPARENT,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Button {
    pub primary: ButtonType,
    pub secondary: ButtonType,
    pub dangerous: ButtonType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct ButtonType {
    #[serde(with = "color_serde")]
    pub background: Color,
    #[serde(with = "color_serde")]
    pub background_hover: Color,
    #[serde(with = "color_serde")]
    pub background_selected: Color,
    #[serde(with = "color_serde")]
    pub background_selected_hover: Color,
}

impl Default for ButtonType {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            background_hover: Color::TRANSPARENT,
            background_selected: Color::TRANSPARENT,
            background_selected_hover: Color::TRANSPARENT,
        }
    }
}

mod color_serde {
    use iced::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)
            .map(|hex| super::hex_to_color(&hex))?
            .unwrap_or(Color::TRANSPARENT))
    }

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::color_to_hex(*color).serialize(serializer)
    }
}

pub fn hex_to_color(hex: &str) -> Option<Color> {
    if hex.len() == 7 || hex.len() == 9 {
        let hash = &hex[0..1];
        let r = u8::from_str_radix(&hex[1..3], 16);
        let g = u8::from_str_radix(&hex[3..5], 16);
        let b = u8::from_str_radix(&hex[5..7], 16);
        let a = (hex.len() == 9)
            .then(|| u8::from_str_radix(&hex[7..9], 16).ok())
            .flatten();

        return match (hash, r, g, b, a) {
            ("#", Ok(r), Ok(g), Ok(b), None) => Some(Color {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: 1.0,
            }),
            ("#", Ok(r), Ok(g), Ok(b), Some(a)) => Some(Color {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: a as f32 / 255.0,
            }),
            _ => None,
        };
    }

    None
}

pub fn color_to_hex(color: Color) -> String {
    use std::fmt::Write;

    let mut hex = String::with_capacity(9);

    let [r, g, b, a] = color.into_rgba8();

    let _ = write!(&mut hex, "#");
    let _ = write!(&mut hex, "{r:02X}");
    let _ = write!(&mut hex, "{g:02X}");
    let _ = write!(&mut hex, "{b:02X}");

    if a < u8::MAX {
        let _ = write!(&mut hex, "{a:02X}");
    }

    hex
}
