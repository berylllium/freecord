use iced::widget::column;

use crate::widget::Element;

#[derive(Clone, Debug)]
pub enum Message {}

#[derive(Clone, Debug)]
pub struct DirectChat {}

impl DirectChat {
    pub fn view<'a>(&self) -> Element<'a, Message> {
        column![].into()
    }
}
