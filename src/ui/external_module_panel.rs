//! Render declarative panels described by external modules.

use crate::{
    app::modules::external,
    domain::modules::{ButtonStyle, TextStyle, WidgetAction, WidgetNode},
    ui::theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{
        Checkbox, button, column, container, progress_bar, row, scrollable, space, text,
        text_input, toggler,
    },
};

fn message(module: &str, widget: &str, action: WidgetAction) -> crate::app::Message {
    crate::app::Message::Module(crate::app::modules::Event::External(
        external::Event::Widget(module.to_owned(), widget.to_owned(), action),
    ))
}

/// Render one widget tree with the shared application theme.
pub(crate) fn render<'a>(
    module: &'a str,
    node: &'a WidgetNode,
) -> Element<'a, crate::app::Message> {
    let content: Element<'_, crate::app::Message> = match node {
        WidgetNode::Column { children, spacing } => column(
            children
                .iter()
                .map(|child| render(module, child))
                .collect::<Vec<_>>(),
        )
        .spacing(f32::from(*spacing))
        .into(),
        WidgetNode::Row { children, spacing } => row(children
            .iter()
            .map(|child| render(module, child))
            .collect::<Vec<_>>())
        .spacing(f32::from(*spacing))
        .into(),
        WidgetNode::Text { content, style } => match style {
            TextStyle::Heading => text(content)
                .size(16)
                .font(theme::semibold())
                .shaping(text::Shaping::Advanced)
                .into(),
            TextStyle::Body => text(content)
                .size(13)
                .shaping(text::Shaping::Advanced)
                .into(),
            TextStyle::Caption => text(content)
                .size(12)
                .shaping(text::Shaping::Advanced)
                .into(),
            TextStyle::Muted => text(content)
                .size(13)
                .style(theme::MUTED.text())
                .shaping(text::Shaping::Advanced)
                .into(),
            TextStyle::Danger => text(content)
                .size(13)
                .style(theme::RED.text())
                .shaping(text::Shaping::Advanced)
                .into(),
        },
        WidgetNode::Button {
            id,
            label,
            style,
            disabled,
        } => {
            let widget = id.clone();
            let module_id = module.to_owned();
            button(text(label).size(13))
                .on_press_maybe(
                    (!disabled).then(|| message(&module_id, &widget, WidgetAction::Click)),
                )
                .padding([8, 14])
                .style(match style {
                    ButtonStyle::Primary => {
                        theme::primary as fn(&iced::Theme, button::Status) -> button::Style
                    }
                    ButtonStyle::Secondary | ButtonStyle::Danger => {
                        theme::secondary as fn(&iced::Theme, button::Status) -> button::Style
                    }
                })
                .into()
        }
        WidgetNode::TextInput {
            id,
            value,
            placeholder,
            disabled,
        } => {
            let widget = id.clone();
            let module_id = module.to_owned();
            text_input(placeholder, value)
                .on_input_maybe((!disabled).then_some(move |input: String| {
                    message(&module_id, &widget, WidgetAction::Change { value: input })
                }))
                .size(13)
                .style(theme::input)
                .into()
        }
        WidgetNode::Checkbox { id, label, checked } => {
            let widget = id.clone();
            let module_id = module.to_owned();
            Checkbox::new(*checked)
                .label(label.clone())
                .on_toggle(move |value: bool| {
                    message(&module_id, &widget, WidgetAction::Toggle { value })
                })
                .text_size(13)
                .into()
        }
        WidgetNode::Toggle {
            id,
            checked,
            disabled,
        } => {
            let widget = id.clone();
            let module_id = module.to_owned();
            container(
                toggler(*checked)
                    .on_toggle_maybe((!disabled).then_some(move |value: bool| {
                        message(&module_id, &widget, WidgetAction::Toggle { value })
                    }))
                    .style(theme::toggle),
            )
            .into()
        }
        WidgetNode::Progress { value } => container(progress_bar(0.0..=1.0, *value).girth(6))
            .width(Length::Fill)
            .into(),
        WidgetNode::Separator => container(space())
            .height(1)
            .width(Length::Fill)
            .style(theme::separator)
            .into(),
        WidgetNode::Scroll { child } => scrollable(render(module, child))
            .style(theme::scroll)
            .height(Length::Fill)
            .into(),
        WidgetNode::Spacer => space().into(),
    };
    content
}

/// Zone panel with a tab row when several modules share one zone.
pub(crate) fn zone_panel<'a>(
    app: &'a crate::app::App,
    zone: crate::domain::modules::Placement,
) -> Element<'a, crate::app::Message> {
    let modules = app.external_zone_modules(zone);
    let mut tabs = row![].spacing(4).align_y(Alignment::Center);
    let mut active_panel: Option<(&'a str, Option<&'a WidgetNode>)> = None;
    let active_id = app.modules.external.active.get(&zone);
    for module in &modules {
        let selected = active_id == Some(&module.manifest.id);
        let name = module
            .manifest
            .name
            .resolve(app.i18n.language().code())
            .to_owned();
        let select = crate::app::Message::Module(crate::app::modules::Event::External(
            external::Event::SelectZone(zone, module.manifest.id.clone()),
        ));
        tabs = tabs.push(
            button(text(name).size(12).style(if selected {
                theme::TEXT.text()
            } else {
                theme::MUTED.text()
            }))
            .on_press(select)
            .padding([4, 10])
            .style(theme::action(selected)),
        );
        let panel = app
            .modules
            .external
            .sessions
            .get(&module.manifest.id)
            .and_then(|state| state.panel.as_ref());
        if selected || active_panel.is_none() {
            active_panel = Some((module.manifest.id.as_str(), panel));
        }
    }
    let body = match active_panel {
        Some((id, Some(panel))) => render(id, panel),
        _ => container(space()).into(),
    };
    column![
        container(tabs).padding([6, 12]).width(Length::Fill),
        container(space())
            .height(1)
            .width(Length::Fill)
            .style(theme::separator),
        container(body).padding(12).width(Length::Fill),
    ]
    .width(Length::Fill)
    .into()
}

/// Compact header strip for modules placed in the workspace header.
pub(crate) fn header_panel<'a>(app: &'a crate::app::App) -> Element<'a, crate::app::Message> {
    let modules = app.external_zone_modules(crate::domain::modules::Placement::Header);
    let mut strip = row![].spacing(12).align_y(Alignment::Center);
    for module in &modules {
        if let Some(state) = app.modules.external.sessions.get(&module.manifest.id)
            && let Some(panel) = &state.panel
        {
            strip = strip.push(render(&module.manifest.id, panel));
        }
    }
    container(strip).into()
}
