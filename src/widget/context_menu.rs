use std::slice;

pub use iced::widget::container::{Style, StyleFn};
use iced::{
    Event, Length, Point, Rectangle, Renderer, Size, Task, Vector,
    advanced::{
        self, Clipboard, Layout, Shell, Widget,
        layout::{self, Limits, Node},
        mouse::Cursor,
        overlay, renderer,
        widget::{self, Operation, operation, tree},
    },
    mouse,
    widget::{column, container},
};

use crate::theme::Theme;

use super::{Element, double_pass::double_pass};

pub fn context_menu<'a, T, Message>(
    base: impl Into<Element<'a, Message>>,
    entries: Vec<T>,
    entry: impl Fn(T, Length) -> Element<'a, Message> + 'a,
) -> ContextMenu<'a, T, Message> {
    ContextMenu {
        base: base.into(),
        entries,
        entry: Box::new(entry),
        menu: None,
    }
}

pub struct ContextMenu<'a, T, Message> {
    base: Element<'a, Message>,
    entries: Vec<T>,
    entry: Box<dyn Fn(T, Length) -> Element<'a, Message> + 'a>,
    menu: Option<Element<'a, Message>>,
}

#[derive(Debug)]
pub struct State {
    pub status: Status,
    menu_tree: widget::Tree,
}

impl State {
    pub fn new() -> Self {
        Self {
            status: Status::Closed,
            menu_tree: widget::Tree::empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Closed,
    Open(Point),
}

impl Status {
    pub fn open(self) -> Option<Point> {
        match self {
            Status::Closed => None,
            Status::Open(point) => Some(point),
        }
    }
}

impl<'a, T, Message> Widget<Message, Theme, Renderer> for ContextMenu<'a, T, Message>
where
    T: Copy + 'a,
    Message: 'a,
{
    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.base.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &Renderer, limits: &Limits) -> Node {
        self.base
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![widget::Tree::new(&self.base)]
    }

    fn diff(&mut self, tree: &mut tree::Tree) {
        tree.diff_children(slice::from_mut(&mut self.base));
    }

    fn operate(
        &mut self,
        tree: &mut advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<()>,
    ) {
        let state = tree.state.downcast_mut::<State>();

        operation.custom(None, layout.bounds(), state);

        self.base
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let is_mouse_event = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        );

        if is_mouse_event {
            let state = tree.state.downcast_mut::<State>();
            let prev_status = state.status;

            let Some(position) = cursor.position_over(layout.bounds()).map(|_| {
                let widget = layout.bounds();
                Point::new(widget.x + widget.width, widget.y + widget.height)
            }) else {
                return;
            };

            let next_status = match prev_status {
                Status::Closed => Status::Open(position),
                Status::Open(_) => Status::Closed,
            };

            if next_status != prev_status {
                state.status = next_status;
                if !matches!(next_status, Status::Closed) || !matches!(prev_status, Status::Closed)
                {
                    shell.request_redraw();
                }
            }
        }

        self.base.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        _state: &tree::Tree,
        layout: Layout<'_>,
        cursor: advanced::mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let base_state = tree.children.first_mut().unwrap();
        let base =
            self.base
                .as_widget_mut()
                .overlay(base_state, layout, renderer, viewport, translation);

        let state = tree.state.downcast_mut::<State>();

        let overlay = overlay(
            state,
            &mut self.menu,
            &self.entries,
            &self.entry,
            translation,
        );

        if base.is_none() && overlay.is_none() {
            None
        } else {
            Some(overlay::Group::with_children(base.into_iter().chain(overlay).collect()).overlay())
        }
    }
}

fn build_menu<'a, T, Message>(
    entries: &[T],
    entry: &(dyn Fn(T, Length) -> Element<'a, Message> + 'a),
) -> Element<'a, Message>
where
    T: Copy + 'a,
    Message: 'a,
{
    let build_menu = |length, view: &(dyn Fn(T, Length) -> Element<'a, Message> + 'a)| {
        container(column(
            entries.iter().copied().map(|entry| view(entry, length)),
        ))
        .padding(4)
        .style(|theme| <Theme as Catalog>::style(theme, &<Theme as Catalog>::default()))
    };

    double_pass(
        build_menu(Length::Shrink, entry),
        build_menu(Length::Fill, entry),
    )
    // build_menu(Length::Shrink, entry).into()
}

pub fn overlay<'a, 'b, T, Message>(
    state: &'b mut State,
    menu: &'b mut Option<Element<'a, Message>>,
    entries: &[T],
    entry: &(dyn Fn(T, Length) -> Element<'a, Message> + 'a),
    translation: Vector,
) -> Option<overlay::Element<'b, Message, Theme, Renderer>>
where
    T: Copy + 'a,
    Message: 'a,
{
    if entries.is_empty() {
        return None;
    }

    // Ensure overlay is created / diff'd
    match state.status {
        Status::Open(_) => match menu {
            Some(menu) => state.menu_tree.diff(menu),
            None => {
                let _menu = build_menu(entries, entry);
                state.menu_tree = widget::Tree::new(&_menu);
                *menu = Some(_menu);
            }
        },
        Status::Closed => {
            *menu = None;
        }
    }

    state
        .status
        .open()
        .zip(menu.as_mut())
        .map(|(position, menu)| {
            overlay::Element::new(Box::new(Overlay {
                menu,
                state,
                position: position + translation,
            }))
        })
}

pub fn close<Message: 'static + Send>(f: fn(bool) -> Message) -> Task<Message> {
    struct Close<T> {
        any_closed: bool,
        f: fn(bool) -> T,
    }

    impl<T> Operation<T> for Close<T> {
        fn container(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<T>),
        ) {
            operate_on_children(self);
        }

        fn custom(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn std::any::Any,
        ) {
            if let Some(state) = state.downcast_mut::<State>()
                && let Status::Open(_) = state.status
            {
                state.status = Status::Closed;
                self.any_closed = true;
            }
        }

        fn finish(&self) -> operation::Outcome<T> {
            operation::Outcome::Some((self.f)(self.any_closed))
        }
    }

    widget::operate(Close {
        any_closed: false,
        f,
    })
}

impl<'a, T, Message> From<ContextMenu<'a, T, Message>> for Element<'a, Message>
where
    T: Copy + 'a,
    Message: 'a,
{
    fn from(context_menu: ContextMenu<'a, T, Message>) -> Self {
        Element::new(context_menu)
    }
}

struct Overlay<'a, 'b, Message> {
    menu: &'b mut Element<'a, Message>,
    state: &'b mut State,
    position: Point,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Overlay<'_, '_, Message>
where
    Renderer: advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds)
            .width(Length::Fill)
            .height(Length::Fill);

        let node = self
            .menu
            .as_widget_mut()
            .layout(&mut self.state.menu_tree, renderer, &limits);

        // Small padding to ensure that we don't spawn context menu at the very edge of the viewport.
        let padding = 5.0;
        let viewport = Rectangle::new(
            Point::new(Point::ORIGIN.x + padding, Point::ORIGIN.y + padding),
            Size::new(bounds.width - 2.0 * padding, bounds.height - 2.0 * padding),
        );
        let mut bounds = Rectangle::new(self.position, node.size());

        if bounds.x < viewport.x {
            bounds.x = viewport.x;
        } else if viewport.x + viewport.width < bounds.x + bounds.width {
            bounds.x = viewport.x + viewport.width - bounds.width;
        }

        if bounds.y < viewport.y {
            bounds.y = viewport.y;
        } else if viewport.y + viewport.height < bounds.y + bounds.height {
            bounds.y = viewport.y + viewport.height - bounds.height;
        }

        node.move_to(bounds.position())
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.menu.as_widget().draw(
            &self.state.menu_tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.menu
            .as_widget_mut()
            .operate(&mut self.state.menu_tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = &event
            && cursor.position_over(layout.bounds()).is_none()
        {
            self.state.status = Status::Closed;
        }

        if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = &event
            && cursor.position_over(layout.bounds()).is_some()
        {
            self.state.status = Status::Closed;
        }

        self.menu.as_widget_mut().update(
            &mut self.state.menu_tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.menu.as_widget().mouse_interaction(
            &self.state.menu_tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
}

pub trait Catalog {
    type Class<'a>;

    fn default<'a>() -> Self::Class<'a>;

    fn style(&self, class: &Self::Class<'_>) -> container::Style;
}
