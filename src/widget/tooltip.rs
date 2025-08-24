use iced::widget;
pub use iced::widget::tooltip::Position;

use crate::theme;

use super::Element;

pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip: Option<&'a str>,
    position: Position,
) -> Element<'a, Message> {
    match tooltip {
        Some(tooltip) => widget::tooltip(
            content,
            widget::container(widget::text(tooltip).style(theme::text::secondary))
                .style(theme::container::tooltip)
                .padding(8),
            position,
        )
        .into(),
        None => content.into(),
    }
}
