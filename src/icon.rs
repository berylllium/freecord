use iced::widget::{text, text::LineHeight};

use crate::widget::Text;

pub const ICON_SIZE: f32 = 20.0;

pub fn menu<'a>() -> Text<'a> {
    to_text('\u{2630}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(LineHeight::Relative(1.0))
        .size(ICON_SIZE)
}
