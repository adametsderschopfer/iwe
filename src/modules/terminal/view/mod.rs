//! Render the terminal screen and capture input only while its canvas has focus.

mod input;

use super::{Event, State as TerminalState, Status, style};
use crate::{
    i18n::{FluentArgs, I18n, Key},
    ui::{
        icons::{Icon, sized_icon},
        theme, tooltip,
    },
};
use iced::{
    Alignment, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme, font, keyboard,
    mouse,
    widget::{
        button,
        canvas::{self, Action, Frame, Geometry},
        column, container, row, scrollable, space, text,
    },
    window,
};

pub(crate) fn view<'a>(
    terminal: &'a TerminalState,
    i18n: &'a I18n,
    code_size: u16,
    interactive: bool,
) -> Element<'a, Event> {
    let header = row![
        text(i18n.text(Key::Terminal))
            .size(13)
            .font(theme::semibold()),
        space().width(Length::Fill),
        icon_control(
            i18n,
            Icon::Close,
            Key::TerminalHide,
            "terminal-hide".into(),
            Event::Hide
        ),
    ]
    .align_y(Alignment::Center);
    let mut tabs = row![].spacing(style::TAB_GAP).align_y(Alignment::Center);
    for tab in &terminal.tabs {
        let request = tab.request;
        let mut args = FluentArgs::new();
        args.set("number", tab.number);
        tabs = tabs.push(
            container(
                button(
                    row![
                        text(i18n.format(Key::TerminalTab, &args))
                            .size(12)
                            .wrapping(text::Wrapping::None),
                        icon_control(
                            i18n,
                            Icon::Close,
                            Key::TerminalCloseTab,
                            format!("terminal-close-tab-{request}"),
                            Event::Close(request),
                        ),
                    ]
                    .spacing(style::TAB_CONTENT_GAP)
                    .align_y(Alignment::Center),
                )
                .padding(iced::Padding {
                    left: 10.,
                    right: 4.,
                    top: 2.,
                    bottom: 2.,
                })
                .style(theme::action(terminal.active == Some(request)))
                .on_press(Event::Select(request)),
            )
            .id(format!("terminal-tab-{request}")),
        );
    }
    tabs = tabs.push(icon_control(
        i18n,
        Icon::Plus,
        Key::TerminalNewTab,
        "terminal-add-tab".into(),
        Event::Start,
    ));
    let strip = scrollable(tabs)
        .id("terminal-tabs")
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new(),
        ))
        .style(theme::scroll)
        .width(Length::Fill);
    let mut content = column![
        container(header)
            .padding([4, 10])
            .height(style::HEADER_HEIGHT)
            .width(Length::Fill),
        container(space().height(1)).style(theme::separator),
        container(strip)
            .id("terminal-tab-strip")
            .padding([4, 10])
            .height(style::TAB_BAR_HEIGHT)
            .width(Length::Fill),
        container(space().height(1)).style(theme::separator),
    ]
    .height(Length::Fill);
    let Some(tab) = terminal.active_tab() else {
        return content.into();
    };
    let status = match tab.status {
        Status::Starting => Some(Key::TerminalStarting),
        Status::Failed => Some(Key::TerminalFailed),
        Status::Exited => Some(Key::TerminalExited),
        Status::Running => None,
    };
    if let Some(key) = status {
        content = content.push(
            container(text(i18n.text(key)).size(12).style(theme::MUTED.text())).padding([6, 12]),
        );
    }
    content
        .push(
            container(
                canvas::Canvas::new(Screen {
                    screen: tab.parser.screen(),
                    request: tab.request,
                    size: f32::from(code_size),
                    interactive,
                    running: tab.running(),
                })
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .id("terminal-screen")
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .into()
}

fn icon_control<'a>(
    i18n: &I18n,
    glyph: Icon,
    hint: Key,
    id: String,
    event: Event,
) -> Element<'a, Event> {
    container(tooltip::view(
        button(sized_icon(glyph, theme::MUTED, 14.))
            .padding(4)
            .style(theme::action(false))
            .on_press(event),
        text(i18n.text(hint)).size(12),
        tooltip::Position::Bottom,
    ))
    .id(id)
    .into()
}

struct Screen<'a> {
    screen: &'a vt100::Screen,
    request: u64,
    size: f32,
    interactive: bool,
    running: bool,
}

#[derive(Default)]
pub(crate) struct State {
    request: Option<u64>,
    focused: bool,
    dimensions: (u16, u16),
    anchor: Option<(u16, u16)>,
    selection: Option<(u16, u16)>,
    dragging: bool,
    scrollback: usize,
}

impl Screen<'_> {
    fn cell_size(&self) -> Size {
        Size::new(
            self.size * style::CELL_WIDTH,
            self.size * style::LINE_HEIGHT,
        )
    }

    fn dimensions(&self, bounds: Rectangle) -> (u16, u16) {
        let cell = self.cell_size();
        (
            ((bounds.height - 2. * style::PADDING) / cell.height)
                .floor()
                .clamp(1., 200.) as u16,
            ((bounds.width - 2. * style::PADDING) / cell.width)
                .floor()
                .clamp(1., 400.) as u16,
        )
    }

    fn position(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<(u16, u16)> {
        let position = cursor.position_in(bounds)?;
        let cell = self.cell_size();
        let (rows, cols) = self.screen.size();
        Some((((position.y - style::PADDING) / cell.height).max(0.) as u16).min(rows - 1)).map(
            |row| {
                (
                    row,
                    (((position.x - style::PADDING) / cell.width).max(0.) as u16).min(cols - 1),
                )
            },
        )
    }

    fn selected(&self, state: &State) -> String {
        let (Some(anchor), Some(end)) = (state.anchor, state.selection) else {
            return self.screen.contents();
        };
        let (start, end) = (anchor.min(end), anchor.max(end));
        let mut lines = Vec::new();
        let (_, cols) = self.screen.size();
        for row in start.0..=end.0 {
            let first = if row == start.0 { start.1 } else { 0 };
            let last = if row == end.0 { end.1 } else { cols - 1 };
            let mut line = String::new();
            for col in first..=last {
                if let Some(cell) = self.screen.cell(row, col) {
                    if cell.is_wide_continuation() {
                        continue;
                    }
                    let value = cell.contents();
                    line.push_str(if value.is_empty() { " " } else { value });
                }
            }
            lines.push(line.trim_end().to_owned());
        }
        lines.join("\n")
    }
}

impl canvas::Program<Event> for Screen<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Event>> {
        if state.request != Some(self.request) {
            *state = State {
                request: Some(self.request),
                ..State::default()
            };
        }
        if state.scrollback != self.screen.scrollback() {
            state.anchor = None;
            state.selection = None;
            state.scrollback = self.screen.scrollback();
        }
        match event {
            canvas::Event::Window(window::Event::RedrawRequested(_)) => {
                let dimensions = self.dimensions(bounds);
                if state.dimensions != dimensions {
                    state.dimensions = dimensions;
                    return Some(Action::publish(Event::Resize(
                        self.request,
                        dimensions.0,
                        dimensions.1,
                    )));
                }
            }
            canvas::Event::Window(window::Event::Unfocused) => state.focused = false,
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.focused = self.interactive && cursor.is_over(bounds);
                state.anchor = self.position(bounds, cursor).filter(|_| state.focused);
                state.selection = None;
                state.dragging = state.focused;
                if state.focused {
                    return Some(Action::publish(Event::Focus(self.request)).and_capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                state.selection = self.position(bounds, cursor);
                return Some(Action::request_redraw().and_capture());
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta })
                if self.interactive && cursor.is_over(bounds) =>
            {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * 3.,
                    mouse::ScrollDelta::Pixels { y, .. } => y / self.cell_size().height,
                };
                return Some(
                    Action::publish(Event::Scroll(self.request, lines.round() as i32))
                        .and_capture(),
                );
            }
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                ..
            }) if state.focused && self.interactive => {
                // Deployment shortcuts belong to the source area, never to a focused terminal.
                if *modifiers == keyboard::Modifiers::COMMAND | keyboard::Modifiers::ALT
                    && matches!(
                        physical_key,
                        keyboard::key::Physical::Code(
                            keyboard::key::Code::KeyC | keyboard::key::Code::KeyV
                        )
                    )
                {
                    return Some(Action::capture());
                }
                let clipboard = if cfg!(target_os = "macos") {
                    modifiers.logo() && !modifiers.control() && !modifiers.alt()
                } else {
                    modifiers.control() && modifiers.shift() && !modifiers.alt()
                };
                if clipboard
                    && (matches!(key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c"))
                        || *physical_key
                            == keyboard::key::Physical::Code(keyboard::key::Code::KeyC))
                {
                    return Some(Action::publish(Event::Copy(self.selected(state))).and_capture());
                }
                if clipboard
                    && (matches!(key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("v"))
                        || *physical_key
                            == keyboard::key::Physical::Code(keyboard::key::Code::KeyV))
                    && self.running
                {
                    return Some(Action::publish(Event::Paste(self.request)).and_capture());
                }
                if self.running
                    && let Some(bytes) = input::encode(
                        key,
                        text.as_deref(),
                        *modifiers,
                        self.screen.application_cursor(),
                    )
                    .or_else(|| {
                        modifiers
                            .control()
                            .then(|| input::physical_control(*physical_key))
                            .flatten()
                    })
                {
                    state.anchor = None;
                    state.selection = None;
                    return Some(Action::publish(Event::Input(self.request, bytes)).and_capture());
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let cell_size = self.cell_size();
        let default_bg = style::background(theme);
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), default_bg);
        let (rows, cols) = self.screen.size();
        let dimensions = self.dimensions(bounds);
        let selection = state
            .anchor
            .zip(state.selection)
            .map(|(a, b)| (a.min(b), a.max(b)));
        for row in 0..rows.min(dimensions.0) {
            for col in 0..cols.min(dimensions.1) {
                let Some(cell) = self.screen.cell(row, col) else {
                    continue;
                };
                let position = Point::new(
                    style::PADDING + f32::from(col) * cell_size.width,
                    style::PADDING + f32::from(row) * cell_size.height,
                );
                let mut fg = style::color(cell.fgcolor(), theme::TEXT.resolve(theme), theme);
                let mut bg = style::color(cell.bgcolor(), default_bg, theme);
                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if selection.is_some_and(|(start, end)| (row, col) >= start && (row, col) <= end) {
                    bg = theme::CODE_SELECTION.resolve(theme);
                }
                if bg != default_bg {
                    frame.fill_rectangle(position, cell_size, bg);
                }
                if cell.is_wide_continuation() || cell.contents().is_empty() {
                    continue;
                }
                let font = Font {
                    weight: if cell.bold() {
                        font::Weight::Bold
                    } else {
                        font::Weight::Normal
                    },
                    style: if cell.italic() {
                        font::Style::Italic
                    } else {
                        font::Style::Normal
                    },
                    ..Font::MONOSPACE
                };
                frame.fill_text(canvas::Text {
                    content: cell.contents().to_owned(),
                    position,
                    color: fg,
                    font,
                    size: self.size.into(),
                    line_height: iced::widget::text::LineHeight::Absolute(cell_size.height.into()),
                    shaping: iced::widget::text::Shaping::Advanced,
                    ..canvas::Text::default()
                });
                if cell.underline() {
                    frame.fill_rectangle(
                        Point::new(position.x, position.y + cell_size.height - 2.),
                        Size::new(cell_size.width, 1.),
                        fg,
                    );
                }
            }
        }
        if state.focused
            && self.interactive
            && self.running
            && !self.screen.hide_cursor()
            && self.screen.scrollback() == 0
        {
            let (row, col) = self.screen.cursor_position();
            frame.fill_rectangle(
                Point::new(
                    style::PADDING + f32::from(col) * cell_size.width,
                    style::PADDING + f32::from(row) * cell_size.height,
                ),
                Size::new(2., cell_size.height),
                theme::TEXT.resolve(theme),
            );
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.interactive && cursor.is_over(bounds) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests;
