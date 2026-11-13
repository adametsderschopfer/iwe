use super::*;
use crate::{
    domain::repository::GraphRow,
    i18n::{I18n, Language},
};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace
        .create_space("Work".to_owned(), vec![PathBuf::from("project")]);
    app.loaded = true;
    app
}

#[test]
fn collapse_mode_applies_to_all_tabs_new_files_and_restored_preferences() {
    use crate::domain::changes::Source;
    let document = || {
        Document::from_source(
            Source {
                text: "same\nnew\nsame\n".into(),
                previous: Some("same\nold\nsame\n".into()),
                ..Source::default()
            },
            "rs",
        )
    };
    let mut app = app();
    let first = open_change(&mut app, "first.rs");
    let _ = app.finish_preview(first, Ok(document()));
    let second = open_change(&mut app, "second.rs");
    let _ = app.finish_preview(second, Ok(document()));
    let _ = app.update(Message::ToggleUnchanged(second));
    assert!(app.workspace.preferences.collapse_unchanged);
    assert!(
        app.tabs
            .files
            .iter()
            .all(|preview| preview.folding.collapsed)
    );
    let third = open_change(&mut app, "third.rs");
    let _ = app.finish_preview(third, Ok(document()));
    assert!(app.preview().folding.collapsed);
    let serialized = serde_json::to_value(&app.workspace).unwrap();
    let mut restored = App::new(Rc::new(I18n::new(Language::English)));
    restored.workspace = serde_json::from_value(serialized).unwrap();
    restored.workspace.normalize();
    let id = open_change(&mut restored, "restored.rs");
    let _ = restored.finish_preview(id, Ok(document()));
    assert!(restored.preview().folding.collapsed);
    let _ = app.update(Message::ToggleUnchanged(third));
    assert!(!app.workspace.preferences.collapse_unchanged);
    assert!(
        app.tabs
            .files
            .iter()
            .all(|preview| !preview.folding.collapsed)
    );
}

fn open(app: &mut App, path: &str) -> u64 {
    let _ = app.open_file(PathBuf::from("project").join(path));
    app.preview().id
}

fn open_change(app: &mut App, path: &str) -> u64 {
    app.file_panel = crate::domain::workspace::FilePanel::Changes;
    let _ = app.open_working_change(PathBuf::from("project").join(path));
    app.preview().id
}

fn loaded(app: &mut App, id: u64, text: &str) {
    let _ = app.update(Message::FileLoaded(
        app.generation,
        id,
        Ok(Document::new(text.to_owned(), "rs")),
    ));
    app.preview_mut().editor = None;
}

fn diff_settings() -> iced::Settings {
    iced::Settings {
        default_font: theme::font(),
        ..Default::default()
    }
}

fn comparison_app() -> App {
    let mut app = app();
    app.panes = iced::widget::pane_grid::State::new(super::super::Panel::Workspace).0;
    let id = open_change(&mut app, "balance.py");
    let before = "from decimal import Decimal\n\ndef get_balance(account):\n    balance = account.balance\n    fee = Decimal('10.00')\n    return balance - fee\n\ndef serialize_account(account):\n    return {\n        'name': account.name,\n        'balance': get_balance(account),\n        'currency': 'USD',\n    }\n\n# End of account helpers\n";
    let after = "from decimal import Decimal\n\ndef get_balance(account):\n    balance = account.balance\n    fee = Decimal('5.00')\n    discount = account.discount\n    return balance - fee + discount\n\ndef serialize_account(account):\n    return {\n        'name': account.name,\n        'balance': get_balance(account),\n    }\n\n# End of account helpers\n\ndef is_active(account):\n    return account.enabled\n";
    let _ = app.finish_preview(
        id,
        Ok(Document::from_source(
            crate::domain::changes::Source {
                text: after.into(),
                previous: Some(before.into()),
                ..Default::default()
            },
            "py",
        )),
    );
    assert!(app.preview().editor.is_none());
    app
}

#[test]
fn appearance_switch_preserves_prepared_files_and_diff_positions() {
    use crate::domain::workspace::Appearance;
    for view in [
        crate::domain::workspace::DiffView::SideBySide,
        crate::domain::workspace::DiffView::Unified,
    ] {
        let mut app = comparison_app();
        app.workspace.preferences.diff_view = view;
        let id = app.preview().id;
        let _ = app.update(Message::ToggleUnchanged(id));
        app.preview_mut().scroll_y = 32.;
        app.preview_mut().diff_scroll_y = 42.;
        let document = Arc::clone(app.preview().document.as_ref().unwrap());
        let revision = app.preview().folding.revision;
        let request = app.file_request;
        for appearance in [Appearance::Light, Appearance::Dark] {
            let _ = app.update(Message::SetAppearance(appearance));
            assert!(Arc::ptr_eq(
                &document,
                app.preview().document.as_ref().unwrap()
            ));
            assert_eq!(app.preview().id, id);
            assert_eq!(app.file_request, request);
            assert_eq!(app.preview().folding.revision, revision);
            assert_eq!(app.preview().scroll_y, 32.);
            assert_eq!(app.preview().diff_scroll_y, 42.);
            let mut ui =
                iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&app.theme())
                    .unwrap()
                    .matches_image(output.join(format!("appearance-{view:?}-{appearance:?}")))
                    .unwrap();
            }
        }
    }
}

#[test]
fn diff_toolbar_switches_both_views_in_all_languages_and_window_sizes() {
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        for width in [1080., 1440.] {
            let mut app = comparison_app();
            app.i18n = Rc::new(I18n::new(language));
            let mut ui =
                iced_test::Simulator::with_size(diff_settings(), (width, 920.), app.view());
            let before = ui
                .find(Id::new("source-before"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            let after = ui
                .find(Id::new("source-after"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(before.width > 150. && after.width > 150.);
            assert_eq!(before.y, after.y);
            assert_eq!(before.height, after.height);
            assert!(before.x + before.width < after.x);
            assert!(after.x + after.width <= width);
            for key in [crate::i18n::Key::DiffBefore, crate::i18n::Key::DiffAfter] {
                assert!(
                    ui.find(app.i18n.text(key))
                        .unwrap()
                        .visible_bounds()
                        .is_some()
                );
            }
            if language == Language::Russian
                && width == 1440.
                && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
            {
                ui.snapshot(&iced::Theme::Dark)
                    .unwrap()
                    .matches_image(output.join("side-by-side-diff"))
                    .unwrap();
            }
            let button = ui
                .click(Id::new("toggle-diff-view"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(button.x + button.width <= width);
            for message in ui.into_messages() {
                let _ = app.update(message);
            }
            assert_eq!(app.workspace.preferences.diff_view, DiffView::Unified);
            let mut ui =
                iced_test::Simulator::with_size(diff_settings(), (width, 920.), app.view());
            assert!(ui.find(Id::new("source-code")).is_ok());
            assert!(ui.find(Id::new("source-before")).is_err());
            let old_number = ui
                .find(Id::new("unified-number-0-0"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            let new_number = ui
                .find(Id::new("unified-number-1-0"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert_eq!(old_number.y, new_number.y);
            assert!(old_number.x + old_number.width < new_number.x);
            assert!(new_number.x + new_number.width < width);
            assert!(ui.find("~").is_err());
            assert!(ui.find("−").is_err());
            if language == Language::Russian
                && width == 1440.
                && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
            {
                ui.snapshot(&iced::Theme::Dark)
                    .unwrap()
                    .matches_image(output.join("unified-diff"))
                    .unwrap();
            }
            ui.click(Id::new("toggle-diff-view")).unwrap();
            for message in ui.into_messages() {
                let _ = app.update(message);
            }
            assert_eq!(app.workspace.preferences.diff_view, DiffView::SideBySide);
        }
    }
}

#[test]
fn diff_modes_preserve_documents_folds_scroll_and_saved_preferences() {
    use crate::ui::code::folding::SideRow;
    let mut app = comparison_app();
    let id = app.preview().id;
    let document = Arc::clone(app.preview().document.as_ref().unwrap());
    let _ = app.update(Message::CodeScrolled { id, x: 31., y: 88. });
    let _ = app.update(Message::DiffScrolled {
        id,
        side: Side::Before,
        x: 13.,
        y: 160.,
    });
    let _ = app.update(Message::DiffScrolled {
        id,
        side: Side::After,
        x: 47.,
        y: 160.,
    });
    let _ = app.update(Message::ToggleUnchanged(id));
    let range = match app.preview().folding.side_row(0) {
        SideRow::Fold { range, .. } => range,
        _ => panic!("Unchanged opening lines must collapse"),
    };
    let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
    ui.click(iced::widget::Id::from(format!(
        "diff-fold-{id}-0-{}",
        range.start
    )))
    .unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    assert!(matches!(
        app.preview().folding.side_row(0),
        SideRow::Line(_)
    ));
    let _ = app.update(Message::ToggleDiffView(id));
    assert_eq!(app.workspace.preferences.diff_view, DiffView::Unified);
    assert!(matches!(
        app.preview().folding.row(0),
        crate::ui::code::folding::Row::Line(_)
    ));
    let _ = app.update(Message::ToggleDiffView(id));
    assert!(Arc::ptr_eq(
        &document,
        app.preview().document.as_ref().unwrap()
    ));
    let _ = app.update(Message::ToggleUnchanged(id));
    assert_eq!((app.preview().scroll_x, app.preview().scroll_y), (31., 88.));
    assert_eq!(
        (app.preview().diff_scroll_x, app.preview().diff_scroll_y),
        ([13., 47.], 160.)
    );
    let _ = app.update(Message::ToggleDiffView(id));
    let saved = serde_json::to_value(&app.workspace).unwrap();
    let restored: crate::domain::workspace::Workspace = serde_json::from_value(saved).unwrap();
    assert_eq!(restored.preferences.diff_view, DiffView::Unified);
    let other = open(&mut app, "other.py");
    assert_ne!(id, other);
    let _ = app.finish_preview(other, Ok((*document).clone()));
    assert_eq!(app.workspace.preferences.diff_view, DiffView::Unified);
    let _ = app.update(Message::ToggleDiffView(id));
    let _ = app.update(Message::DiffScrolled {
        id,
        side: Side::Before,
        x: 999.,
        y: 999.,
    });
    assert_eq!(app.workspace.preferences.diff_view, DiffView::Unified);
    assert_eq!(app.preview().diff_scroll_y, 0.);
}

#[test]
fn unified_folds_expand_and_line_number_setting_hides_both_columns() {
    use iced::widget::Id;
    let mut app = comparison_app();
    app.i18n = Rc::new(I18n::new(Language::Russian));
    app.workspace.preferences.diff_view = DiffView::Unified;
    let id = app.preview().id;
    let _ = app.update(Message::ToggleUnchanged(id));
    let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
    assert!(ui.find(Id::new("unified-number-0-0")).is_err());
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        ui.snapshot(&iced::Theme::Dark)
            .unwrap()
            .matches_image(output.join("unified-fold-separators"))
            .unwrap();
    }
    ui.click(Id::from(format!("unchanged-{id}-0"))).unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
    for side in [0, 1] {
        assert!(
            ui.find(Id::from(format!("unified-number-{side}-0")))
                .unwrap()
                .visible_bounds()
                .is_some()
        );
    }
    drop(ui);
    let _ = app.update(Message::SetLineNumbers(false));
    let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
    for side in [0, 1] {
        assert!(
            ui.find(Id::from(format!("unified-number-{side}-0")))
                .is_err()
        );
    }
    assert!(
        ui.find(Id::new("unified-line-0"))
            .unwrap()
            .visible_bounds()
            .is_some()
    );
}

#[test]
fn ordinary_files_do_not_show_a_diff_switch() {
    let mut app = app();
    let id = open(&mut app, "plain.txt");
    loaded(&mut app, id, "unchanged");
    let mut ui = iced_test::Simulator::with_size(diff_settings(), (1080., 720.), app.view());
    assert!(ui.find(iced::widget::Id::new("toggle-diff-view")).is_err());
    assert!(ui.find(iced::widget::Id::new("source-code")).is_ok());
    assert!(
        ui.find(iced::widget::Id::new("source-overview-unified"))
            .is_err()
    );
}

#[test]
fn change_overviews_leave_scrollbar_dragging_and_wheel_scrolling_available() {
    use iced::{Event, Point, mouse, widget::Id};
    for mode in [DiffView::SideBySide, DiffView::Unified] {
        for width in [1080., 1440.] {
            let mut app = comparison_app();
            app.workspace.preferences.diff_view = mode;
            app.i18n = Rc::new(I18n::new(Language::Russian));
            let id = app.preview().id;
            let before = (0..220)
                .map(|index| format!("value_{index} = {index}\n"))
                .collect::<String>();
            let after = before
                .replace(
                    "value_10 = 10\n",
                    "value_10 = 10\nbonus = 25\nready = True\n",
                )
                .replace("value_100 = 100\nvalue_101 = 101\n", "")
                .replace("value_205 = 205\n", "value_205 = 1000\n");
            let _ = app.finish_preview(
                id,
                Ok(Document::from_source(
                    crate::domain::changes::Source {
                        text: after,
                        previous: Some(before),
                        ..Default::default()
                    },
                    "py",
                )),
            );
            let targets: &[(&str, &str)] = if mode == DiffView::SideBySide {
                &[
                    ("source-before", "source-overview-before"),
                    ("source-after", "source-overview-after"),
                ]
            } else {
                &[("source-code", "source-overview-unified")]
            };
            for &(source, overview) in targets {
                let mut ui =
                    iced_test::Simulator::with_size(diff_settings(), (width, 920.), app.view());
                let pane = ui.find(Id::new(source)).unwrap().visible_bounds().unwrap();
                let marks = ui
                    .find(Id::new(overview))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert_eq!(marks.width, theme::CODE_CHANGE_MARK_WIDTH);
                assert!((marks.x + marks.width - pane.x - pane.width).abs() < 0.1);
                assert_eq!(marks.y, pane.y);
                assert!(marks.y + marks.height <= pane.y + pane.height);
                if width == 1440.
                    && source != "source-after"
                    && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
                {
                    ui.snapshot(&iced::Theme::Dark)
                        .unwrap()
                        .matches_image(output.join(format!("change-overview-{mode:?}")))
                        .unwrap();
                }
                ui.point_at(marks.center());
                ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0., y: -100. },
                })]);
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(messages.iter().any(|message| matches!(message,
                    Message::CodeScrolled { y, .. } | Message::DiffScrolled { y, .. } if *y > 0.
                )));
                let mut ui =
                    iced_test::Simulator::with_size(diff_settings(), (width, 920.), app.view());
                let start = Point::new(
                    pane.x + pane.width - theme::CODE_SCROLLBAR_WIDTH / 2.,
                    pane.y + 8.,
                );
                let end = Point::new(marks.center_x(), marks.y + marks.height - 2.);
                ui.point_at(start);
                ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
                    mouse::Button::Left,
                ))]);
                ui.point_at(end);
                ui.simulate([
                    Event::Mouse(mouse::Event::CursorMoved { position: end }),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ]);
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(messages.iter().any(|message| matches!(message,
                    Message::CodeScrolled { y, .. } | Message::DiffScrolled { y, .. } if *y > 4000.
                )), "Dragging across the marker lane must reach the document end");
            }
        }
    }
}

#[tokio::test]
async fn diff_scrolling_synchronizes_widgets_and_keeps_horizontal_offsets_independent() {
    use iced::{Rectangle, Vector, futures::StreamExt, mouse};
    use iced_test::{
        core::{
            clipboard,
            renderer::Headless,
            widget::{
                Id, Operation,
                operation::{self, Scrollable},
            },
        },
        runtime::{Action, UserInterface, task, user_interface},
    };
    #[derive(Default)]
    struct Offsets([Option<Vector>; 2]);
    impl Operation for Offsets {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _: Rectangle,
            _: Rectangle,
            offset: Vector,
            _: &mut dyn Scrollable,
        ) {
            for side in [Side::Before, Side::After] {
                if id == Some(&Id::new(side.scroll_id())) {
                    self.0[side.index()] = Some(offset);
                }
            }
        }
    }
    let mut app = comparison_app();
    let id = app.preview().id;
    let before: String = (0..120)
        .map(|index| format!("line_{index} = '{}'\n", "wide source ".repeat(8)))
        .collect();
    let after = before.replace("line_100 =", "changed_line =");
    let _ = app.finish_preview(
        id,
        Ok(Document::from_source(
            crate::domain::changes::Source {
                text: after,
                previous: Some(before),
                ..Default::default()
            },
            "py",
        )),
    );
    app.preview_mut().diff_scroll_x = [25., 75.];
    let mut renderer =
        iced_test::renderer::Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let size = iced::Size::new(1440., 920.);
    let mut cache = user_interface::Cache::default();
    for _ in 0..2 {
        cache = UserInterface::build(app.view(), size, cache, &mut renderer).into_cache();
    }
    let mut ui = UserInterface::build(app.view(), size, cache, &mut renderer);
    let mut stream = task::into_stream(app.restore_preview_scroll()).unwrap();
    while let Some(action) = stream.next().await {
        if let Action::Widget(mut operation) = action {
            ui.operate(&renderer, operation.as_mut());
        }
    }
    cache = ui.into_cache();
    for (side, delta, expected) in [(Side::Before, -240., 240.), (Side::After, 95., 145.)] {
        let mut ui = UserInterface::build(app.view(), size, cache, &mut renderer);
        let mut selector = iced_test::Selector::find(Id::new(side.scroll_id()));
        ui.operate(&renderer, &mut operation::black_box(&mut selector));
        let operation::Outcome::Some(Some(pane)) = selector.finish() else {
            panic!("Comparison pane must exist")
        };
        let mut messages = Vec::new();
        ui.update(
            &[iced::Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0., y: delta },
            })],
            mouse::Cursor::Available(pane.visible_bounds().unwrap().center()),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        assert!(messages.iter().any(|message| matches!(message, Message::DiffScrolled { side: changed, .. } if *changed == side)));
        cache = ui.into_cache();
        let commands = iced::Task::batch(
            messages
                .into_iter()
                .map(|message| app.update(message))
                .collect::<Vec<_>>(),
        );
        let mut stream = task::into_stream(commands).unwrap();
        let mut ui = UserInterface::build(app.view(), size, cache, &mut renderer);
        while let Some(action) = stream.next().await {
            if let Action::Widget(mut operation) = action {
                ui.operate(&renderer, operation.as_mut());
            }
        }
        let mut offsets = Offsets::default();
        ui.operate(&renderer, &mut offsets);
        for (index, x) in [25., 75.].into_iter().enumerate() {
            let offset = offsets.0[index].unwrap();
            assert!((offset.y - expected).abs() < 0.5, "{offset:?}");
            assert_eq!(offset.x, x);
        }
        cache = ui.into_cache();
    }
    let _ = app.update(Message::ToggleDiffView(id));
    cache = UserInterface::build(app.view(), size, cache, &mut renderer).into_cache();
    let _ = app.update(Message::ToggleDiffView(id));
    let mut ui = UserInterface::build(app.view(), size, cache, &mut renderer);
    let mut stream = task::into_stream(app.restore_preview_scroll()).unwrap();
    while let Some(action) = stream.next().await {
        if let Action::Widget(mut operation) = action {
            ui.operate(&renderer, operation.as_mut());
        }
    }
    let mut offsets = Offsets::default();
    ui.operate(&renderer, &mut offsets);
    assert_eq!(
        offsets.0,
        [Some(Vector::new(25., 145.)), Some(Vector::new(75., 145.))]
    );
}

#[tokio::test]
async fn opening_changes_displays_text_before_highlighting_and_comparison() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("small.rs");
    let source = "fn main() {\n    let value = 42;\n}\n";
    std::fs::write(&path, source).unwrap();
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec![directory.path().to_owned()]);
    let mut next = app.open_working_change(path);
    for stage in [Preparation::Text, Preparation::Syntax, Preparation::Changes] {
        let mut stream = task::into_stream(next).unwrap();
        let message = loop {
            if let Action::Output(message) = stream.next().await.expect("next preparation stage") {
                break message;
            }
        };
        if stage == Preparation::Changes {
            assert!(matches!(message, Message::FileLoaded(_, _, Ok(_))));
        } else {
            assert!(
                matches!(&message, Message::FilePrepared(_, _, completed, Ok(_)) if *completed == stage)
            );
        }
        next = app.update(message);
        let document = app.preview().document.as_ref().unwrap();
        assert_eq!(document.text, source);
        assert_eq!(document.lines.len(), 3);
        assert_eq!(app.preview().loading, stage != Preparation::Changes);
        if stage == Preparation::Text {
            assert!(document.lines.iter().all(|line| line.len() == 1));
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                (1080., 720.),
                app.view(),
            );
            assert!(
                ui.find(iced::widget::Id::new("source-code"))
                    .unwrap()
                    .visible_bounds()
                    .is_some()
            );
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&iced::Theme::Dark)
                    .unwrap()
                    .matches_image(output.join("file-loading-progress"))
                    .unwrap();
            }
        } else {
            assert!(document.lines[0].len() > 1);
        }
    }
}

#[test]
fn closing_and_reopening_an_incomplete_preview_rejects_old_stages() {
    let mut app = app();
    let original = open(&mut app, "small.rs");
    let generation = app.generation;
    let _ = app.update(Message::FilePrepared(
        generation,
        original,
        Preparation::Text,
        Ok(Document::plain("current".into())),
    ));
    assert_eq!(app.preview().preparation, Preparation::Syntax);
    let _ = app.close_tab(original);
    let _ = app.reopen_closed_tab();
    let reopened = app.preview().id;
    assert_ne!(original, reopened);
    assert!(app.preview().loading);
    assert_eq!(app.preview().preparation, Preparation::Text);
    let _ = app.update(Message::FilePrepared(
        generation,
        original,
        Preparation::Syntax,
        Ok(Document::new("stale".into(), "rs")),
    ));
    let _ = app.update(Message::FilePrepared(
        generation + 1,
        reopened,
        Preparation::Text,
        Ok(Document::plain("wrong project".into())),
    ));
    let _ = app.update(Message::FilePrepared(
        generation,
        reopened,
        Preparation::Syntax,
        Ok(Document::new("wrong order".into(), "rs")),
    ));
    assert_eq!(app.preview().document.as_ref().unwrap().text, "current");
    let _ = app.update(Message::FilePrepared(
        generation,
        reopened,
        Preparation::Text,
        Err(Failure::Read),
    ));
    assert!(!app.preview().loading);
    assert_eq!(app.preview().error, Some(Failure::Read));
}

#[test]
fn expanded_ranges_remain_specific_to_each_tab_under_the_shared_mode() {
    use crate::{domain::changes::Source, ui::code::folding::Row};
    use iced::widget::Id;
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = app();
        app.workspace.preferences.diff_view = DiffView::Unified;
        app.i18n = Rc::new(I18n::new(language));
        let first = open_change(&mut app, "diff.rs");
        let tail = (0..40)
            .map(|index| format!("// unchanged {index}\n"))
            .collect::<String>();
        let original = format!("fn main() {{\n    let old = 1;\n    keep();\n}}\n{tail}");
        let current = format!("fn main() {{\n    let new = 2;\n    keep();\n}}\n{tail}");
        let document = Document::from_source(
            Source {
                text: current.clone(),
                previous: Some(original),
                ..Source::default()
            },
            "rs",
        );
        let _ = app.finish_preview(first, Ok(document));
        assert!(app.preview().editor.is_none());
        app.preview_mut().scroll_y = 100.;
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        ui.click(Id::new("fold-unchanged")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.preview().folding.collapsed);
        assert_eq!(app.preview().scroll_y, 0.);
        assert_eq!(app.preview().folding.row(0), Row::Fold(0..1));
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("folded-code-{suffix}")))
                .unwrap();
        }
        ui.click(Id::from(format!("unchanged-{first}-0"))).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(
            matches!(messages.as_slice(), [Message::ExpandUnchanged(id, range)] if *id == first && *range == (0..1))
        );
        for message in messages {
            let _ = app.update(message);
        }
        assert_eq!(app.preview().folding.row(0), Row::Line(0));
        let second = open(&mut app, "plain.rs");
        loaded(&mut app, second, "plain");
        assert!(!app.preview().folding.collapsed);
        let _ = app.update(Message::SelectTab(first));
        assert!(app.preview().folding.collapsed);
        let _ = app.update(Message::ToggleUnchanged(first));
        assert!(!app.preview().folding.collapsed);
        assert_eq!(app.preview().scroll_y, 100.);
        assert_eq!(app.preview().document.as_ref().unwrap().text, current);
        let _ = app.update(Message::ToggleUnchanged(first));
        app.preview_mut().revision = Some("commit".to_owned());
        assert!(app.preview().folding.collapsed);
        let refreshed = Document::new("clean\n".to_owned(), "rs");
        let _ = app.finish_preview(first, Ok(refreshed));
        assert!(!app.preview().folding.collapsed);
        assert_eq!(app.preview().scroll_y, 0.);
    }
}

#[test]
fn selecting_the_open_project_does_not_refresh_or_reset_its_state() {
    let mut app = app();
    let id = open(&mut app, "src/main.rs");
    loaded(&mut app, id, "current source");
    app.modules.git.branch = Some(crate::domain::repository::project::BranchKey {
        root: PathBuf::from("project"),
        reference: "refs/heads/feature".to_owned(),
    });
    app.modules.git.query = "search".to_owned();
    app.modules.git.history_scroll_y = 450.;
    let before = (app.generation, app.repo_request, app.file_request);
    let _ = app.update(Message::SelectProject(0, 0));
    assert_eq!((app.generation, app.repo_request, app.file_request), before);
    assert_eq!(app.preview().id, id);
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "current source"
    );
    assert_eq!(
        app.modules
            .git
            .branch
            .as_ref()
            .map(|key| key.reference.as_str()),
        Some("refs/heads/feature")
    );
    assert_eq!(app.modules.git.query, "search");
    assert_eq!(app.modules.git.history_scroll_y, 450.);
    assert!(!app.saving);
}

#[tokio::test]
async fn footer_copies_the_complete_active_file_path_and_falls_back_to_project() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, clipboard, task};
    let mut app = app();
    let path = PathBuf::from("project").join("src").join("main.rs");
    let _ = app.open_file(path.clone());
    let id = app.preview().id;
    loaded(&mut app, id, "source");
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    let bounds = ui
        .click(iced::widget::Id::new("footer-path"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    assert!(
        bounds.width < 300.,
        "The copy target must fit the short path"
    );
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>().as_slice(),
        [Message::CopyPath]
    ));
    for expected in [path, PathBuf::from("project")] {
        let mut stream = task::into_stream(app.update(Message::CopyPath)).unwrap();
        match stream.next().await.unwrap() {
            Action::Clipboard(clipboard::Action::Write { contents, .. }) => {
                assert_eq!(contents, expected.to_string_lossy())
            }
            action => panic!("Expected a clipboard write, got {action:?}"),
        }
        let _ = app.close_tab(id);
    }
}

#[test]
fn switching_tabs_reuses_documents_and_restores_both_scroll_axes() {
    let mut app = app();
    let first = open(&mut app, "src/first.rs");
    loaded(&mut app, first, "first");
    let document = app.preview().document.as_ref().unwrap().text.as_ptr();
    let _ = app.update(Message::CodeScrolled {
        id: first,
        x: 180.,
        y: 640.,
    });
    let second = open(&mut app, "src/second.rs");
    loaded(&mut app, second, "second");
    let _ = app.update(Message::CodeScrolled {
        id: second,
        x: 35.,
        y: 90.,
    });

    assert_eq!(open(&mut app, "src/first.rs"), first);
    assert_eq!(app.tabs.files.len(), 2);
    assert_eq!(
        app.preview().document.as_ref().unwrap().text.as_ptr(),
        document
    );
    assert_eq!(
        (app.preview().scroll_x, app.preview().scroll_y),
        (180., 640.)
    );
    assert!(!app.preview().loading);
    assert_eq!(
        app.files.selected_path,
        Some(PathBuf::from("project/src/first.rs"))
    );

    let _ = app.update(Message::SelectTab(second));
    assert_eq!((app.preview().scroll_x, app.preview().scroll_y), (35., 90.));
    let _ = app.update(Message::SelectTab(first));
    assert_eq!(app.preview().id, first);
    let _ = app.update(Message::SelectTab(second));
    assert_eq!(app.preview().id, second);
}

#[test]
fn wide_commit_tab_keeps_its_label_clear_of_the_close_button() {
    use iced::widget::Id;
    let mut app = app();
    let filename = format!("{}.vue", "W".repeat(40));
    let id = open(&mut app, &filename);
    app.preview_mut().revision = Some("51ad1920123456789".into());
    let label = crate::ui::tab_label::label(&filename, app.preview().revision.as_deref());
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings {
            default_font: theme::font(),
            ..iced::Settings::default()
        },
        (1920., 720.),
        app.view(),
    );
    let text = ui.find(label.as_str()).unwrap().visible_bounds().unwrap();
    let close_id = Id::from(format!("close-file-tab-{id}"));
    let close = ui.find(close_id.clone()).unwrap().visible_bounds().unwrap();
    assert!(
        text.x + text.width < close.x,
        "The commit label must not cover the close button"
    );
    ui.click(close_id).unwrap();
    assert!(
        ui.into_messages()
            .any(|message| matches!(message, Message::CloseTab(target) if target == id))
    );
}

#[test]
fn middle_click_closes_only_its_tab_and_wheel_scrolling_keeps_tabs() {
    use iced::{Event, mouse, widget::Id};
    let mut app = app();
    let first = open(&mut app, "first.rs");
    let second = open(&mut app, "second.rs");
    let third = open(&mut app, "third.rs");
    for target in [second, third, first] {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        let bounds = ui
            .find(Id::from(format!("file-tab-{target}")))
            .unwrap()
            .visible_bounds()
            .unwrap();
        ui.point_at(bounds.center());
        ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: -1. },
        })]);
        ui.simulate([
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)),
        ]);
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(matches!(messages.as_slice(), [Message::CloseTab(id)] if *id == target));
        let _ = app.update(messages.into_iter().next().unwrap());
        assert!(!app.tabs.files.iter().any(|tab| tab.id == target));
        if target == second {
            assert_eq!(app.preview().id, third);
        }
    }
    assert!(app.preview().title.is_empty());
}

#[test]
fn close_shortcut_closes_tabs_without_exiting_and_keeps_their_documents_for_reopening() {
    use iced::keyboard::{Event, Key, Modifiers, key};
    let event = |name: &str, code, modifiers| {
        iced::Event::Keyboard(Event::KeyPressed {
            key: Key::Character(name.into()),
            modified_key: Key::Character(name.into()),
            physical_key: key::Physical::Code(code),
            location: iced::keyboard::Location::Standard,
            modifiers,
            repeat: false,
            text: None,
        })
    };
    let mut app = app();
    let first = open(&mut app, "first.rs");
    loaded(&mut app, first, "first");
    let second = open(&mut app, "second.rs");
    loaded(&mut app, second, "second");
    let document = app.preview().document.as_ref().unwrap().text.as_ptr();
    for expected in [Some("first.rs"), None, None] {
        let _ = app
            .update(crate::app::shortcut(event("w", key::Code::KeyW, Modifiers::COMMAND)).unwrap());
        assert_eq!(app.tabs.files.len(), 1);
        assert_eq!(
            app.preview().path,
            expected.map(|path| PathBuf::from("project").join(path))
        );
    }
    for expected in ["first.rs", "second.rs"] {
        let _ = app.update(
            crate::app::shortcut(event(
                "t",
                key::Code::KeyT,
                Modifiers::COMMAND | Modifiers::SHIFT,
            ))
            .unwrap(),
        );
        assert_eq!(
            app.preview().path,
            Some(PathBuf::from("project").join(expected))
        );
    }
    assert_eq!(
        app.preview().document.as_ref().unwrap().text.as_ptr(),
        document
    );
}

#[test]
fn closing_the_active_tab_keeps_tabs_during_dialogs_and_settings() {
    let mut app = app();
    let first = open(&mut app, "first.rs");
    let second = open(&mut app, "second.rs");
    app.space_draft = Some(crate::app::spaces::Draft::default());
    let _ = app.update(Message::CloseActiveTab);
    assert_eq!(app.preview().id, second);
    app.space_draft = None;
    app.navigation = crate::app::Navigation::Settings;
    let _ = app.update(Message::CloseActiveTab);
    assert_eq!(app.preview().id, second);
    app.navigation = crate::app::Navigation::Files;
    let _ = app.update(Message::CloseActiveTab);
    assert_eq!(app.preview().id, first);
    let _ = app.update(Message::CloseActiveTab);
    assert!(app.preview().title.is_empty());
    let _ = app.update(Message::CloseActiveTab);
    assert!(app.preview().title.is_empty());
}

#[test]
fn closed_tabs_reopen_in_reverse_order_with_cached_documents_and_scroll_positions() {
    let mut app = app();
    let first = open(&mut app, "first.rs");
    loaded(&mut app, first, "first");
    let first_document = app.preview().document.as_ref().unwrap().text.as_ptr();
    app.preview_mut().scroll_x = 45.;
    app.preview_mut().scroll_y = 320.;
    let second = open(&mut app, "second.rs");
    loaded(&mut app, second, "second");
    let second_document = app.preview().document.as_ref().unwrap().text.as_ptr();
    let _ = app.update(Message::CloseTab(first));
    let _ = app.update(Message::CloseActiveTab);
    let _ = app.update(Message::CloseActiveTab);
    assert_eq!(app.tabs.closed.len(), 2);

    app.space_draft = Some(crate::app::spaces::Draft::default());
    let _ = app.update(Message::ReopenClosedTab);
    assert!(app.preview().path.is_none());
    app.space_draft = None;
    app.navigation = crate::app::Navigation::Settings;
    let _ = app.update(Message::ReopenClosedTab);
    assert!(app.preview().path.is_none());
    app.navigation = crate::app::Navigation::Files;

    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.preview().title, "second.rs");
    assert_ne!(app.preview().id, second);
    assert_eq!(
        app.preview().document.as_ref().unwrap().text.as_ptr(),
        second_document
    );
    assert!(!app.preview().loading);
    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.preview().title, "first.rs");
    assert_ne!(app.preview().id, first);
    assert_eq!(
        app.preview().document.as_ref().unwrap().text.as_ptr(),
        first_document
    );
    assert_eq!(
        (app.preview().scroll_x, app.preview().scroll_y),
        (45., 320.)
    );
    assert_eq!(app.files.selected_path, app.preview().path);
    let active = app.preview().id;
    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.preview().id, active);
    assert_eq!(app.tabs.files.len(), 2);
}

#[test]
fn closed_documents_have_a_byte_budget_without_losing_tab_history() {
    let mut app = app();
    // Distinct lines exercise eviction even when repeated lines share their allocations.
    let text: String = (0..8000)
        .map(|index| format!("let value_{index} = {index}; // Closed preview.\n"))
        .collect();
    let mut documents = Vec::new();
    let mut original_bytes = 0;
    for index in 0..20 {
        let id = open(&mut app, &format!("file-{index}.rs"));
        let document = Document::plain(text.clone());
        original_bytes += document.memory_bytes();
        let _ = app.finish_preview(id, Ok(document));
        documents.push(Arc::downgrade(app.preview().document.as_ref().unwrap()));
        app.preview_mut().scroll_y = index as f32 * 24.;
        let _ = app.close_tab(id);
    }
    let retained: usize = app
        .tabs
        .closed
        .iter()
        .filter_map(|preview| preview.document.as_ref())
        .map(|document| document.memory_bytes())
        .sum();
    eprintln!(
        "Closed documents: {} bytes before budget, {} bytes retained",
        original_bytes, retained
    );
    assert!(original_bytes > CLOSED_DOCUMENT_BYTES * 2);
    assert!(retained <= CLOSED_DOCUMENT_BYTES);
    assert!(documents[0].upgrade().is_none());
    assert!(documents[19].upgrade().is_some());
    assert_eq!(app.tabs.closed.len(), 20);
    for index in (0..20).rev() {
        let _ = app.reopen_closed_tab();
        assert_eq!(app.preview().title, format!("file-{index}.rs"));
        assert_eq!(app.preview().scroll_y, index as f32 * 24.);
        assert_eq!(app.preview().loading, app.preview().document.is_none());
    }
}

#[tokio::test]
async fn suspended_pages_reopen_from_disk_and_detect_changed_files() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large.txt");
    let content = "a\n".repeat(400_000);
    std::fs::write(&path, &content).unwrap();
    let source = Source::Working {
        compare: false,
        root: directory.path().into(),
        path: path.clone(),
    };
    let first = source
        .clone()
        .read_page(TextCursor::default(), None)
        .await
        .unwrap();
    let page = first.page.unwrap();
    let second = source
        .clone()
        .read_page(page.next.unwrap(), page.version)
        .await
        .unwrap();
    let second_page = second.page.unwrap();
    for changed in [false, true] {
        let mut app = app();
        let id = open(&mut app, "large.txt");
        app.preview_mut().source = Some(source.clone());
        app.preview_mut().page_history = vec![page.cursor];
        let _ = app.finish_preview(id, Ok(second.clone()));
        app.preview_mut().scroll_y = 240.;
        let _ = app.close_tab(id);
        app.tabs.trim_closed_documents(0);
        assert!(app.tabs.closed[0].document.is_none());
        if changed {
            std::fs::write(&path, "changed\n").unwrap();
        }
        let mut actions = task::into_stream(app.reopen_closed_tab()).unwrap();
        let mut loaded = None;
        while let Some(action) = actions.next().await {
            if let Action::Output(Message::FilePrepared(_, _, Preparation::Text, result)) = action {
                loaded = Some(result.unwrap());
            }
        }
        let loaded = loaded.unwrap();
        assert_eq!(
            loaded.page.unwrap().cursor,
            if changed {
                TextCursor::default()
            } else {
                second_page.cursor
            }
        );
        let id = app.preview().id;
        let _ = app.continue_preview(id, Preparation::Text, Ok(loaded.clone()));
        let _ = app.finish_preview(id, Ok(loaded));
        assert_eq!(app.preview().scroll_y, if changed { 0. } else { 240. });
        assert_eq!(app.preview().page_history.len(), usize::from(!changed));
    }
}

#[test]
fn suspended_comparisons_restore_expanded_folds_only_for_matching_content() {
    use crate::ui::code::folding::Row;
    for changed in [false, true] {
        let mut app = comparison_app();
        let id = app.preview().id;
        let _ = app.toggle_unchanged(id);
        let document = app.preview().document.as_ref().unwrap().as_ref().clone();
        let range = (0..app.preview().folding.len(&document))
            .find_map(|index| match app.preview().folding.row(index) {
                Row::Fold(range) => Some(range),
                Row::Line(_) => None,
            })
            .unwrap();
        let _ = app.expand_unchanged(id, range.clone());
        let count = app.preview().folding.len(&document);
        app.preview_mut().scroll_y = 96.;
        app.preview_mut().diff_scroll_y = 144.;
        let _ = app.close_tab(id);
        app.tabs.trim_closed_documents(0);
        assert!(app.tabs.closed[0].document.is_none());
        let _ = app.reopen_closed_tab();
        let id = app.preview().id;
        let _ = app.continue_preview(
            id,
            Preparation::Text,
            Ok(Document::plain(document.text.clone())),
        );
        let restored = if changed {
            Document::from_source(
                crate::domain::changes::Source {
                    text: format!("new_entry = 1\n{}", document.text),
                    previous: Some(document.text),
                    ..Default::default()
                },
                "py",
            )
        } else {
            document
        };
        let _ = app.finish_preview(id, Ok(restored));
        let preview = app.preview();
        assert!(preview.suspended.is_none());
        assert!(preview.folding.collapsed);
        if changed {
            assert!(preview.folding.len(preview.document.as_ref().unwrap()) < count);
            assert_eq!(preview.diff_scroll_y, 0.);
        } else {
            assert_eq!(
                preview.folding.len(preview.document.as_ref().unwrap()),
                count
            );
            assert_eq!((preview.scroll_y, preview.diff_scroll_y), (96., 144.));
            assert!(matches!(preview.folding.row(range.start), Row::Line(_)));
        }
    }
}

#[test]
fn restoring_tabs_skips_open_duplicates_and_projects_keep_their_closure_history() {
    let mut app = app();
    let first = open(&mut app, "first.rs");
    loaded(&mut app, first, "first");
    let second = open(&mut app, "second.rs");
    loaded(&mut app, second, "second");
    let _ = app.close_tab(first);
    let _ = app.close_tab(second);
    let manual = open(&mut app, "second.rs");
    let _ = app.update(Message::Refresh);
    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.tabs.files.len(), 2);
    assert_eq!(app.preview().title, "first.rs");
    assert!(app.tabs.files.iter().any(|tab| tab.id == manual));
    let _ = app.update(Message::CloseActiveTab);
    app.workspace
        .create_space("Other".into(), vec![PathBuf::from("other")]);
    let _ = app.load_directory();
    let _ = app.update(Message::ReopenClosedTab);
    assert!(app.preview().path.is_none());
    assert!(app.tabs.closed.is_empty());
    app.workspace.select_project(0, 0);
    let _ = app.load_directory();
    assert_eq!(app.preview().title, "second.rs");
    assert!(app.preview().loading);
    assert_eq!(app.tabs.closed.len(), 1);
    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.preview().title, "first.rs");
    assert_eq!(app.preview().document.as_ref().unwrap().text, "first");
    assert_eq!(app.tabs.files.len(), 2);
}

#[test]
fn switching_projects_restores_tabs_documents_positions_and_pending_reads() {
    let mut app = app();
    let ready = open(&mut app, "ready.rs");
    loaded(&mut app, ready, "ready source");
    let _ = app.update(Message::CodeScrolled {
        id: ready,
        x: 21.,
        y: 64.,
    });
    let pending = open(&mut app, "pending.rs");
    assert!(app.preview().loading);
    let ready_document = Arc::clone(
        app.tabs
            .files
            .iter()
            .find(|tab| tab.id == ready)
            .unwrap()
            .document
            .as_ref()
            .unwrap(),
    );
    let _ = app.close_tab(ready);
    assert_eq!(app.tabs.closed.len(), 1);

    app.workspace
        .create_space("Other".into(), vec![PathBuf::from("other")]);
    let _ = app.load_directory();
    assert!(app.preview().path.is_none());
    assert!(app.tabs.closed.is_empty());
    loaded(&mut app, pending, "stale result");
    assert!(app.preview().path.is_none());
    assert!(app.preview().document.is_none());

    app.workspace.select_project(0, 0);
    let _ = app.load_directory();
    assert_eq!(app.preview().id, pending);
    assert_eq!(app.preview().title, "pending.rs");
    assert!(app.preview().loading);
    assert_eq!(app.tabs.closed.len(), 1);
    let _ = app.update(Message::FilePrepared(
        app.generation - 1,
        pending,
        Preparation::Text,
        Ok(Document::new("stale".into(), "rs")),
    ));
    assert!(app.preview().loading);
    assert!(app.preview().document.is_none());
    loaded(&mut app, pending, "restored source");
    assert!(!app.preview().loading);
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "restored source"
    );

    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.preview().title, "ready.rs");
    assert_eq!((app.preview().scroll_x, app.preview().scroll_y), (21., 64.));
    assert!(Arc::ptr_eq(
        &ready_document,
        app.preview().document.as_ref().unwrap()
    ));
}

#[test]
fn protected_drafts_travel_with_their_project_when_switches_bypass_the_prompt() {
    let mut app = app();
    let id = open(&mut app, "draft.rs");
    let _ = app.finish_preview(id, Ok(Document::new("draft text".into(), "rs")));
    let buffer = app.preview_mut().editor.as_mut().unwrap();
    buffer.saving = true;
    let document = Arc::clone(app.preview().document.as_ref().unwrap());

    app.workspace
        .create_space("Other".into(), vec![PathBuf::from("other")]);
    let _ = app.load_directory();
    assert!(app.preview().path.is_none());
    let stashed = app
        .tabs
        .stash
        .iter()
        .find(|(root, _)| *root == PathBuf::from("project"))
        .unwrap();
    assert!(stashed.1.files[0].editor.is_some());
    assert!(stashed.1.files[0].document.is_some());

    app.workspace.select_project(0, 0);
    let _ = app.load_directory();
    assert_eq!(app.preview().id, id);
    assert_eq!(app.preview().title, "draft.rs");
    let buffer = app.preview().editor.as_ref().unwrap();
    assert!(buffer.saving);
    assert_eq!(buffer.document.text, "draft text");
    assert!(Arc::ptr_eq(
        &document,
        app.preview().document.as_ref().unwrap()
    ));
}

#[test]
fn the_stash_keeps_only_the_four_most_recent_projects() {
    let mut app = app();
    let first = open(&mut app, "first.rs");
    loaded(&mut app, first, "first");
    for index in 0..5 {
        app.workspace.create_space(
            format!("Space {index}"),
            vec![PathBuf::from(format!("root-{index}"))],
        );
        let _ = app.load_directory();
        let id = open(&mut app, "tab.rs");
        loaded(&mut app, id, "background source");
    }
    assert_eq!(app.tabs.stash.len(), 4);
    assert!(
        app.tabs
            .stash
            .iter()
            .all(|(root, _)| *root != PathBuf::from("project"))
    );

    app.workspace.select_project(3, 0);
    let _ = app.load_directory();
    assert_eq!(app.preview().title, "project/tab.rs");
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "background source"
    );
    // Restoring a project removes its entry while the departing project joins the stash.
    assert_eq!(app.tabs.stash.len(), 3);

    app.workspace.select_project(0, 0);
    let _ = app.load_directory();
    assert!(app.preview().title.is_empty());
    assert!(app.preview().path.is_none());
    assert_eq!(app.tabs.stash.len(), 4);
}

#[tokio::test]
async fn stashed_documents_beyond_the_budget_reload_from_disk_on_return() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large.txt");
    let content = "stashed line\n".repeat(1_400_000);
    std::fs::write(&path, &content).unwrap();
    let mut app = app();
    let id = open(&mut app, "large.txt");
    app.preview_mut().source = Some(Source::Working {
        compare: false,
        root: directory.path().to_owned(),
        path: path.clone(),
    });
    let _ = app.finish_preview(id, Ok(Document::plain(content.clone())));
    assert!(app.preview().document.is_some());
    app.preview_mut().scroll_y = 240.;

    app.workspace
        .create_space("Other".into(), vec![PathBuf::from("other")]);
    let _ = app.load_directory();
    let stashed = app
        .tabs
        .stash
        .iter()
        .find(|(root, _)| *root == PathBuf::from("project"))
        .unwrap();
    let preview = &stashed.1.files[0];
    assert!(preview.document.is_none());
    assert!(preview.suspended.is_some());
    assert!(preview.editor.is_none());

    app.workspace.select_project(0, 0);
    let mut actions = task::into_stream(app.sync_preview_project()).unwrap();
    let mut loaded = None;
    while let Some(action) = actions.next().await {
        if let Action::Output(Message::FilePrepared(_, _, Preparation::Text, result)) = action {
            loaded = Some(result.unwrap());
        }
    }
    let loaded = loaded.expect("The restored preview must reread its document");
    assert!(loaded.page.is_some());
    assert_eq!(loaded.text, content[..loaded.text.len()]);
    let _ = app.continue_preview(id, Preparation::Text, Ok(loaded.clone()));
    let _ = app.finish_preview(id, Ok(loaded));
    assert!(!app.preview().loading);
    assert!(app.preview().document.is_some());
    assert_eq!(app.preview().scroll_y, 240.);
}

#[test]
fn restoring_pending_or_failed_tabs_retries_and_rejects_previous_read_results() {
    let mut app = app();
    for failure in [false, true] {
        let old = open(&mut app, "pending.rs");
        if failure {
            let _ = app.finish_preview(old, Err(Failure::Read));
        }
        let _ = app.close_tab(old);
        let _ = app.update(Message::ReopenClosedTab);
        let restored = app.preview().id;
        assert_ne!(restored, old);
        assert!(app.preview().loading);
        assert!(app.preview().error.is_none());
        loaded(&mut app, old, "outdated");
        assert!(app.preview().loading);
        loaded(&mut app, restored, "restored");
        assert_eq!(app.preview().document.as_ref().unwrap().text, "restored");
        assert!(!app.preview().loading);
    }
}

#[test]
fn pending_reads_finish_in_their_tab_and_cannot_reopen_closed_tabs() {
    let mut app = app();
    let first = open(&mut app, "first.rs");
    let second = open(&mut app, "second.rs");
    loaded(&mut app, first, "first result");
    assert_eq!(app.preview().id, second);
    assert!(app.preview().loading);
    let _ = app.update(Message::SelectTab(first));
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "first result"
    );

    let _ = app.update(Message::CloseTab(second));
    loaded(&mut app, second, "closed result");
    assert_eq!(app.tabs.files.len(), 1);
    assert_eq!(app.preview().id, first);
    let reopened = open(&mut app, "second.rs");
    assert_ne!(reopened, second);
    loaded(&mut app, second, "late result");
    assert!(app.preview().loading);
    loaded(&mut app, reopened, "reopened result");
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "reopened result"
    );

    let _ = app.update(Message::CloseActiveTab);
    assert_eq!(app.preview().id, first);
    let _ = app.update(Message::CloseActiveTab);
    assert!(app.preview().title.is_empty());
    loaded(&mut app, first, "late first result");
    assert!(app.preview().document.is_none());
}

#[test]
fn commit_versions_have_separate_tabs_from_working_files() {
    let mut app = app();
    let working = open(&mut app, "src/lib.rs");
    loaded(&mut app, working, "working");
    let commit = Commit {
        id: "a".repeat(40),
        parents: Vec::new(),
        subject: "Snapshot".to_owned(),
        author: "Author".to_owned(),
        date: "2026-11-08".to_owned(),
        timestamp: 0,
        refs: String::new(),
        graph: GraphRow::default(),
    };
    let file = ChangedFile {
        path: "src/lib.rs".to_owned(),
        status: 'M',
    };
    let source = Source::Commit {
        root: PathBuf::from("project"),
        commit: Box::new(commit.clone()),
        file: file.clone(),
    };
    let _ = app.open_source(source.clone());
    let historical = app.preview().id;
    loaded(&mut app, historical, "historical");
    let _ = app.open_source(source);
    assert_eq!(app.tabs.files.len(), 2);
    assert_eq!(app.preview().id, historical);
    assert_eq!(app.preview().revision.as_deref(), Some(commit.id.as_str()));
    assert_eq!(open(&mut app, "src/lib.rs"), working);
    assert_eq!(app.preview().document.as_ref().unwrap().text, "working");

    let _ = app.open_source(Source::Commit {
        root: PathBuf::from("project"),
        commit: Box::new(Commit {
            id: "b".repeat(40),
            ..commit
        }),
        file,
    });
    assert_eq!(app.tabs.files.len(), 3);
    let _ = app.close_tab(historical);
    let _ = app.update(Message::ReopenClosedTab);
    assert_eq!(app.tabs.files.len(), 3);
    assert_eq!(
        app.preview().revision.as_deref(),
        Some("a".repeat(40).as_str())
    );
    assert_eq!(app.preview().document.as_ref().unwrap().text, "historical");
    assert!(!app.preview().loading);
}

#[test]
fn refresh_keeps_tabs_and_project_changes_reject_pending_reads() {
    let mut app = app();
    let ready = open(&mut app, "ready.rs");
    loaded(&mut app, ready, "ready");
    let pending = open(&mut app, "pending.rs");
    let generation = app.generation;
    let _ = app.update(Message::Refresh);
    assert_eq!(app.tabs.files.len(), 2);
    assert_eq!(app.preview().id, pending);
    let _ = app.update(Message::FileLoaded(generation, pending, Err(Failure::Read)));
    assert!(app.preview().loading);
    loaded(&mut app, pending, "after refresh");
    assert!(!app.preview().loading);
    let _ = app.update(Message::SelectTab(ready));
    assert_eq!(app.preview().document.as_ref().unwrap().text, "ready");

    app.workspace
        .create_space("Other".to_owned(), vec![PathBuf::from("other")]);
    let _ = app.load_directory();
    loaded(&mut app, pending, "old project");
    assert!(app.preview().title.is_empty());
    assert!(app.preview().document.is_none());
}

#[test]
fn reopening_failed_files_retries_in_the_same_tab_slot() {
    let mut app = app();
    let first = open(&mut app, "failed.rs");
    let _ = app.update(Message::FileLoaded(
        app.generation,
        first,
        Err(Failure::Read),
    ));
    let retry = open(&mut app, "failed.rs");
    assert_eq!(app.tabs.files.len(), 1);
    assert_ne!(retry, first);
    assert!(app.preview().loading);
    loaded(&mut app, first, "stale");
    assert!(app.preview().loading);
    loaded(&mut app, retry, "retried");
    assert_eq!(app.preview().document.as_ref().unwrap().text, "retried");
}

#[test]
fn closing_a_tab_does_not_trigger_its_selection_button() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        let first = open(&mut app, "src/first.rs");
        loaded(&mut app, first, "first");
        let second = open(&mut app, "src/second.rs");
        loaded(&mut app, second, "second");
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings {
                default_font: theme::font(),
                ..Default::default()
            },
            (1080., 720.),
            app.view(),
        );
        ui.click(iced::widget::Id::from(format!("close-file-tab-{first}")))
            .unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(matches!(messages.as_slice(), [Message::CloseTab(id)] if *id == first));
        let _ = app.update(messages.into_iter().next().unwrap());
        assert_eq!(app.preview().id, second);
    }
}

#[tokio::test]
async fn switching_tabs_restores_widget_offsets_and_reveals_overflowed_tabs() {
    use iced::{Rectangle, Vector, futures::StreamExt};
    use iced_test::{
        core::{
            renderer::Headless,
            widget::{Id, Operation, operation::Scrollable},
        },
        runtime::{Action, UserInterface, task, user_interface},
    };

    #[derive(Default)]
    struct Offsets {
        code: Option<Vector>,
        tabs: Option<Vector>,
    }
    impl Operation for Offsets {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _: Rectangle,
            _: Rectangle,
            translation: Vector,
            _: &mut dyn Scrollable,
        ) {
            if id == Some(&Id::new("source-code")) {
                self.code = Some(translation);
            }
            if id == Some(&Id::new("file-tabs")) {
                self.tabs = Some(translation);
            }
        }
    }

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace
        .create_space("Work".to_owned(), vec![root.to_owned()]);
    let source: String = (1..=300)
        .map(|index| format!("let line_{index} = \"{}\";\n", "wide source ".repeat(20)))
        .collect();
    let mut ids = Vec::new();
    for number in 0..8 {
        let name = if number % 2 == 0 {
            format!("{}_file_{number}.rs", "W".repeat(32))
        } else {
            format!("file_{number}.rs")
        };
        let path = root.join(name);
        std::fs::write(&path, &source).unwrap();
        let _ = app.open_file(path);
        let id = app.preview().id;
        loaded(&mut app, id, &source);
        ids.push(id);
    }
    let first = ids[0];
    let last = ids[7];
    let _ = app.update(Message::CodeScrolled {
        id: first,
        x: 180.,
        y: 640.,
    });
    let _ = app.update(Message::CodeScrolled {
        id: last,
        x: 35.,
        y: 90.,
    });
    let mut renderer =
        iced_test::renderer::Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let size = iced::Size::new(1080., 720.);
    let mut cache = user_interface::Cache::default();
    for _ in 0..2 {
        cache = UserInterface::build(app.view(), size, cache, &mut renderer).into_cache();
    }
    for (id, expected) in [
        (first, Vector::new(180., 640.)),
        (last, Vector::new(35., 90.)),
        (first, Vector::new(180., 640.)),
    ] {
        let commands = app.update(Message::SelectTab(id));
        let mut stream = task::into_stream(commands).unwrap();
        let mut messages = Vec::new();
        let mut ui = UserInterface::build(app.view(), size, cache, &mut renderer);
        while let Some(action) = stream.next().await {
            match action {
                Action::Widget(mut operation) => {
                    ui.operate(&renderer, operation.as_mut());
                }
                Action::Output(message) => messages.push(message),
                _ => {}
            }
        }
        let mut offsets = Offsets::default();
        ui.operate(&renderer, &mut offsets);
        assert_eq!(offsets.code, Some(expected));
        let tabs = offsets.tabs.unwrap();
        assert_eq!(tabs.x > 0., id == last);
        let mut selector = iced_test::Selector::find(Id::from(format!("file-tab-{id}")));
        ui.operate(
            &renderer,
            &mut iced_test::core::widget::operation::black_box(&mut selector),
        );
        let iced_test::core::widget::operation::Outcome::Some(Some(tab)) = selector.finish() else {
            panic!("The active tab must exist");
        };
        assert!(
            tab.visible_bounds().is_some(),
            "The active tab must remain visible"
        );
        cache = ui.into_cache();
        for message in messages {
            let _ = app.update(message);
        }
    }
}

#[test]
fn overflow_arrow_lists_open_tabs_and_selects_a_hidden_file_in_every_language() {
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.loaded = true;
        app.workspace
            .create_space("Work".into(), vec![PathBuf::from("project")]);
        let first = open(&mut app, "first.rs");
        {
            let mut ui =
                iced_test::Simulator::with_size(diff_settings(), (1080., 720.), app.view());
            assert!(ui.find(Id::new("open-tabs")).is_err());
        }
        let mut ids = vec![first];
        for index in 1..12 {
            ids.push(open(&mut app, &format!("file-{index}.rs")));
        }
        {
            let mut ui =
                iced_test::Simulator::with_size(diff_settings(), (6000., 720.), app.view());
            assert!(ui.find(Id::new("open-tabs")).is_err());
        }
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1080., 720.), app.view());
        ui.click(Id::new("open-tabs")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.tabs.menu.is_some());
        let target = ids[4];
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1080., 720.), app.view());
        ui.click(Id::from(format!("open-file-tab-{target}")))
            .unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.preview().id, target);
        assert_eq!(app.tabs.files.len(), ids.len());
        assert!(app.tabs.menu.is_none());
        let _ = app.update(Message::OpenTabMenu(iced::Point::new(500., 100.)));
        let _ = app.update(Message::DismissDialog);
        assert!(app.tabs.menu.is_none());
        assert_eq!(app.preview().id, target);
    }
}

#[test]
fn remote_comparison_keeps_local_preview_until_result_and_never_steals_focus() {
    let mut app = app();
    let path = PathBuf::from("project/compare.txt");
    let _ = app.begin_remote_comparison(
        "server".into(),
        "Test server".into(),
        path.clone(),
        "/srv/compare.txt".into(),
    );
    let initial = app.preview().id;
    assert_eq!(app.active_local_file(), Some(path.clone()));
    assert!(app.preview().pending_comparison.is_some());
    let _ = app.finish_preview(initial, Ok(Document::new("local copy\n".into(), "txt")));
    assert!(app.preview().pending_comparison.is_some());
    let selected = open(&mut app, "other.txt");
    let _ = app.open_remote_preview(
        path,
        "/srv/compare.txt".into(),
        ("server".into(), "Test server".into()),
        Arc::new("local copy\n".into()),
        Some(Arc::new("server copy\n".into())),
    );
    assert_eq!(app.preview().id, selected);
    let comparison = app
        .tabs
        .files
        .iter()
        .find(|p| p.pending_comparison.is_some())
        .unwrap();
    assert!(comparison.comparison_received());
    assert_eq!(comparison.document.as_ref().unwrap().text, "local copy\n");
    let id = comparison.id;
    // A delayed local preparation response must not replace the server result.
    let _ = app.finish_preview(initial, Ok(Document::new("obsolete\n".into(), "txt")));
    let _ = app.finish_preview(
        id,
        Ok(Document::from_source(
            crate::domain::changes::Source {
                text: "local copy\n".into(),
                previous: Some("server copy\n".into()),
                ..Default::default()
            },
            "txt",
        )),
    );
    let completed = app.tabs.files.iter().find(|p| p.id == id).unwrap();
    assert!(completed.pending_comparison.is_none());
    assert!(completed.document.as_ref().unwrap().comparison.is_some());
    assert_eq!(app.preview().id, selected);
}

#[test]
fn closed_comparison_does_not_reopen_when_server_finishes() {
    let mut app = app();
    let path = PathBuf::from("project/compare.txt");
    let _ = app.begin_remote_comparison(
        "server".into(),
        "Test server".into(),
        path.clone(),
        "/srv/compare.txt".into(),
    );
    let id = app.preview().id;
    let _ = app.update(Message::CloseTab(id));
    let count = app.tabs.files.len();
    let _ = app.open_remote_preview(
        path,
        "/srv/compare.txt".into(),
        ("server".into(), "Test server".into()),
        Arc::new("local\n".into()),
        Some(Arc::new("remote\n".into())),
    );
    assert_eq!(app.tabs.files.len(), count);
    assert!(
        app.tabs
            .files
            .iter()
            .all(|p| p.pending_comparison.is_none())
    );
}

#[test]
fn pending_comparison_renders_two_labeled_panes_in_every_language() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        let _ = app.begin_remote_comparison(
            "server".into(),
            "Test server".into(),
            "project/compare.txt".into(),
            "/srv/compare.txt".into(),
        );
        let id = app.preview().id;
        let _ = app.finish_preview(id, Ok(Document::new("local copy\n".into(), "txt")));
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
        let remote = ui
            .find(app.i18n.text(crate::i18n::Key::DeploymentRemote))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let local = ui
            .find(app.i18n.text(crate::i18n::Key::DeploymentLocal))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(remote.x < local.x);
        let content = ui
            .find(iced::widget::Id::new("source-code"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(content.x >= local.x - 12.);
    }
}

#[tokio::test]
async fn pending_remote_comparison_reads_local_text_without_waiting_for_network() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("compare.txt");
    std::fs::write(&path, "local copy\n").unwrap();
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace
        .create_space("Test".into(), vec![directory.path().to_owned()]);
    let mut next = app.begin_remote_comparison(
        "server".into(),
        "Test server".into(),
        path.clone(),
        "/srv/compare.txt".into(),
    );
    for _ in 0..3 {
        let mut stream = task::into_stream(next).unwrap();
        let message = loop {
            if let Action::Output(message) = stream.next().await.unwrap() {
                break message;
            }
        };
        next = app.update(message);
        assert_eq!(
            app.preview().document.as_ref().unwrap().text,
            "local copy\n"
        );
        assert!(app.preview().pending_comparison.is_some());
    }
    assert!(!app.preview().loading);
    assert!(
        app.preview()
            .document
            .as_ref()
            .unwrap()
            .comparison
            .is_none()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "local copy\n");
}

#[tokio::test]
async fn remote_comparison_prepares_full_result_in_background() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let mut app = app();
    let path = PathBuf::from("project/compare.txt");
    let _ = app.begin_remote_comparison(
        "server".into(),
        "Test server".into(),
        path.clone(),
        "/srv/compare.txt".into(),
    );
    let id = app.preview().id;
    let _ = app.finish_preview(id, Ok(Document::new("local\n".into(), "txt")));
    let selected = open(&mut app, "other.txt");
    let next = app.open_remote_preview(
        path,
        "/srv/compare.txt".into(),
        ("server".into(), "Test server".into()),
        Arc::new("local\n".into()),
        Some(Arc::new("server\n".into())),
    );
    let mut stream = task::into_stream(next).unwrap();
    let message = loop {
        if let Action::Output(message) = stream.next().await.unwrap() {
            break message;
        }
    };
    assert!(matches!(message, Message::FileLoaded(_, _, Ok(_))));
    let _ = app.update(message);
    assert_eq!(app.preview().id, selected);
    let comparison = app.tabs.files.iter().find(|p| p.remote).unwrap();
    assert!(comparison.pending_comparison.is_none());
    assert_eq!(comparison.document.as_ref().unwrap().text, "local\n");
    assert!(comparison.document.as_ref().unwrap().comparison.is_some());
    let id = comparison.id;
    let _ = app.select_tab(id);
    let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
    assert!(
        ui.find(app.i18n.text(crate::i18n::Key::DeploymentRemote))
            .unwrap()
            .visible_bounds()
            .is_some()
    );
    assert!(
        ui.find(app.i18n.text(crate::i18n::Key::DeploymentLocal))
            .unwrap()
            .visible_bounds()
            .is_some()
    );
}

#[tokio::test]
async fn explicit_remote_comparison_keeps_identical_and_empty_files_side_by_side() {
    for text in ["same content\n", ""] {
        let source = Source::Remote {
            path: "same.txt".into(),
            remote: "/srv/same.txt".into(),
            label: "Test server".into(),
            identity: "server".into(),
            local_root: None,
            text: Arc::new(text.into()),
            previous: Some(Arc::new(text.into())),
        };
        let document = source
            .clone()
            .compare(Arc::new(Document::new(text.into(), "txt")))
            .await
            .unwrap();
        let comparison = document
            .comparison
            .as_ref()
            .expect("Explicit comparisons retain both sides");
        assert!(comparison.hunks.is_empty());
        assert_eq!(document.text, text);
        let mut app = app();
        let _ = app.open_source(source);
        let id = app.preview().id;
        let _ = app.finish_preview(id, Ok(document));
        assert!(app.preview().editor.is_none());
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
        for key in [
            crate::i18n::Key::DeploymentRemote,
            crate::i18n::Key::DeploymentLocal,
        ] {
            assert!(
                ui.find(app.i18n.text(key))
                    .unwrap()
                    .visible_bounds()
                    .is_some()
            );
        }
    }
}

#[tokio::test]
async fn project_files_skip_git_comparison_and_have_separate_change_tabs() {
    use iced::futures::StreamExt;
    use iced_test::runtime::Action;
    let mut app = app();
    let path = PathBuf::from("project/file.txt");
    let _ = app.open_file(path.clone());
    let local = app.preview().id;
    app.preview_mut().preparation = Preparation::Syntax;
    let task = app.continue_preview(
        local,
        Preparation::Syntax,
        Ok(Document::plain("local".into())),
    );
    if let Some(mut stream) = iced_test::runtime::task::into_stream(task) {
        while let Some(action) = stream.next().await {
            assert!(!matches!(action, Action::Output(_)));
        }
    }
    assert!(!app.preview().loading);
    assert!(app.preview().editor.is_some());
    assert!(
        app.preview()
            .document
            .as_ref()
            .unwrap()
            .comparison
            .is_none()
    );
    let _ = app.open_working_change(path.clone());
    let change = app.preview().id;
    assert_ne!(local, change);
    app.preview_mut().preparation = Preparation::Syntax;
    let task = app.continue_preview(
        change,
        Preparation::Syntax,
        Ok(Document::plain("local".into())),
    );
    assert!(iced_test::runtime::task::into_stream(task).is_some());
    assert_eq!(app.preview().preparation, Preparation::Changes);
    assert!(app.preview().editor.is_none());
    let _ = app.open_file(path);
    assert_eq!(app.preview().id, local);
    let _ = app.refresh_saved_editor(local);
    app.preview_mut().preparation = Preparation::Syntax;
    let _ = app.continue_preview(
        local,
        Preparation::Syntax,
        Ok(Document::plain("saved".into())),
    );
    assert!(!app.preview().loading);
    assert!(
        app.preview()
            .document
            .as_ref()
            .unwrap()
            .comparison
            .is_none()
    );
}

#[test]
fn comparison_controls_follow_the_document_in_every_file_section() {
    use crate::domain::workspace::FilePanel;
    use iced::widget::Id;
    let mut app = comparison_app();
    for panel in [FilePanel::Project, FilePanel::Changes] {
        app.file_panel = panel;
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1080., 720.), app.view());
        for control in ["toggle-diff-view", "fold-unchanged"] {
            assert!(ui.find(Id::from(control)).is_ok());
        }
    }
    app.preview_mut().document = Some(Arc::new(Document::plain("local file\n".into())));
    for panel in [FilePanel::Project, FilePanel::Changes] {
        app.file_panel = panel;
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1080., 720.), app.view());
        for control in ["toggle-diff-view", "fold-unchanged"] {
            assert!(ui.find(Id::from(control)).is_err());
        }
    }
}

#[test]
fn server_comparisons_keep_the_saved_layout_and_controls_in_project_files() {
    use iced::widget::Id;
    for mode in [DiffView::Unified, DiffView::SideBySide] {
        let mut app = app();
        app.file_panel = crate::domain::workspace::FilePanel::Project;
        app.workspace.preferences.diff_view = mode;
        let path = PathBuf::from("project/compare.txt");
        let local = "unchanged\n".repeat(30) + "local\n";
        let remote = "unchanged\n".repeat(30) + "server\n";
        let _ = app.begin_remote_comparison(
            "server".into(),
            "Test server".into(),
            path.clone(),
            "/srv/compare.txt".into(),
        );
        let id = app.preview().id;
        let _ = app.finish_preview(id, Ok(Document::new(local.clone(), "txt")));
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
        let source = ui
            .find(Id::new("source-code"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert_eq!(
            ui.find(app.i18n.text(crate::i18n::Key::DeploymentLocal))
                .is_ok(),
            mode == DiffView::SideBySide
        );
        if mode == DiffView::Unified {
            assert!(source.width > 600.);
        }
        drop(ui);
        let _ = app.open_remote_preview(
            path,
            "/srv/compare.txt".into(),
            ("server".into(), "Test server".into()),
            Arc::new(local.clone()),
            Some(Arc::new(remote.clone())),
        );
        let document = Document::from_source(
            crate::domain::changes::Source {
                text: local,
                previous: Some(remote),
                ..Default::default()
            },
            "txt",
        );
        let _ = app.finish_preview(app.preview().id, Ok(document));
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
        assert!(ui.find(Id::new("fold-unchanged")).is_ok());
        assert_eq!(
            ui.find(Id::new("source-before")).is_ok(),
            mode == DiffView::SideBySide
        );
        ui.click(Id::new("toggle-diff-view")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_ne!(app.workspace.preferences.diff_view, mode);
    }
}

#[test]
fn server_target_popup_stays_inside_the_active_file() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        app.workspace.preferences.show_git = false;
        let id = open(&mut app, "example.rs");
        loaded(&mut app, id, "fn main() {\n    println!(\"Hello\");\n}\n");
        app.modules.deployment.target = Some((
            crate::modules::deployment::Operation::Compare(PathBuf::from("project/example.rs")),
            vec![
                ("dev".into(), "Development".into()),
                ("prod".into(), "Production".into()),
            ],
        ));
        let mut ui = iced_test::Simulator::with_size(diff_settings(), (1440., 920.), app.view());
        let tabs = ui
            .find(iced::widget::Id::new("file-tabs-header"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let target = ui
            .find(iced::widget::Id::new("deployment-target-0"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(target.x >= tabs.x && target.y >= tabs.y + tabs.height);
        assert!(target.width < 400.);
        assert!(
            ui.find(app.i18n.text(crate::i18n::Key::DeploymentCancel))
                .is_err()
        );
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            std::fs::create_dir_all(&output).unwrap();
            ui.snapshot(&app.theme())
                .unwrap()
                .matches_image(output.join(format!("server-popup-{language:?}")))
                .unwrap();
        }
        drop(ui);
        let _ = open(&mut app, "other.rs");
        assert!(app.modules.deployment.target.is_none());
        app.modules.deployment.target = Some((
            crate::modules::deployment::Operation::Compare(PathBuf::from("project/other.rs")),
            vec![("dev".into(), "Development".into())],
        ));
        let _ = app.select_tab(id);
        assert!(app.modules.deployment.target.is_none());
    }
}

#[tokio::test]
async fn server_comparison_reuses_only_the_matching_highlighted_local_snapshot() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    for unchanged in [true, false] {
        let mut app = app();
        let path = PathBuf::from("project/compare.rs");
        let _ = app.begin_remote_comparison(
            "server".into(),
            "Server".into(),
            path.clone(),
            "/compare.rs".into(),
        );
        let id = app.preview().id;
        let original = "let value = 1;\n";
        let _ = app.finish_preview(id, Ok(Document::new(original.into(), "rs")));
        let prepared = app.preview().document.clone().unwrap();
        let text = if unchanged {
            original
        } else {
            "let value = 2;\n"
        };
        let task = app.open_remote_preview(
            path,
            "/compare.rs".into(),
            ("server".into(), "Server".into()),
            Arc::new(text.into()),
            Some(Arc::new("let value = 0;\n".into())),
        );
        assert_eq!(
            app.preview().preparation,
            if unchanged {
                Preparation::Changes
            } else {
                Preparation::Text
            }
        );
        let mut stream = task::into_stream(task).unwrap();
        while let Some(action) = stream.next().await {
            if let Action::Output(message) = action {
                if unchanged {
                    let Message::FileLoaded(_, _, Ok(document)) = &message else {
                        panic!("Matching syntax must skip text and syntax stages");
                    };
                    let after = document.comparison.as_ref().unwrap().rows[0].cells[1]
                        .as_ref()
                        .unwrap();
                    assert!(Arc::ptr_eq(
                        &document.lines[after.index],
                        &prepared.lines[0]
                    ));
                } else {
                    assert!(matches!(
                        message,
                        Message::FilePrepared(_, _, Preparation::Text, Ok(_))
                    ));
                }
                let _ = app.update(message);
            }
        }
    }
}
