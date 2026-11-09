//! Show operation results above the workspace without changing its layout.

use crate::ui::tooltip;
use crate::{
    app::notifications::{Event, Kind, State},
    i18n::{I18n, Key},
    ui::{
        ellipsis,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, keyed_column, mouse_area, opaque, progress_bar, row, stack, text},
};

pub(crate) fn overlay<'a, Message: Clone + 'a>(
    state: &'a State,
    i18n: &'a I18n,
    base: Element<'a, Message>,
    on_event: fn(Event) -> Message,
) -> Element<'a, Message> {
    if state.items.is_empty() {
        return base;
    }
    let mut items = Vec::new();
    for item in &state.items {
        let (symbol, tone) = match item.kind {
            Kind::Success => (Icon::Check, theme::GREEN),
            Kind::Error => (Icon::Warning, theme::RED),
        };
        let mut content = column![text(i18n.text(item.message)).size(13)].spacing(6);
        if let Some(context) = &item.context {
            content = content.push(tooltip::view(
                container(ellipsis::text(context.as_str(), 12.)).style(|theme| container::Style {
                    text_color: Some(theme::MUTED.resolve(theme)),
                    ..Default::default()
                }),
                text(context).size(12),
                tooltip::Position::Top,
            ));
        }
        let close = tooltip::view(
            container(
                button(icon(Icon::Close, theme::MUTED))
                    .padding(3)
                    .style(theme::action(false))
                    .on_press(on_event(Event::Dismiss(item.id))),
            )
            .id(format!("notification-close-{}", item.id)),
            text(i18n.text(Key::Dismiss)).size(12),
            tooltip::Position::Top,
        );
        items.push((item.id,
            mouse_area(container(
                column![
                row![icon(symbol, tone), content.width(Length::Fill), close]
                    .spacing(10)
                    .align_y(Alignment::Start),
                progress_bar(0.0..=1.0, state.remaining(item))
                    .girth(3)
                    .style(move |theme| theme::notification_progress(theme, tone)),
                ].spacing(10),
            )
            .id(format!("notification-{}", item.id))
            .padding(theme::NOTIFICATION_PADDING)
            .width(Length::Fill)
            .style(theme::notification))
            .on_enter(on_event(Event::Hover(item.id, true)))
            .on_exit(on_event(Event::Hover(item.id, false))).into(),
        ));
    }
    let items = container(opaque(keyed_column(items).spacing(theme::NOTIFICATION_GAP)))
    .id("notifications")
    .width(theme::NOTIFICATION_WIDTH);
    stack![
        base,
        container(items)
            .padding(theme::NOTIFICATION_INSET)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
    ]
    .into()
}
