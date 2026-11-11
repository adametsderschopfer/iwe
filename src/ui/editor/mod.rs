//! Render editable source buffers and unsaved-change decisions.

mod gutter;
mod syntax;

const TEXT_PADDING: f32 = 12.;
const LINE_HEIGHT: f32 = 1.65;

use crate::{
    app::{
        App, Message,
        editor::{Buffer, Event},
    },
    domain::shortcuts::Command,
    i18n::Key,
    ui::theme,
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, space, text, text_editor},
};

pub(crate) fn view<'a>(app: &'a App, buffer: &'a Buffer) -> Element<'a, Message> {
    let id = app.preview().id;
    let bindings = &app.workspace.preferences.shortcuts;
    let blocked = app.editor.pending.is_some()
        || app.auth_dialog.is_some()
        || app.line_navigation.is_some()
        || app.space_draft.is_some()
        || app.project_add.is_some()
        || app.modules.git.git_actions.has_dialog()
        || app.modules.deployment.has_dialog();
    let extension = app
        .preview()
        .path
        .as_deref()
        .map(crate::ui::code::syntax_for)
        .unwrap_or_default();
    let editor = text_editor(&buffer.content)
        .id(format!("text-editor-{id}"))
        .on_action(move |action| Message::Editor(Event::Action(id, action)))
        .font(theme::code_font())
        .size(app.workspace.preferences.code_size as f32)
        .line_height(iced::widget::text::LineHeight::Relative(LINE_HEIGHT))
        .height(Length::Fill)
        .padding(TEXT_PADDING)
        .wrapping(iced::widget::text::Wrapping::None)
        .highlight_with::<syntax::Highlighter>(extension.to_owned(), |tone, theme| {
            iced::advanced::text::highlighter::Format {
                color: Some(tone.resolve(theme)),
                font: None,
            }
        })
        .key_binding(move |key| {
            use text_editor::{Binding, Status};
            if blocked || !matches!(key.status, Status::Focused { .. }) {
                return None;
            }
            if key.modifiers.command()
                && !key.modifiers.alt()
                && key.key.to_latin(key.physical_key) == Some('z')
            {
                return Some(Binding::Custom(Message::Editor(Event::Undo(
                    id,
                    key.modifiers.shift(),
                ))));
            }
            if key.key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) {
                return Some(Binding::Sequence(vec![Binding::Insert(' '); 4]));
            }
            for command in Command::ALL {
                if crate::ui::shortcuts::matches(
                    &bindings.binding(command),
                    &key.key,
                    key.physical_key,
                    key.modifiers,
                ) {
                    return match command {
                        Command::Copy => Some(Binding::Copy),
                        Command::SelectAll => Some(Binding::SelectAll),
                        _ => crate::app::shortcuts::command_message(command).map(Binding::Custom),
                    };
                }
            }
            Binding::from_key_press(key)
        })
        .style(|style_theme, _| text_editor::Style {
            background: theme::code_surface(style_theme)
                .background
                .unwrap_or(iced::Color::TRANSPARENT.into()),
            border: iced::Border::default(),
            placeholder: theme::MUTED.resolve(style_theme),
            value: theme::syntax::TEXT.resolve(style_theme),
            selection: theme::CODE_SELECTION.resolve(style_theme),
        });
    let cursor = buffer.content.cursor().position;
    let mut body = row![];
    if app.workspace.preferences.line_numbers {
        let size = app.workspace.preferences.code_size as f32;
        let digits = buffer.content.line_count().max(1).ilog10() + 1;
        body = body.push(
            container(
                iced::widget::canvas(gutter::Gutter {
                    content: &buffer.content,
                    size,
                })
                .width(size * 0.65 * digits.max(3) as f32 + 16.)
                .height(Length::Fill),
            )
            .id("editor-line-numbers"),
        );
    }
    body = body.push(editor).height(Length::Fill);
    column![
        body,
        container(
            text(format!("{}:{}", cursor.line + 1, cursor.column + 1))
                .size(11)
                .style(theme::MUTED.text())
        )
        .padding([4, 12])
    ]
    .height(Length::Fill)
    .into()
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if app.editor.pending.is_none() || app.editor.saving_all {
        return base;
    }
    let busy = app
        .tabs
        .files
        .iter()
        .any(|tab| tab.editor.as_ref().is_some_and(|buffer| buffer.saving));
    let mut files = column![text(app.i18n.text(Key::EditorUnsavedTitle)).size(20)].spacing(12);
    for tab in app.tabs.files.iter().filter(|tab| {
        app.editor_pending_tab(tab.id) && tab.editor.as_ref().is_some_and(Buffer::protected)
    }) {
        files = files.push(text(&tab.title).size(13));
        if let Some(error) = tab.editor.as_ref().and_then(|buffer| buffer.error) {
            files = files.push(text(app.i18n.text(error)).size(12).style(theme::RED.text()));
        }
    }
    if busy {
        files = files.push(text(app.i18n.text(Key::EditorSaving)).size(12));
    }
    let action = |key, event, enabled: bool| {
        button(text(app.i18n.text(key)).size(13))
            .on_press_maybe(enabled.then_some(Message::Editor(event)))
            .style(theme::action(false))
    };
    files = files.push(
        row![
            action(Key::Cancel, Event::Cancel, true),
            space().width(Length::Fill),
            action(Key::EditorDiscard, Event::Discard, !busy),
            action(Key::EditorSaveAll, Event::SaveAll, !busy)
        ]
        .spacing(8),
    );
    crate::ui::modal::overlay(base, files.into(), 560., Message::Editor(Event::Cancel))
}
