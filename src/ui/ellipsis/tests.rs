use super::*;

#[test]
fn long_titles_fit_one_line_at_different_widths() {
    for title in [
        "Исправлен пересчёт размеров интерфейса при изменении ширины окна",
        "WWWW WWWW WWWW WWWW WWWW",
        "修正窗口布局和文件目录显示",
    ] {
        let mut state = State::default();
        for width in [1., 70., 130., 250., 800.] {
            state.update(title, width, 13.);
            assert!(state.paragraph.min_width() <= width + 0.1);
            assert!(state.paragraph.min_height() <= 13. * 1.3 + 0.1);
            if paragraph(title, 13.).min_width() > width && width > 20. {
                assert!(state.displayed.ends_with('…'));
            }
        }
        assert_eq!(state.displayed, title);
    }
}

#[test]
fn compact_labels_fit_their_text_and_truncate_long_names() {
    use iced::{
        Settings,
        widget::{Id, container},
    };
    for name in [
        "front",
        "mz-site-front",
        &"long-repository-name-".repeat(20),
    ] {
        let view: Element<'_, ()> = container(compact_text(name, 10.))
            .id("label")
            .padding([2, 5])
            .max_width(160)
            .into();
        let mut ui = iced_test::Simulator::with_size(Settings::default(), (300., 100.), view);
        let bounds = ui.find(Id::new("label")).unwrap().visible_bounds().unwrap();
        assert!(bounds.width <= 160.);
        let natural = paragraph(name, 10.).min_width() + 10.;
        if natural <= 160. {
            assert!((bounds.width - natural).abs() < 0.1);
            assert!(ui.find(name).is_ok());
        } else {
            assert!(ui.find(name).is_err());
        }
    }
}
