use super::*;
use crate::{
    domain::{
        jobs::{Job, JobAction, Stage},
        repository::{Pipeline, Remote, Repository},
    },
    i18n::{I18n, Language},
};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace.preferences.show_pipelines = true;
    app.repository = Some(
        Repository {
            root: "/fixture".into(),
            head: "main".into(),
            branches: Vec::new(),
            commits: Vec::new(),
            next_history: None,
            remotes: vec!["origin".into()],
            remote: Remote::parse("https://github.com/example/fixture.git"),
        }
        .into(),
    );
    app.pipelines = PipelineState::Ready(
        (1..=8)
            .map(|id| Pipeline {
                id,
                number: Some(id),
                title: format!("Run {id}"),
                branch: "main".into(),
                commit: None,
                status: "running".into(),
                date: String::new(),
                url: format!("https://github.com/example/fixture/actions/runs/{id}"),
            })
            .collect(),
    );
    app
}

fn details() -> Details {
    Details {
        stages: vec![Stage {
            name: None,
            provider_status: None,
            jobs: vec![Job {
                id: 91,
                name: "Test / Linux".into(),
                status: "success".into(),
                url: String::new(),
                actions: vec![JobAction::Retry],
            }],
        }],
        cancelable: false,
    }
}

fn finish(app: &mut App, id: u64, result: Result<Details, Failure>) {
    let request = app.jobs.overview.entries[&id].loading.unwrap();
    let _ = app.update_pipeline_overview(Event::Loaded(app.jobs.epoch, request, id, result));
}

#[test]
fn visible_summaries_limit_concurrency_and_reject_previous_project_results() {
    let mut app = app();
    let _ = app.update_pipeline_overview(Event::Viewport(0..5));
    assert_eq!(
        app.jobs
            .overview
            .entries
            .values()
            .filter(|entry| entry.is_loading())
            .count(),
        2
    );
    assert!(!app.jobs.overview.entries.contains_key(&6));
    let request = app.jobs.overview.request;
    let _ = app.refresh_pipeline_overviews(false);
    let _ = app.update_pipeline_overview(Event::Refresh(1));
    assert_eq!(app.jobs.overview.request, request);
    finish(&mut app, 1, Ok(details()));
    assert!(app.jobs.overview.entries[&3].is_loading());
    let _ = app.update_pipeline_overview(Event::Viewport(6..8));
    finish(&mut app, 2, Ok(details()));
    assert!(app.jobs.overview.entries[&7].is_loading());
    assert!(!app.jobs.overview.entries[&4].is_loading());
    let epoch = app.jobs.epoch;
    let request = app.jobs.overview.entries[&7].loading.unwrap();
    app.reset_jobs();
    let _ = app.update_pipeline_overview(Event::Loaded(epoch, request, 7, Ok(details())));
    assert!(app.jobs.overview.entries.is_empty());
}

#[test]
fn overview_errors_preserve_statuses_and_require_explicit_retry() {
    let mut app = app();
    let _ = app.update_pipeline_overview(Event::Viewport(0..1));
    finish(&mut app, 1, Ok(details()));
    let _ = app.refresh_pipeline_overviews(true);
    assert!(app.jobs.overview.entries[&1].details.is_some());
    finish(&mut app, 1, Err(Failure::Unavailable));
    let request = app.jobs.overview.request;
    let _ = app.refresh_pipeline_overviews(true);
    assert_eq!(app.jobs.overview.request, request);
    assert!(app.jobs.overview.entries[&1].details.is_some());
    assert!(app.job_controls_blocked(1));
    let _ = app.update_pipeline_overview(Event::Refresh(1));
    assert!(app.jobs.overview.entries[&1].is_loading());
    finish(&mut app, 1, Ok(details()));
    assert!(!app.job_controls_blocked(1));
}

#[test]
fn completed_summaries_reuse_cache_until_status_changes_or_manual_refresh() {
    let mut app = app();
    if let PipelineState::Ready(runs) = &mut app.pipelines {
        runs[0].status = "success".into();
    }
    let _ = app.update_pipeline_overview(Event::Viewport(0..1));
    finish(&mut app, 1, Ok(details()));
    let request = app.jobs.overview.request;
    let _ = app.refresh_pipeline_overviews(true);
    assert_eq!(app.jobs.overview.request, request);
    if let PipelineState::Ready(runs) = &mut app.pipelines {
        runs[0].status = "running".into();
    }
    let _ = app.refresh_pipeline_overviews(false);
    assert!(app.jobs.overview.entries[&1].is_loading());
    finish(&mut app, 1, Ok(details()));
    let _ = app.update_pipeline_overview(Event::Open(1, Target::Job(91), Point::new(100., 100.)));
    assert!(app.jobs.overview.popup.is_some());
    let _ = app.update_pipeline_overview(Event::Refresh(1));
    finish(&mut app, 1, Ok(Details::default()));
    assert!(app.jobs.overview.popup.is_none());
}

#[test]
fn summaries_stop_loading_when_hidden_and_only_allow_current_cached_job_actions() {
    let mut app = app();
    app.workspace.preferences.show_pipelines = false;
    let _ = app.refresh_pipeline_overviews(false);
    assert!(app.jobs.overview.entries.is_empty());
    app.workspace.preferences.show_pipelines = true;
    let _ = app.update_pipeline_overview(Event::Viewport(0..1));
    finish(&mut app, 1, Ok(details()));
    let _ = app.update_jobs(JobsEvent::Control(
        1,
        crate::domain::jobs::Action::Job(91, JobAction::Cancel),
    ));
    assert!(app.jobs.pending.is_none());
    let _ = app.update_jobs(JobsEvent::Control(
        1,
        crate::domain::jobs::Action::Job(91, JobAction::Retry),
    ));
    assert!(app.jobs.pending.is_some());
    let request = app.jobs.action_request;
    let _ = app.update_jobs(JobsEvent::Control(
        1,
        crate::domain::jobs::Action::Job(91, JobAction::Retry),
    ));
    assert_eq!(app.jobs.action_request, request);
    app.jobs.pending = None;
    app.pipelines = PipelineState::Ready(Vec::new());
    assert!(app.job_details(1).is_none());
}
