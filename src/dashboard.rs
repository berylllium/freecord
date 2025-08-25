pub mod pane;
pub mod sidebar;

use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};

use iced::{
    Length, Task, Vector, alignment,
    widget::{PaneGrid, column, container, pane_grid, row},
};
use pane::Pane;
use sidebar::Sidebar;

use crate::{
    buffer::{Buffer, BufferAction},
    config::{self, Config},
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
    NewWindow(window::Id, Pane),
}

pub enum Event {
    ConfigReloaded(Result<Config, config::Error>),
}

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
    pub fn update(&mut self, message: Message, config: &Config) -> (Task<Message>, Option<Event>) {
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
                pane::Message::Buffer(_pane, _message) => {}
                pane::Message::Popout => return (self.popout_pane(config), None),
                pane::Message::Merge => return (self.merge_pane(config), None),
                pane::Message::Close => {
                    return (self.close_pane(window, self.focus.pane), None);
                }
            },
            Message::Sidebar(message) => {
                let (task, event) = self.sidebar.update(message);

                let Some(event) = event else {
                    return (task.map(Message::Sidebar), None);
                };

                let (event_task, event) = match event {
                    sidebar::Event::OpenConfigFile => {
                        let _ = open::that_detached(Config::path());
                        (Task::none(), None)
                    }
                    sidebar::Event::ConfigReloaded(config) => {
                        (Task::none(), Some(Event::ConfigReloaded(config)))
                    }
                };

                return (
                    Task::batch(vec![task.map(Message::Sidebar), event_task]),
                    event,
                );
            }
            Message::NewWindow(window, pane) => {
                let (state, pane) = pane_grid::State::new(pane);
                self.panes.popout.insert(window, state);

                return (self.focus_pane(window, pane), None);
            }
        }

        (Task::none(), None)
    }

    pub fn view_popout<'a>(
        &'a self,
        window: window::Id,
        pane_logs: &'a [logger::Record],
        config: &'a Config,
    ) -> Element<'a, Message> {
        if let Some(state) = self.panes.popout.get(&window) {
            let content = container(
                PaneGrid::new(state, |id, pane, _maximized| {
                    let is_focused = self.is_focused(window, id);

                    pane.view(id, 1, is_focused, pane_logs, config, true)
                })
                .on_click(pane::Message::Clicked),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

            Element::new(content).map(move |message| Message::Pane(window, message))
        } else {
            column![].into()
        }
    }

    pub fn view<'a>(
        &'a self,
        pane_logs: &'a [logger::Record],
        config: &'a Config,
        version: &'static str,
    ) -> Element<'a, Message> {
        let sidebar = self.sidebar.view(version).map(Message::Sidebar);

        let pane_grid: Element<_> = PaneGrid::new(&self.panes.main, |id, pane, _maximized| {
            let is_focused = self.is_focused(self.main_window(), id);

            pane.view(
                id,
                self.panes.main.panes.len(),
                is_focused,
                pane_logs,
                config,
                false,
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
        if !self.is_focused(window, pane) {
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

    fn new_pane(&mut self, axis: pane_grid::Axis) -> Task<Message> {
        if self.focus.window == self.main_window() {
            return self.split_pane(axis);
        } else {
            let pane = self.panes.main.iter().last().map(|(pane, _)| pane).copied();

            if let Some(pane) = pane {
                let result = self.panes.main.split(axis, pane, Pane::new(Buffer::Empty));
                self.last_changed = Some(Instant::now());

                if let Some((pane, _)) = result {
                    return self.focus_pane(self.main_window(), pane);
                }
            } else {
                let (state, pane) = pane_grid::State::new(Pane::new(Buffer::Empty));
                self.panes.main = state;
                self.last_changed = Some(Instant::now());
                return self.focus_pane(self.main_window(), pane);
            }
        }

        Task::none()
    }

    fn split_pane(&mut self, axis: pane_grid::Axis) -> Task<Message> {
        if self.focus.window == self.main_window() {
            let result = self
                .panes
                .main
                .split(axis, self.focus.pane, Pane::new(Buffer::Empty));
            self.last_changed = Some(Instant::now());

            if let Some((pane, _)) = result {
                return self.focus_pane(self.main_window(), pane);
            }
        }

        Task::none()
    }

    fn popout_pane(&mut self, config: &Config) -> Task<Message> {
        let Focus { pane, .. } = self.focus;

        self.focus_history = self
            .focus_history
            .clone()
            .into_iter()
            .filter(|p| *p != pane)
            .collect();

        if let Some((pane, _)) = self.panes.main.close(pane) {
            return self.open_buffer(pane.buffer, BufferAction::NewWindow, config);
        }

        Task::none()
    }

    fn merge_pane(&mut self, config: &Config) -> Task<Message> {
        let Focus { window, pane } = self.focus;

        if let Some(pane) = self
            .panes
            .popout
            .remove(&window)
            .and_then(|panes| panes.get(pane).cloned())
        {
            let task = self.open_buffer(pane.buffer, BufferAction::NewPane, config);

            return Task::batch(vec![
                window::close(window),
                window::gain_focus(self.main_window()).chain(task),
            ]);
        }

        Task::none()
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

    fn open_buffer(
        &mut self,
        buffer: Buffer,
        buffer_action: BufferAction,
        config: &Config,
    ) -> Task<Message> {
        let panes = self.panes.clone();

        self.last_changed = Some(Instant::now());

        match buffer_action {
            BufferAction::Replace => Task::none(),
            BufferAction::NewPane => {
                if self.panes.len() == 1 {
                    for (id, pane) in panes.main.iter() {
                        if matches!(pane.buffer, Buffer::Empty) {
                            self.panes.main.panes.entry(*id).and_modify(|p| {
                                *p = Pane::new(Buffer::from(buffer));
                            });
                            self.last_changed = Some(Instant::now());

                            return self.focus_pane(self.main_window(), *id);
                        }
                    }
                }

                let pane_to_split = {
                    if self.focus.window == self.main_window() {
                        self.focus.pane
                    } else if let Some(pane) = self.panes.main.panes.keys().last() {
                        *pane
                    } else {
                        log::error!("Didn't find any panes to split");
                        return Task::none();
                    }
                };

                let result = self.panes.main.split(
                    config.pane.split_axis.into(),
                    pane_to_split,
                    Pane::new(Buffer::from(buffer)),
                );

                if let Some((pane, _)) = result {
                    return self.focus_pane(self.main_window(), pane);
                }

                Task::none()
            }
            BufferAction::NewWindow => {
                window::get_position(self.main_window()).then(move |main_window_position| {
                    let (_, task) = window::open(window::Settings {
                        position: main_window_position
                            .map(|point| {
                                window::Position::Specific(point + Vector::new(20.0, 20.0))
                            })
                            .unwrap_or_default(),
                        exit_on_close_request: false,
                        ..window::settings()
                    });

                    task.map({
                        let pane = Pane::new(buffer.clone());
                        move |id| Message::NewWindow(id, pane.clone())
                    })
                })
            }
        }
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

#[derive(Clone)]
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
