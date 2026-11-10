use crate::{
    app::{App, Message},
    modules::git,
};
pub(super) fn overlay<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    if app.modules.git.git_actions.menu.is_none()
        && !app.modules.git.git_actions.has_dialog()
        && app.modules.git.git_actions.bulk.menu.is_none()
    {
        return base;
    }
    iced::widget::stack![
        base,
        git::overlay(
            super::git::context(app),
            iced::widget::space()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        )
        .map(crate::app::modules::git_message)
    ]
    .into()
}
