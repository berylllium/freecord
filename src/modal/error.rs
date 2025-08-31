use iced::{
    Length, alignment,
    widget::{button, column, container, text},
};

use crate::{theme, widget::Element};

use super::Message;

#[derive(Debug, Clone)]
pub struct Error {
    pub title: String,
    pub message: String,
}

impl Error {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        container(
            column![
                text("An error has occurred."),
                text(self.title.as_str()),
                text(self.message.as_str()),
                button(
                    container(text("Close"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill)
                )
                .style(|theme, status| theme::button::secondary(theme, status, false))
                .padding(5)
                .width(Length::Fixed(250.0))
                .on_press(Message::Close),
            ]
            .spacing(20)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Length::Shrink)
        .style(theme::container::error_tooltip)
        .padding(25)
        .into()
    }
}
