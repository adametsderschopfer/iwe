//! Share source-tab labels and measured widths with tab scrolling.

use super::{ellipsis, theme};
use std::path::Path;

pub(crate) fn label(title: &str, revision: Option<&str>) -> String {
    let filename = Path::new(title)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(title);
    let filename = if filename.chars().count() > theme::FILE_TAB_NAME_LIMIT {
        format!(
            "{}…",
            filename
                .chars()
                .take(theme::FILE_TAB_NAME_LIMIT - 1)
                .collect::<String>()
        )
    } else {
        filename.to_owned()
    };
    if let Some(revision) = revision {
        format!(
            "{filename} · {}",
            revision.chars().take(7).collect::<String>()
        )
    } else {
        filename
    }
}

pub(crate) fn width(label: &str) -> f32 {
    let text = ellipsis::measured_width(label, theme::FILE_TAB_TEXT_SIZE).ceil();
    (text
        + theme::FILE_TAB_ICON_WIDTH
        + theme::FILE_TAB_LABEL_GAP
        + theme::FILE_TAB_PADDING * 2.
        + theme::FILE_TAB_CLOSE_WIDTH
        + theme::FILE_TAB_END_GAP
        + 2.)
        .max(theme::FILE_TAB_MIN_WIDTH)
}

#[cfg(test)]
mod tests;
