//! Place native glass behind the renderer without changing its content view.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub(crate) use macos::apply;

#[cfg(not(target_os = "macos"))]
pub(crate) fn apply(_window: &dyn iced::window::Window) -> bool {
    false
}

#[cfg(any(target_os = "macos", test))]
fn background_precedes_content<T: PartialEq>(views: &[T], background: &T, content: &T) -> bool {
    matches!(
        (
            views.iter().position(|view| view == background),
            views.iter().position(|view| view == content),
        ),
        (Some(background), Some(content)) if background < content
    )
}

#[cfg(test)]
mod tests;
