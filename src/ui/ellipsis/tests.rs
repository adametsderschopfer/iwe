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
