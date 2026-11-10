use crate::{
    app::{App, Message, Navigation},
    modules::pipelines,
};
fn context(app: &App) -> pipelines::View<'_> {
    pipelines::View {
        state: &app.modules.pipelines,
        i18n: &app.i18n,
        remote: app
            .active_repository()
            .and_then(|repo| repo.remote.as_ref()),
        account_username: &app.account_username,
        disconnecting: app.disconnecting,
        visible: app.navigation != Navigation::Settings && app.workspace.preferences.show_pipelines,
    }
}
pub(crate) fn view(app: &App) -> iced::Element<'_, Message> {
    pipelines::view(context(app)).map(crate::app::modules::pipelines_message)
}
pub(super) fn overlay<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    if app.modules.pipelines.jobs.overview.popup.is_none() {
        return base;
    }
    iced::widget::stack![
        base,
        pipelines::overlay(
            context(app),
            iced::widget::space()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        )
        .map(crate::app::modules::pipelines_message)
    ]
    .into()
}
