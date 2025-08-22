pub mod empty;
pub mod logs;

use iced::{Task, widget::pane_grid};
use logs::Logs;

use crate::{logger, widget::Element};

#[derive(Clone, Debug)]
pub enum Buffer {
    Empty,
    Logs(Logs),
}

#[derive(Clone, Debug)]
pub enum Message {
    Logs(logs::Message),
}

pub enum Event {}

impl Buffer {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        pane_logs: &[logger::Record],
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty => empty::view(),
            Buffer::Logs(logs) => logs.view(pane_logs).map(Message::Logs),
        }
    }

    pub fn focus(&self) -> Task<Message> {
        match self {
            Self::Empty | Self::Logs(_) => Task::none(),
        }
    }
}
