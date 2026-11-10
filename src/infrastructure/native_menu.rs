//! Own AppKit menu objects on the event-loop thread and return typed selections.

use muda::{
    Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, Code, Modifiers},
};
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Model {
    pub application: String,
    pub settings: String,
    pub about: String,
    pub deployment: String,
    pub connections: String,
    pub keys: String,
    pub profiles: String,
    pub spaces_title: String,
    pub projects_title: String,
    pub new_space: String,
    pub no_projects: String,
    pub hide: String,
    pub hide_others: String,
    pub show_all: String,
    pub services: String,
    pub quit: String,
    pub enabled: bool,
    pub recording_shortcut: bool,
    pub spaces: Vec<(String, Vec<(String, PathBuf)>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Settings,
    About,
    DeploymentConnections,
    DeploymentKeys,
    DeploymentProfiles,
    NewSpace,
    Project(usize, PathBuf),
    Quit,
}

struct Installed {
    model: Model,
    _menu: Menu,
    actions: BTreeMap<String, Action>,
}

// AppKit menus cannot leave the event-loop thread.
thread_local! {
    static INSTALLED: RefCell<Option<Installed>> = const { RefCell::new(None) };
}

fn item(
    text: &str,
    enabled: bool,
    action: Action,
    accelerator: Option<Accelerator>,
    actions: &mut BTreeMap<String, Action>,
) -> MenuItem {
    let item = MenuItem::new(text.replace('&', "&&"), enabled, accelerator);
    actions.insert(item.id().0.clone(), action);
    item
}

fn install(model: Model) -> muda::Result<Installed> {
    let menu = Menu::new();
    let mut actions = BTreeMap::new();
    let application = Submenu::new(&model.application, true);
    application.append(&item(&model.about, true, Action::About, None, &mut actions))?;
    let settings = Submenu::new(&model.settings, model.enabled);
    settings.append(&item(
        &model.settings,
        model.enabled,
        Action::Settings,
        (!model.recording_shortcut).then(|| Accelerator::new(Modifiers::META, Code::Comma)),
        &mut actions,
    ))?;
    application.append(&PredefinedMenuItem::separator())?;
    let spaces = Submenu::new(&model.spaces_title, model.enabled);
    spaces.append(&item(
        &model.new_space,
        model.enabled,
        Action::NewSpace,
        None,
        &mut actions,
    ))?;
    let projects = Submenu::new(&model.projects_title, model.enabled);
    if model.spaces.iter().all(|(_, entries)| entries.is_empty()) {
        projects.append(&MenuItem::new(&model.no_projects, false, None))?;
    }
    for (index, (name, entries)) in model.spaces.iter().enumerate() {
        if !entries.is_empty() {
            let group = Submenu::new(name.replace('&', "&&"), true);
            for (name, path) in entries {
                group.append(&item(
                    name,
                    true,
                    Action::Project(index, path.clone()),
                    None,
                    &mut actions,
                ))?;
            }
            projects.append(&group)?;
        }
    }
    application.append(&PredefinedMenuItem::services(Some(&model.services)))?;
    application.append(&PredefinedMenuItem::separator())?;
    if model.recording_shortcut {
        application.append(&MenuItem::new(&model.hide, false, None))?;
        application.append(&MenuItem::new(&model.hide_others, false, None))?;
    } else {
        application.append(&PredefinedMenuItem::hide(Some(&model.hide)))?;
        application.append(&PredefinedMenuItem::hide_others(Some(&model.hide_others)))?;
    }
    application.append(&PredefinedMenuItem::show_all(Some(&model.show_all)))?;
    application.append(&PredefinedMenuItem::separator())?;
    application.append(&item(
        &model.quit,
        true,
        Action::Quit,
        (!model.recording_shortcut).then(|| Accelerator::new(Modifiers::META, Code::KeyQ)),
        &mut actions,
    ))?;
    menu.append(&application)?;
    menu.append(&spaces)?;
    menu.append(&projects)?;
    menu.append(&settings)?;
    let deployment = Submenu::new(&model.deployment, model.enabled);
    for (label, action) in [
        (&model.connections, Action::DeploymentConnections),
        (&model.keys, Action::DeploymentKeys),
        (&model.profiles, Action::DeploymentProfiles),
    ] {
        deployment.append(&item(label, model.enabled, action, None, &mut actions))?;
    }
    menu.append(&deployment)?;
    menu.init_for_nsapp();
    Ok(Installed {
        model,
        _menu: menu,
        actions,
    })
}

pub(crate) fn sync_and_poll(model: Model) -> Result<Vec<Action>, ()> {
    objc2::MainThreadMarker::new().ok_or(())?;
    INSTALLED.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot
            .as_ref()
            .is_none_or(|installed| installed.model != model)
        {
            *slot = Some(install(model).map_err(|_| ())?);
        }
        let events = MenuEvent::receiver()
            .try_iter()
            .filter_map(|event| slot.as_ref()?.actions.get(event.id.as_ref()).cloned())
            .collect();
        Ok(events)
    })
}
