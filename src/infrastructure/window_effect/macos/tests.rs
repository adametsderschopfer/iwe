use super::*;

#[test]
fn resizing_renderer_layers_preserves_pixel_scale_and_glass_siblings() {
    let parent = CALayer::new();
    let glass = CALayer::new();
    let content = CALayer::new();
    let drawable = CALayer::new();
    parent.addSublayer(&glass);
    parent.addSublayer(&content);
    content.addSublayer(&drawable);
    drawable.setContentsScale(2.);
    let mut frame = drawable.frame();
    frame.size.width = 1440.;
    frame.size.height = 920.;
    drawable.setFrame(frame);
    assert_eq!(&*drawable.contentsGravity(), ns_string!("resize"));

    anchor_renderer_layers(&content);
    for (width, height) in [(1080., 720.), (1440., 920.), (1800., 1000.), (1080., 720.)] {
        frame.size.width = width;
        frame.size.height = height;
        drawable.setFrame(frame);
        assert_eq!(&*drawable.contentsGravity(), ns_string!("topLeft"));
        assert_eq!(drawable.contentsScale(), 2.);
        assert_eq!(drawable.frame().size, frame.size);
    }
    assert_eq!(&*glass.contentsGravity(), ns_string!("resize"));
    assert_eq!(child_layers(&parent), vec![glass, content]);
}

#[test]
fn anchoring_does_not_replace_existing_layer_geometry_or_scaling() {
    let content = CALayer::new();
    let drawable = CALayer::new();
    content.addSublayer(&drawable);
    drawable.setGeometryFlipped(true);
    drawable.setContentsScale(1.5);
    let geometry = drawable.isGeometryFlipped();
    let flipped = drawable.contentsAreFlipped();
    anchor_renderer_layers(&content);
    let gravity = drawable.contentsGravity();
    assert_eq!(
        &*gravity,
        if flipped {
            ns_string!("bottomLeft")
        } else {
            ns_string!("topLeft")
        }
    );
    assert_eq!(drawable.isGeometryFlipped(), geometry);
    assert_eq!(drawable.contentsScale(), 1.5);
    anchor_renderer_layers(&content);
    assert_eq!(drawable.contentsGravity(), gravity);
}
