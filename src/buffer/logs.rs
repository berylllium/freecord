use iced::{
    Length,
    widget::{Column, container, horizontal_space, row, text},
};

use crate::{logger, widget::Element};

#[derive(Clone, Debug)]
pub struct Logs {}

#[derive(Clone, Debug)]
pub struct Message {}

#[derive(Clone, Debug)]
pub struct Event {}

impl Logs {
    pub fn new() -> Self {
        Self {}
    }

    pub fn view<'a>(&self, pane_logs: &[logger::Record]) -> Element<'a, Message> {
        let content = Column::from_iter(pane_logs.iter().map(|record| {
            let timestamp = text(record.timestamp.format("%H:%M").to_string());

            let log_level = {
                let level = record.level;
                text(format!("{level: >5}"))
            };

            let message = text(record.message.clone()).wrapping(text::Wrapping::Glyph);

            row![
                timestamp,
                horizontal_space().width(8),
                log_level,
                text(" "),
                message
            ]
            .into()
        }));

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .clip(true)
            .into()
    }
}
