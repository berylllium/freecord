use iced::{
    Length,
    widget::{button, column, vertical_space},
};

use crate::{icon, theme, widget::Element};

pub struct Sidebar {}

#[derive(Debug, Clone)]
pub enum Message {
    OnPressAddContact,
}

pub enum Event {}

impl Sidebar {
    pub fn new() -> Self {
        Self {}
    }
    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let contacts = vertical_space().height(Length::Fill);

        let menu_button = button(icon::menu().style(theme::text::tertiary))
            .width(Length::Shrink)
            .padding(5)
            .on_press(Message::OnPressAddContact);

        column![contacts, menu_button].into()
    }
}
