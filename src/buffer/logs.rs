use iced::{
    Length, Padding,
    widget::{self, Column, container, row, scrollable, space, text},
};

use crate::{config::Config, logger, widget::Element};

#[derive(Clone, Debug)]
pub struct Logs {
    scrollable: widget::Id,
}

#[derive(Clone, Debug)]
pub struct Message {}

#[derive(Clone, Debug)]
pub struct Event {}

impl Logs {
    pub fn new() -> Self {
        Self {
            scrollable: widget::Id::unique(),
        }
    }

    pub fn view<'a>(
        &self,
        pane_logs: &[logger::Record],
        config: &'a Config,
    ) -> Element<'a, Message> {
        let content = Column::from_iter(pane_logs.iter().map(|record| {
            let timestamp = text(record.timestamp.format("%H:%M").to_string());

            let log_level = {
                let level = record.level;
                text(format!("{level: >5}"))
            };

            let message = text(record.message.clone()).wrapping(text::Wrapping::Glyph);

            row![
                timestamp,
                space::horizontal().width(8),
                log_level,
                text(" "),
                message
            ]
            .into()
        }));

        let content = scrollable(content.padding(Padding::ZERO.right(8)))
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::default()
                    .anchor(scrollable::Anchor::End)
                    .width(config.pane.scrollbar.width)
                    .scroller_width(config.pane.scrollbar.scroller_width),
            ))
            .id(self.scrollable.clone());

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .clip(true)
            .into()
    }
}
