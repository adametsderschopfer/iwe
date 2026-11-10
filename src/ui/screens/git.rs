use crate::{
    app::{App, Message},
    modules::git,
};
pub(super) fn context(app: &App) -> git::View<'_> {
    use git::actions::bulk::Action;
    git::View {
        state: &app.modules.git,
        i18n: &app.i18n,
        repository: &app.repository,
        repository_error: app.repository_error,
        loading: app.loading,
        preview_revision: &app.preview().revision,
        preview_path: &app.preview().path,
        active_runs: match &app.modules.pipelines.pipelines {
            crate::modules::pipelines::PipelineState::Ready(runs) => runs,
            _ => &[],
        },
        bulk_allowed: [
            app.bulk_allowed(Action::Push),
            app.bulk_allowed(Action::Delete),
            app.bulk_allowed(Action::Pull),
        ],
    }
}
pub(crate) fn view(app: &App) -> iced::Element<'_, Message> {
    git::view(context(app)).map(crate::app::modules::git_message)
}

pub(super) fn activity(app: &App) -> Option<iced::Element<'_, Message>> {
    if !app.module_enabled(crate::modules::ModuleId::Git) {
        return None;
    }
    git::activity(context(app)).map(|activity| activity.map(crate::app::modules::git_message))
}
