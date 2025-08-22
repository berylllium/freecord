use iced::{
    Length, alignment,
    widget::{container, text},
};

use crate::widget::Element;

pub fn view<'a, Message: 'a>() -> Element<'a, Message> {
    let content = text("empty pane");

    container(content)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
