//! Own the Windows header menu-bar identifiers and open-menu state.

/// A top-level header menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Menu {
    File,
    View,
    Deployment,
    Help,
}

impl Menu {
    pub(crate) const ALL: [Menu; 4] = [Menu::File, Menu::View, Menu::Deployment, Menu::Help];
}

#[cfg(test)]
mod tests;
