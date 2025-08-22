use iced::{
    alignment,
    widget::{container, pane_grid, text},
};

use crate::{
    buffer::{self, Buffer},
    logger, theme, widget,
};

#[derive(Debug, Clone)]
pub enum Message {
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Buffer(pane_grid::Pane, buffer::Message),
}

#[derive(Clone, Debug)]
pub struct Pane {
    pub buffer: Buffer,
    title_bar: TitleBar,
}

impl Pane {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            title_bar: TitleBar {},
        }
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        is_focused: bool,
        pane_logs: &[logger::Record],
    ) -> widget::Content<'a, Message> {
        let title_bar = self.title_bar.view(&self.buffer);

        let content = self
            .buffer
            .view(id, pane_logs)
            .map(move |m| Message::Buffer(id, m));

        widget::Content::new(content)
            .style(move |theme| theme::container::buffer(theme, is_focused))
            .title_bar(title_bar)
    }
}

#[derive(Clone, Debug)]
struct TitleBar {}

impl TitleBar {
    fn view<'a>(&self, buffer: &Buffer) -> widget::TitleBar<'a, Message> {
        let title_text = match buffer {
            Buffer::Empty => "Empty buffer",
            Buffer::Logs(_) => "Logs",
        };

        let title = container(text(title_text))
            .height(22)
            .padding([0, 10])
            .align_y(alignment::Vertical::Center);

        pane_grid::TitleBar::new(title)
            .padding(6)
            .style(theme::container::buffer_title_bar)
    }
}
