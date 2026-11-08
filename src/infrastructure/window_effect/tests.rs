use super::*;

#[test]
fn glass_must_precede_content_in_the_same_parent() {
    assert!(background_precedes_content(&[10, 20, 30], &10, &20));
    assert!(!background_precedes_content(&[20, 10, 30], &10, &20));
}

#[test]
fn missing_or_identical_views_do_not_enable_transparency() {
    for views in [&[][..], &[10][..], &[20][..]] {
        assert!(!background_precedes_content(views, &10, &20));
    }
    assert!(!background_precedes_content(&[10], &10, &10));
}
