pub mod pane;
pub mod sidebar;

use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};

use iced::{
    Length, Task, alignment,
    widget::{PaneGrid, container, pane_grid, row},
};
use pane::Pane;
use sidebar::Sidebar;

use crate::{
    buffer::Buffer,
    config::Config,
    logger,
    widget::Element,
    window::{self, Window},
};

const FOCUS_HISTORY_CAP: usize = 8;

pub struct Dashboard {
    panes: Panes,
    focus: Focus,
    focus_history: VecDeque<pane_grid::Pane>,
    sidebar: Sidebar,
    last_changed: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(sidebar::Message),
    Pane(window::Id, pane::Message),
}

pub enum Event {}

impl Dashboard {
    pub fn new(main_window: &Window) -> Self {
        let (mut state, pane) =
            pane_grid::State::new(Pane::new(Buffer::Logs(crate::buffer::logs::Logs::new())));

        state.split(pane_grid::Axis::Vertical, pane, Pane::new(Buffer::Empty));

        Self {
            panes: Panes {
                main_window: main_window.id,
                main: state,
                popout: HashMap::new(),
            },
            focus: Focus {
                window: main_window.id,
                pane,
            },
            focus_history: VecDeque::with_capacity(FOCUS_HISTORY_CAP),
            sidebar: Sidebar::new(),
            last_changed: None,
        }
    }
    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Pane(window, message) => match message {
                pane::Message::Clicked(pane) => return (self.focus_pane(window, pane), None),
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
                pane::Message::Close => {
                    return (self.close_pane(window, self.focus.pane), None);
                }
            },
            Message::Sidebar(_) => {}
        }

        (Task::none(), None)
    }

    pub fn view<'a>(
        &'a self,
        pane_logs: &'a [logger::Record],
        config: &'a Config,
    ) -> Element<'a, Message> {
        let sidebar = self.sidebar.view().map(Message::Sidebar);

        let pane_grid: Element<_> = PaneGrid::new(&self.panes.main, |id, pane, _maximized| {
            let is_focused = self.focus
                == Focus {
                    window: self.main_window(),
                    pane: id,
                };

            pane.view(
                id,
                self.panes.main.panes.len(),
                is_focused,
                pane_logs,
                config,
            )
        })
        .on_click(pane::Message::Clicked)
        .on_resize(6, pane::Message::Resized)
        .on_drag(pane::Message::Dragged)
        .spacing(4)
        .into();

        let pane_grid =
            container(pane_grid.map(move |message| Message::Pane(self.main_window(), message)))
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
        let Focus { window, pane } = self.focus;

        self.panes
            .iter()
            .find_map(|(w, p, state)| {
                (w == window && p == pane).then(|| {
                    state.buffer.focus().map(move |message| {
                        Message::Pane(window, pane::Message::Buffer(pane, message))
                    })
                })
            })
            .unwrap_or_else(Task::none)
    }

    fn focus_pane(&mut self, window: window::Id, pane: pane_grid::Pane) -> Task<Message> {
        if self.focus != (Focus { window, pane }) {
            self.focus = Focus { window, pane };

            self.last_changed = Some(Instant::now());

            if window == self.main_window() {
                self.focus_history.push_front(pane);

                self.focus_history.truncate(FOCUS_HISTORY_CAP);
            }
        }

        self.refocus_pane()
    }

    fn focus_first_pane(&mut self, window: window::Id) -> Task<Message> {
        let pane = self
            .panes
            .iter()
            .find_map(|(w, pane, _)| (w == window).then_some(pane));

        pane.map_or(Task::none(), |pane| self.focus_pane(window, pane))
    }

    fn focus_window_pane(&mut self, window: window::Id) -> Task<Message> {
        if self.focus.window == window {
            Task::none()
        } else if let Some(pane) = self
            .focus_history
            .front()
            .filter(|_| window == self.main_window())
        {
            self.focus_pane(window, *pane)
        } else {
            self.focus_first_pane(window)
        }
    }

    fn focus_window(&mut self, window: window::Id) -> Task<Message> {
        let task = self.focus_window_pane(window);

        window::gain_focus(window).chain(task)
    }

    fn close_pane(&mut self, window: window::Id, pane: pane_grid::Pane) -> Task<Message> {
        self.last_changed = Some(Instant::now());

        if window == self.main_window() {
            self.focus_history = self
                .focus_history
                .iter()
                .filter(|p| **p != pane)
                .copied()
                .collect();

            if let Some((_, sibling)) = self.panes.main.close(pane) {
                if self.is_focused(window, pane) {
                    return self.focus_pane(self.main_window(), sibling);
                }
            } else if let Some(pane) = self.panes.main.get_mut(pane) {
                pane.buffer = Buffer::Empty;
            }
        } else if self.panes.popout.remove(&window).is_some() {
            return window::close(window).chain(self.focus_window(self.main_window()));
        }

        Task::none()
    }

    fn main_window(&self) -> window::Id {
        self.panes.main_window
    }

    fn is_focused(&self, window: window::Id, pane: pane_grid::Pane) -> bool {
        self.focus == (Focus { window, pane })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Focus {
    pub window: window::Id,
    pub pane: pane_grid::Pane,
}

pub struct Panes {
    main_window: window::Id,
    main: pane_grid::State<Pane>,
    popout: HashMap<window::Id, pane_grid::State<Pane>>,
}

impl Panes {
    fn len(&self) -> usize {
        self.main.panes.len() + self.popout.len()
    }

    fn get(&self, window: window::Id, pane: pane_grid::Pane) -> Option<&Pane> {
        if self.main_window == window {
            self.main.get(pane)
        } else {
            self.popout.get(&window).and_then(|panes| panes.get(pane))
        }
    }

    fn iter(&self) -> impl Iterator<Item = (window::Id, pane_grid::Pane, &Pane)> {
        self.main
            .iter()
            .map(|(pane, state)| (self.main_window, *pane, state))
            .chain(self.popout.iter().flat_map(|(window_id, panes)| {
                panes.iter().map(|(pane, state)| (*window_id, *pane, state))
            }))
    }
}
