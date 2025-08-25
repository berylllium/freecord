use std::{marker::PhantomData, slice};

use iced::{
    Length, Rectangle, Renderer, Size,
    advanced::{
        self, Layout, Widget,
        layout::{Limits, Node},
        mouse::Cursor,
        renderer::Style,
        widget::Tree,
    },
};

use crate::theme::Theme;

use super::Element;

pub fn double_pass<'a, Message: 'a>(
    first_pass: impl Into<Element<'a, Message>>,
    second_pass: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    DoublePass {
        first_pass: first_pass.into(),
        second_pass: second_pass.into(),
        state: PhantomData,
    }
    .into()
}

pub struct DoublePass<'a, Message, State = ()> {
    first_pass: Element<'a, Message>,
    second_pass: Element<'a, Message>,
    state: PhantomData<State>,
}

impl<'a, Message, State> Widget<Message, Theme, Renderer> for DoublePass<'a, Message, State>
where
    Message: 'a,
    State: Default + 'static,
{
    fn size(&self) -> Size<Length> {
        self.second_pass.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.second_pass.as_widget().size_hint()
    }

    fn tag(&self) -> advanced::widget::tree::Tag {
        struct Marker<State>(State);
        advanced::widget::tree::Tag::of::<Marker<State>>()
    }

    fn state(&self) -> advanced::widget::tree::State {
        advanced::widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.second_pass)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(slice::from_mut(&mut self.second_pass));
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let tree = &mut tree.children[0];

        let layout = {
            let mut tree = Tree::new(&self.first_pass);

            self.first_pass
                .as_widget_mut()
                .layout(&mut tree, renderer, limits)
        };

        let new_limits = Limits::new(
            Size::ZERO,
            layout.size().expand(Size::new(horizontal_expansion(), 1.0)),
        );

        self.second_pass
            .as_widget_mut()
            .layout(tree, renderer, &new_limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let tree = &tree.children[0];
        self.second_pass
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let tree = &mut tree.children[0];
        self.second_pass.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: advanced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        let tree = &tree.children[0];
        self.second_pass
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation,
    ) {
        let tree = &mut tree.children[0];
        self.second_pass
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let tree = &mut tree.children[0];
        self.second_pass
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

pub fn horizontal_expansion() -> f32 {
    1.0
}

impl<'a, Message: 'a> From<DoublePass<'a, Message>> for Element<'a, Message> {
    fn from(value: DoublePass<'a, Message>) -> Self {
        Element::new(value)
    }
}
