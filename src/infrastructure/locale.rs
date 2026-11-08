//! Read the application override and native system locale at startup.

use crate::i18n::Language;

pub(crate) fn detect() -> Language {
    let explicit = std::env::var("IWE_LANG").ok();
    let system = sys_locale::get_locale();
    Language::select(explicit.as_deref(), system.as_deref())
}
