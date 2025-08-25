use iced::widget::{text, text::LineHeight};

use crate::{font, widget::Text};

pub const ICON_SIZE: f32 = 12.0;

pub fn cancel<'a>() -> Text<'a> {
    to_text('\u{E813}')
}

pub fn popout<'a>() -> Text<'a> {
    to_text('\u{E814}')
}

pub fn logs<'a>() -> Text<'a> {
    to_text('\u{E809}')
}

pub fn config_file<'a>() -> Text<'a> {
    to_text('\u{F1C9}')
}

pub fn refresh<'a>() -> Text<'a> {
    to_text('\u{E815}')
}

pub fn menu<'a>() -> Text<'a> {
    to_text('\u{F0C9}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(LineHeight::Relative(1.0))
        .size(ICON_SIZE)
        .font(font::DEFAULT.icons)
}
