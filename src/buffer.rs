pub mod empty;
pub mod logs;

use iced::{Task, widget::pane_grid};
use logs::Logs;

use crate::{config::Config, logger, widget::Element};

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
    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        pane_logs: &[logger::Record],
        config: &'a Config,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty => empty::view(),
            Buffer::Logs(logs) => logs.view(pane_logs, config).map(Message::Logs),
        }
    }

    pub fn focus(&self) -> Task<Message> {
        match self {
            Self::Empty | Self::Logs(_) => Task::none(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BufferAction {
    Replace,
    NewPane,
    NewWindow,
}
