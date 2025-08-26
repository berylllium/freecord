use iced::{
    Background, Border, Color, border,
    widget::container::{Catalog, Style, StyleFn, transparent},
};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn buffer(theme: &Theme, focus: bool) -> Style {
    let buffer = theme.styles.buffer;

    Style {
        background: Some(iced::Background::Color(buffer.background)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if focus {
                buffer.border_focus
            } else {
                buffer.border
            },
        },
        ..Default::default()
    }
}

pub fn buffer_title_bar(theme: &Theme) -> Style {
    Style {
        background: Some(iced::Background::Color(
            theme.styles.buffer.background_title_bar,
        )),
        text_color: Some(theme.styles.text.primary),
        border: Border {
            radius: border::top_left(4).top_right(4),
            width: 1.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn general(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(theme.styles.general.background)),
        text_color: Some(theme.styles.text.primary),
        ..Default::default()
    }
}

pub fn tooltip(theme: &Theme) -> Style {
    let general = theme.styles.general;

    Style {
        background: Some(Background::Color(general.background)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: general.border,
        },
        ..Default::default()
    }
}
