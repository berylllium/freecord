pub mod error;

pub use error::Error;
use iced::{Task, widget::column};

use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Modal {
    Error(Vec<Error>),
}

#[derive(Debug, Clone)]
pub enum Message {
    Close,
}

#[derive(Debug, Clone)]
pub enum Event {
    Close,
}

impl Modal {
    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Close => match self {
                Modal::Error(errors) => {
                    if errors.len() > 1 {
                        errors.pop();
                    } else {
                        return (Task::none(), Some(Event::Close));
                    }
                }
            },
        }

        (Task::none(), None)
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        match self {
            Modal::Error(errors) => {
                if let Some(last) = errors.last() {
                    last.view()
                } else {
                    column![].into()
                }
            }
        }
    }
}
