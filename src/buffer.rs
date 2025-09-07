pub mod direct_chat;
pub mod empty;
pub mod logs;

use direct_chat::DirectChat;
use iced::{Task, widget::pane_grid};
use logs::Logs;

use crate::{config::Config, logger, widget::Element};

#[derive(Clone, Debug)]
pub enum Buffer {
    Empty,
    Logs(Logs),
    DirectChat(DirectChat),
}

#[derive(Clone, Debug)]
pub enum Message {
    DirectChat(direct_chat::Message),
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
            Buffer::DirectChat(direct_chat) => direct_chat.view().map(Message::DirectChat),
            Buffer::Logs(logs) => logs.view(pane_logs, config).map(Message::Logs),
        }
    }

    pub fn focus(&self) -> Task<Message> {
        match self {
            Self::Empty | Self::Logs(_) | Self::DirectChat(_) => Task::none(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BufferAction {
    Replace,
    NewPane,
    NewWindow,
}
