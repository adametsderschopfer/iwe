use super::*;
use std::rc::Rc;

struct Line(usize);

struct LineState {
    number: usize,
    identity: Rc<()>,
    layouts: usize,
}

impl Widget<(), Theme, Renderer> for Line {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<LineState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(LineState {
            number: self.0,
            identity: Rc::new(()),
            layouts: 0,
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(24.))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        tree.state.downcast_mut::<LineState>().layouts += 1;
        layout::Node::new(Size::new(100., 24.))
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }
}

#[tokio::test]
async fn stationary_rows_reuse_layout_until_content_or_bounds_change() {
    use iced_test::core::renderer::Headless;
    let renderer =
        iced_test::renderer::Renderer::new(iced::Font::DEFAULT, 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let make = |key| view(key, 0..45, (0..45).map(|index| Element::new(Line(index))));
    let mut widget = make(1);
    let mut tree = Tree::new(&widget);
    let mut limits = layout::Limits::new(Size::ZERO, Size::new(800., 600.));
    widget.as_widget_mut().layout(&mut tree, &renderer, &limits);
    for _ in 0..20 {
        widget = make(1);
        tree.diff(widget.as_widget());
        widget.as_widget_mut().layout(&mut tree, &renderer, &limits);
    }
    assert!(
        tree.children[0]
            .children
            .iter()
            .all(|row| row.state.downcast_ref::<LineState>().layouts == 1)
    );
    limits = layout::Limits::new(Size::ZERO, Size::new(900., 600.));
    widget.as_widget_mut().layout(&mut tree, &renderer, &limits);
    widget = make(2);
    tree.diff(widget.as_widget());
    widget.as_widget_mut().layout(&mut tree, &renderer, &limits);
    assert!(
        tree.children[0]
            .children
            .iter()
            .all(|row| row.state.downcast_ref::<LineState>().layouts == 3)
    );
}

#[test]
fn overlapping_rows_keep_their_state_when_scrolling_or_resizing() {
    let make = |range: Range<usize>| {
        view(
            0,
            range.clone(),
            range.map(|index| Element::new(Line(index))),
        )
    };
    let mut widget = make(4..10);
    let mut tree = Tree::new(&widget);
    for range in [5..11, 3..9, 2..12, 6..8, 20..26, 0..0, 0..4] {
        let previous: Vec<_> = tree.children[0]
            .children
            .iter()
            .map(|row| {
                let state = row.state.downcast_ref::<LineState>();
                (state.number, Rc::clone(&state.identity))
            })
            .collect();
        widget = make(range.clone());
        tree.diff(widget.as_widget());
        assert_eq!(tree.children[0].children.len(), range.len());
        for (index, row) in range.zip(&tree.children[0].children) {
            let state = row.state.downcast_ref::<LineState>();
            assert_eq!(state.number, index);
            if let Some((_, identity)) = previous.iter().find(|(number, _)| *number == index) {
                assert!(Rc::ptr_eq(identity, &state.identity));
            }
        }
    }
}
