#[test]
fn embedded_icon_decodes_with_transparency() {
    let icon = super::window_icon().expect("embedded RGBA icon");
    let (pixels, size) = icon.into_raw();
    assert_eq!((size.width, size.height), (128, 128));
    assert_eq!(pixels.len(), 128 * 128 * 4);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 0));
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 255));
}
