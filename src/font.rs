use std::borrow::Cow;

use iced::font;

pub static DEFAULT: Font = Font {
    mono: iced::Font {
        weight: font::Weight::Bold,
        style: font::Style::Normal,
        ..iced::Font::with_name("Iosevka Term")
    },
    bold: iced::Font {
        weight: font::Weight::Black,
        style: font::Style::Normal,
        ..iced::Font::with_name("Iosevka Term")
    },
    italic: iced::Font {
        weight: font::Weight::Bold,
        style: font::Style::Italic,
        ..iced::Font::with_name("Iosevka Term")
    },
    bold_italic: iced::Font {
        weight: font::Weight::Black,
        style: font::Style::Italic,
        ..iced::Font::with_name("Iosevka Term")
    },
    icons: iced::Font::with_name("freecord-icons"),
};

#[derive(Clone, Debug)]
pub struct Font {
    pub mono: iced::Font,
    pub bold: iced::Font,
    pub italic: iced::Font,
    pub bold_italic: iced::Font,
    pub icons: iced::Font,
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![
        include_bytes!("../assets/font/IosevkaTerm-Regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/font/IosevkaTerm-Bold.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/font/IosevkaTerm-Italic.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/font/IosevkaTerm-BoldItalic.ttf")
            .as_slice()
            .into(),
        include_bytes!("../assets/font/freecord-icons.ttf")
            .as_slice()
            .into(),
    ]
}
