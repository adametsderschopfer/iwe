//! Render the Windows header menu bar and its anchored dropdown menus.

use crate::{
    app::{App, Message, Navigation, SettingsTab, menu_bar::Menu},
    i18n::Key,
    modules::ModuleId,
    ui::{context_menu, theme},
};
use iced::{
    Element, Length, Size,
    widget::{Column, Row, button, container, space, text},
};

pub(super) fn header_prefix(app: &App) -> Element<'_, Message> {
    let deployment_enabled = app.module_enabled(ModuleId::Deployment);
    let mut titles = Row::new().spacing(2);
    for menu in Menu::ALL {
        if menu == Menu::Deployment && !deployment_enabled {
            continue;
        }
        let label = app.i18n.text(match menu {
            Menu::File => Key::MenuFile,
            Menu::View => Key::MenuView,
            Menu::Deployment => Key::Deployment,
            Menu::Help => Key::MenuHelp,
        });
        let open = app.menu_bar.is_some_and(|(open, _)| open == menu);
        let title = container(text(label).size(13))
            .padding([8, 10])
            .style(theme::menu_title(open))
            .id(match menu {
                Menu::File => "menu-bar-file",
                Menu::View => "menu-bar-view",
                Menu::Deployment => "menu-bar-deployment",
                Menu::Help => "menu-bar-help",
            });
        titles = titles.push(context_menu::anchored(title, move |point| {
            Message::OpenMenuBar(menu, point)
        }));
    }
    titles.into()
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some((menu, point)) = app.menu_bar else {
        return base;
    };
    let size = match menu {
        Menu::File => Size::new(260., 250.),
        Menu::View => Size::new(220., 170.),
        Menu::Deployment => Size::new(220., 130.),
        Menu::Help => Size::new(220., 80.),
    };
    context_menu::overlay(
        base,
        point,
        size,
        move || items(app, menu),
        Message::CloseMenuBar,
    )
}

fn items(app: &App, menu: Menu) -> Element<'_, Message> {
    match menu {
        Menu::File => file_items(app),
        Menu::View => view_items(app),
        Menu::Deployment => deployment_items(app),
        Menu::Help => help_items(app),
    }
}

fn file_items(app: &App) -> Element<'_, Message> {
    let enabled = app.menu_actions_enabled();
    column![
        item(
            "menu-item-new-space",
            app.i18n.text(Key::NewSpace),
            enabled.then(|| Message::Space(crate::app::spaces::Message::New)),
        ),
        item(
            "menu-item-add-project",
            app.i18n.text(Key::AddProject),
            enabled.then(|| Message::ProjectAdd(crate::app::project_add::Message::Open(None))),
        ),
        separator(),
        item(
            "menu-item-settings",
            app.i18n.text(Key::Settings),
            enabled.then(|| Message::Navigate(Navigation::Settings)),
        ),
        separator(),
        item(
            "menu-item-exit",
            app.i18n.text(Key::MenuExit),
            app.main_window_close(),
        ),
    ]
    .spacing(4)
    .into()
}

fn view_items(app: &App) -> Element<'_, Message> {
    let mut items = Column::new().spacing(4);
    let mut available = 0;
    for (id, label, widget_id) in [
        (ModuleId::Terminal, Key::Terminal, "menu-item-terminal"),
        (ModuleId::Git, Key::Git, "menu-item-git"),
        (ModuleId::Pipelines, Key::Pipelines, "menu-item-pipelines"),
    ] {
        if !app.panel_available(id) {
            continue;
        }
        available += 1;
        items = items.push(item(
            widget_id,
            app.i18n.text(label),
            Some(Message::Module(crate::app::modules::Event::Toggle(id))),
        ));
    }
    if available == 0 {
        items = items.push(item(
            "menu-item-no-panels",
            app.i18n.text(Key::NoProjects),
            None,
        ));
    }
    items.into()
}

fn deployment_items(app: &App) -> Element<'_, Message> {
    let enabled = app.menu_actions_enabled();
    column![
        item(
            "menu-item-connections",
            app.i18n.text(Key::DeploymentConnections),
            enabled.then(|| Message::SelectSettingsTab(SettingsTab::Connections)),
        ),
        item(
            "menu-item-servers",
            app.i18n.text(Key::DeploymentProfiles),
            enabled.then(|| Message::SelectSettingsTab(SettingsTab::Deployment)),
        ),
    ]
    .spacing(4)
    .into()
}

fn help_items(app: &App) -> Element<'_, Message> {
    item(
        "menu-item-about",
        app.i18n.text(Key::About),
        Some(Message::MenuAbout),
    )
}

fn item<'a>(
    widget_id: &'static str,
    label: String,
    message: Option<Message>,
) -> Element<'a, Message> {
    container(
        button(text(label).size(14).wrapping(text::Wrapping::None))
            .on_press_maybe(message)
            .padding([10, 12])
            .width(Length::Fill)
            .style(theme::action(false)),
    )
    .id(widget_id)
    .into()
}

fn separator() -> Element<'static, Message> {
    container(space())
        .height(1)
        .width(Length::Fill)
        .style(theme::separator)
        .into()
}
