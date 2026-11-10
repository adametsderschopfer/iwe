#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod domain;
mod i18n;
mod infrastructure;
mod modules;
mod ui;

fn main() -> std::process::ExitCode {
    if let Some(code) = infrastructure::updates::startup() {
        return code;
    }
    let language = infrastructure::locale::detect();
    let i18n = std::rc::Rc::new(i18n::I18n::new(language));
    infrastructure::graphics::run(&i18n, || app::run(std::rc::Rc::clone(&i18n)))
}
