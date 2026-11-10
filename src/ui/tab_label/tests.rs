use super::*;

#[test]
fn filename_limit_preserves_short_names_and_handles_unicode() {
    for count in [34, 35, 36, 100] {
        let filename = "界".repeat(count);
        let displayed = label(&format!("src/{filename}"), None);
        if count <= 35 {
            assert_eq!(displayed, filename);
        } else {
            assert_eq!(displayed, format!("{}…", "界".repeat(34)));
        }
    }
}

#[test]
fn commit_suffix_stays_complete_after_filename_truncation() {
    let filename = "W".repeat(40);
    let displayed = label(&filename, Some("51ad1920123456789"));
    assert_eq!(displayed, format!("{}… · 51ad192", "W".repeat(34)));
    assert!(width(&displayed) > theme::FILE_TAB_MIN_WIDTH);
    assert!(width(&displayed) > width(&label("main.rs", None)));
}
