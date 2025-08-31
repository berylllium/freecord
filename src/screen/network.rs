use iced::{
    Length,
    widget::{column, container},
};

use crate::widget::Element;

pub struct Network {}

pub enum Message {}

pub enum Event {}

impl Network {
    pub fn new() -> Self {
        Self {}
    }

    pub fn view<'a>(&self) -> Element<'a, Message> {
        let content = column![];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .into()
    }
}
