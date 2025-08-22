pub mod pane;
pub mod sidebar;

use std::time::Instant;

use iced::{
    Length, Task, alignment,
    widget::{PaneGrid, container, pane_grid, row},
};
use pane::Pane;
use sidebar::Sidebar;

use crate::{buffer::Buffer, logger, widget::Element};

pub struct Dashboard {
    panes: Panes,
    focus: Focus,
    sidebar: Sidebar,
    last_changed: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(sidebar::Message),
    Pane(pane::Message),
}

pub enum Event {}

impl Dashboard {
    pub fn new() -> Self {
        let (mut state, pane) =
            pane_grid::State::new(Pane::new(Buffer::Logs(crate::buffer::logs::Logs::new())));

        state.split(pane_grid::Axis::Vertical, pane, Pane::new(Buffer::Empty));

        Self {
            panes: Panes::new(state),
            focus: Focus { pane },
            sidebar: Sidebar::new(),
            last_changed: None,
        }
    }
    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Pane(message) => match message {
                pane::Message::Clicked(pane) => return (self.focus_pane(pane), None),
                pane::Message::Dragged(drag_event) => match drag_event {
                    pane_grid::DragEvent::Dropped { pane, target } => {
                        self.panes.main.drop(pane, target);
                        self.last_changed = Some(Instant::now());
                    }
                    _ => {}
                },
                pane::Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                    self.panes.main.resize(split, ratio);
                    self.last_changed = Some(Instant::now());
                }

                pane::Message::Buffer(pane, message) => {}
            },
            Message::Sidebar(_) => {}
        }

        (Task::none(), None)
    }

    pub fn view<'a>(&'a self, pane_logs: &'a [logger::Record]) -> Element<'a, Message> {
        let sidebar = self.sidebar.view().map(Message::Sidebar);

        let pane_grid: Element<_> = PaneGrid::new(&self.panes.main, |id, pane, _maximized| {
            let is_focused = self.focus.pane == id;

            pane.view(id, is_focused, pane_logs)
        })
        .on_click(pane::Message::Clicked)
        .on_resize(6, pane::Message::Resized)
        .on_drag(pane::Message::Dragged)
        .spacing(4)
        .into();

        let pane_grid = container(pane_grid.map(Message::Pane))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

        let content = row![sidebar, pane_grid]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn refocus_pane(&mut self) -> Task<Message> {
        let focus_pane = self.focus.pane;

        self.panes
            .iter()
            .find_map(|(pane, state)| {
                (focus_pane == pane).then(|| {
                    state
                        .buffer
                        .focus()
                        .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                })
            })
            .unwrap_or_else(Task::none)
    }

    fn focus_pane(&mut self, pane: pane_grid::Pane) -> Task<Message> {
        if self.focus.pane != pane {
            self.focus.pane = pane;

            self.last_changed = Some(Instant::now());
        }

        self.refocus_pane()
    }
}

pub struct Focus {
    pub pane: pane_grid::Pane,
}

pub struct Panes {
    main: pane_grid::State<Pane>,
}

impl Panes {
    fn new(state: pane_grid::State<Pane>) -> Self {
        Self { main: state }
    }

    fn iter(&self) -> impl Iterator<Item = (pane_grid::Pane, &Pane)> {
        self.main.iter().map(|(pane, state)| (*pane, state))
    }
}
