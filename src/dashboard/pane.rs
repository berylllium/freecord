use iced::{
    alignment,
    widget::{button, center, container, pane_grid, row, text},
};

use crate::{
    buffer::{self, Buffer},
    config::Config,
    icon, logger, theme,
    widget::{
        self,
        tooltip::{self, tooltip},
    },
};

#[derive(Debug, Clone)]
pub enum Message {
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Buffer(pane_grid::Pane, buffer::Message),
    Close,
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
        panes: usize,
        is_focused: bool,
        pane_logs: &[logger::Record],
        config: &'a Config,
    ) -> widget::Content<'a, Message> {
        let title_bar = self.title_bar.view(&self.buffer, panes, true);

        let content = self
            .buffer
            .view(id, pane_logs, config)
            .map(move |m| Message::Buffer(id, m));

        widget::Content::new(content)
            .style(move |theme| theme::container::buffer(theme, is_focused))
            .title_bar(title_bar)
    }
}

#[derive(Clone, Debug)]
struct TitleBar {}

impl TitleBar {
    fn view<'a>(
        &self,
        buffer: &Buffer,
        panes: usize,
        show_tooltips: bool,
    ) -> widget::TitleBar<'a, Message> {
        let title_text = match buffer {
            Buffer::Empty => "Empty buffer",
            Buffer::Logs(_) => "Logs",
        };

        let title = container(text(title_text))
            .height(22)
            .padding([0, 10])
            .align_y(alignment::Vertical::Center);

        let controls = row![if !(panes == 1 && matches!(buffer, Buffer::Empty)) {
            let close_button = button(center(icon::cancel()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::Close)
                .style(|theme, status| theme::button::secondary(theme, status, false));

            let close_button_with_tooltip = tooltip(
                close_button,
                show_tooltips.then_some("Close"),
                tooltip::Position::Bottom,
            );

            Some(close_button_with_tooltip)
        } else {
            None
        }]
        .spacing(2);

        pane_grid::TitleBar::new(title)
            .controls(pane_grid::Controls::new(controls))
            .padding(6)
            .style(theme::container::buffer_title_bar)
    }
}
