//! Keep AppKit material and Iced rendering in separate sibling views.

use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::AnyClass};
use objc2_app_kit::{
    NSApplication, NSGlassEffectView, NSGlassEffectViewStyle, NSView, NSWindowButton,
    NSWindowOrderingMode,
};
use raw_window_handle::RawWindowHandle;

pub(crate) fn apply(window: &dyn iced::window::Window) -> bool {
    configure(window).is_some()
}

fn configure(window: &dyn iced::window::Window) -> Option<()> {
    let mtm = MainThreadMarker::new()?;
    AnyClass::get(c"NSGlassEffectView")?;
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
