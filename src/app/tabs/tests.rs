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
    let first = open(&mut app, "first.rs");
    let _ = app.finish_preview(first, Ok(document()));
    let second = open(&mut app, "second.rs");
    let _ = app.finish_preview(second, Ok(document()));
    let _ = app.update(Message::ToggleUnchanged(second));
    assert!(app.workspace.preferences.collapse_unchanged);
    assert!(
        app.tabs
            .files
            .iter()
            .all(|preview| preview.folding.collapsed)
    );
    let third = open(&mut app, "third.rs");
    let _ = app.finish_preview(third, Ok(document()));
    assert!(app.preview().folding.collapsed);
    let serialized = serde_json::to_value(&app.workspace).unwrap();
    let mut restored = App::new(Rc::new(I18n::new(Language::English)));
    restored.workspace = serde_json::from_value(serialized).unwrap();
    restored.workspace.normalize();
    let id = open(&mut restored, "restored.rs");
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

fn loaded(app: &mut App, id: u64, text: &str) {
    let _ = app.update(Message::FileLoaded(
        app.generation,
        id,
        Ok(Document::new(text.to_owned(), "rs")),
    ));
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
    let id = open(&mut app, "balance.py");
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
async fn opening_displays_text_before_highlighting_and_comparison() {
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
    let mut next = app.open_file(path);
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
        let first = open(&mut app, "diff.rs");
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
    app.branch = Some("refs/heads/feature".to_owned());
    app.query = "search".to_owned();
    app.history_scroll_y = 450.;
    let before = (app.generation, app.repo_request, app.file_request);
    let _ = app.update(Message::SelectProject(0, 0));
    assert_eq!((app.generation, app.repo_request, app.file_request), before);
    assert_eq!(app.preview().id, id);
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "current source"
    );
    assert_eq!(app.branch.as_deref(), Some("refs/heads/feature"));
    assert_eq!(app.query, "search");
    assert_eq!(app.history_scroll_y, 450.);
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
    ui.click(iced::widget::Id::new("footer-path")).unwrap();
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
fn closing_application_works_from_every_screen_and_dialog_without_closing_tabs() {
    let mut app = app();
    assert_eq!(app.update(Message::CloseApplication).units(), 1);
    let active = open(&mut app, "first.rs");
    for navigation in [
        crate::app::Navigation::Projects,
        crate::app::Navigation::Files,
        crate::app::Navigation::Settings,
    ] {
        for dialog in [false, true] {
            app.navigation = navigation;
            app.space_draft = dialog.then(crate::app::spaces::Draft::default);
            assert_eq!(app.update(Message::CloseApplication).units(), 1);
            assert_eq!(app.preview().id, active);
            assert_eq!(app.tabs.files.len(), 1);
        }
    }
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
fn restoring_tabs_skips_open_duplicates_and_resets_history_when_projects_change() {
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
        let path = root.join(format!("file_{number}.rs"));
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
