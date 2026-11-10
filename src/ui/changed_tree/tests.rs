use super::*;
use crate::i18n::Language;
use iced::widget::{container, scrollable};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Open(usize),
    Toggle(String),
}

#[test]
fn active_file_highlight_changes_when_the_active_file_changes() {
    let output = tempfile::tempdir().unwrap();
    let i18n = I18n::new(Language::English);
    let render = |selected| {
        let entries = ["first.rs", "second.rs"]
            .into_iter()
            .enumerate()
            .map(|(index, path)| Entry {
                path: path.into(),
                status: FileStatus::Modified,
                selected: Some(index) == selected,
                open: Message::Open(index),
            })
            .collect();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (320., 120.),
            view(entries, &BTreeSet::new(), &i18n, Message::Toggle, 536., 0),
        );
        ui.snapshot(&iced::Theme::Dark).unwrap()
    };
    let baseline = output.path().join("selected-first");
    assert!(render(Some(0)).matches_hash(&baseline).unwrap());
    assert!(!render(Some(1)).matches_hash(&baseline).unwrap());
    assert!(!render(None).matches_hash(&baseline).unwrap());
    assert!(render(Some(0)).matches_hash(&baseline).unwrap());
}

#[test]
fn compact_folders_show_localized_counts_and_open_the_correct_file_without_badges() {
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let i18n = I18n::new(language);
        let entries = || {
            [
                ("tests/unit/apps/api/views/created.py", FileStatus::Added),
                ("tests/unit/apps/api/views/updated.py", FileStatus::Modified),
                ("services/bank_accounts.py", FileStatus::Added),
            ]
            .into_iter()
            .enumerate()
            .map(|(index, (path, status))| Entry {
                path: path.into(),
                status,
                selected: index == 1,
                open: Message::Open(index),
            })
            .collect()
        };
        let content = || {
            container(
                scrollable(view(
                    entries(),
                    &BTreeSet::new(),
                    &i18n,
                    Message::Toggle,
                    536.,
                    0,
                ))
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                }),
            )
            .padding(12)
            .width(560)
            .height(300)
            .style(theme::surface)
        };
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (560., 300.), content());
        assert!(ui.find("tests/unit/apps/api/views").is_ok());
        assert!(
            ui.find("updated.py")
                .unwrap()
                .visible_bounds()
                .unwrap()
                .width
                > 0.
        );
        for badge in ["A", "M", "U"] {
            assert!(ui.find(badge).is_err());
        }
        let mut args = FluentArgs::new();
        args.set("count", 2);
        assert!(ui.find(i18n.format(Key::FileCount, &args).as_str()).is_ok());
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(std::path::PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("changed-tree-{suffix}")))
                .unwrap();
        }
        let file = ui.find("updated.py").unwrap().visible_bounds().unwrap();
        ui.point_at((540., file.center_y()));
        ui.simulate(iced_test::simulator::click());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Message::Open(1)]);
        let collapsed = BTreeSet::from(["tests/unit/apps/api/views".into()]);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (560., 300.),
            view(entries(), &collapsed, &i18n, Message::Toggle, 536., 0),
        );
        assert!(ui.find("updated.py").is_err());
        ui.click("tests/unit/apps/api/views").unwrap();
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            [Message::Toggle("tests/unit/apps/api/views".into())]
        );
    }
}
