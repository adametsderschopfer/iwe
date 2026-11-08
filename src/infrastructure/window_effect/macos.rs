//! Keep AppKit material and Iced rendering in separate sibling views.

use objc2::{
    MainThreadMarker, MainThreadOnly,
    rc::Retained,
    runtime::{AnyClass, AnyObject},
};
use objc2_app_kit::{
    NSApplication, NSGlassEffectView, NSGlassEffectViewStyle, NSView, NSViewLayerContentsPlacement,
    NSViewLayerContentsRedrawPolicy, NSWindowButton, NSWindowOrderingMode,
};
use objc2_foundation::{NSArray, NSObjectNSKeyValueCoding, ns_string};
use objc2_quartz_core::CALayer;
use raw_window_handle::RawWindowHandle;

pub(crate) fn apply(window: &dyn iced::window::Window) -> bool {
    configure(window).is_some()
}

fn configure(window: &dyn iced::window::Window) -> Option<()> {
    let mtm = MainThreadMarker::new()?;
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };

    let application = NSApplication::sharedApplication(mtm);
    let (native_window, content) =
        application
            .windows()
            .to_vec()
            .into_iter()
            .find_map(|native_window| {
                let content = native_window.contentView()?;
                (Retained::as_ptr(&content).cast() == handle.ns_view.as_ptr())
                    .then_some((native_window, content))
            })?;

    // Preserve pixel size while the renderer prepares a frame for the new window bounds.
    content.setLayerContentsPlacement(NSViewLayerContentsPlacement::TopLeft);
    content.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
    if let Some(layer) = content.layer() {
        anchor_renderer_layers(&layer);
    }

    // Resize behavior also applies on macOS versions without Liquid Glass.
    AnyClass::get(c"NSGlassEffectView")?;

    // Find the shared parent through retained views, without dereferencing the raw window handle.
    let close_button = native_window.standardWindowButton(NSWindowButton::CloseButton)?;
    let parent = content.ancestorSharedWithView(&close_button)?;
    if !parent.subviews().to_vec().contains(&content) {
        return None;
    }

    let glass = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), content.frame());
    glass.setStyle(NSGlassEffectViewStyle::Regular);
    glass.setAutoresizingMask(content.autoresizingMask());
    parent.addSubview_positioned_relativeTo(&glass, NSWindowOrderingMode::Below, Some(&content));

    let siblings: Vec<_> = parent
        .subviews()
        .to_vec()
        .iter()
        .map(Retained::as_ptr)
        .collect();
    let glass_view: &NSView = &glass;
    if !super::background_precedes_content(
        &siblings,
        &(glass_view as *const NSView),
        &Retained::as_ptr(&content),
    ) {
        glass.removeFromSuperview();
        return None;
    }

    Some(())
}

fn anchor_renderer_layers(layer: &CALayer) {
    // Core Animation's default "resize" gravity stretches the previous drawable between frames.
    layer.setContentsGravity(if layer.contentsAreFlipped() {
        ns_string!("bottomLeft")
    } else {
        ns_string!("topLeft")
    });
    for child in child_layers(layer) {
        anchor_renderer_layers(&child);
    }
}

fn child_layers(layer: &CALayer) -> Vec<Retained<CALayer>> {
    // Check dynamic collection types instead of using the unchecked native sublayers binding.
    layer
        .valueForKey(ns_string!("sublayers"))
        .and_then(|value| value.downcast::<NSArray<AnyObject>>().ok())
        .map(|children| children.to_vec())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|child| child.downcast::<CALayer>().ok())
        .collect()
}

#[cfg(test)]
mod tests;
