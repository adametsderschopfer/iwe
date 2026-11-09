use super::*;
use crate::{
    app::Panel,
    domain::{
        jobs::{Job, JobAction, Stage},
        repository::{Provider, Remote, Repository},
    },
    i18n::{I18n, Key, Language},
    ui::theme,
};
use iced::{
    Theme,
    widget::{Id, pane_grid},
};
use std::{path::PathBuf, rc::Rc};

fn run(id: u64) -> Pipeline {
    Pipeline {
        id,
        number: Some(7),
        title: "Build application and publish preview".into(),
        branch: "main".into(),
        commit: None,
        status: "running".into(),
        date: "2026-11-09".into(),
        url: format!("https://gitlab.example.test/team/app/-/pipelines/{id}"),
    }
}

fn details() -> Details {
    Details {
        stages: [
            ("build", "compile", "success", JobAction::Retry),
            ("test", "unit-tests", "running", JobAction::Cancel),
            ("deploy", "deploy-preview", "manual", JobAction::Play),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (stage, name, status, action))| Stage {
            name: Some(stage.into()),
            provider_status: None,
            jobs: vec![Job {
                id: index as u64 + 1,
                name: name.into(),
                status: status.into(),
                url: format!("https://gitlab.example.test/team/app/-/jobs/{}", index + 1),
                actions: vec![action],
            }],
        })
        .collect(),
        cancelable: true,
    }
}

fn app(language: Language) -> App {
    let mut app = App::new(Rc::new(I18n::new(language)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec![PathBuf::from("/project")]);
    app.project_directory = app.workspace.directory().cloned();
    app.repository = Some(Repository {
        remotes: vec!["origin".to_owned()],
        root: "/project".into(),
        branches: Vec::new(),
        commits: Vec::new(),
        next_history: None,
        remote: Remote::parse("https://gitlab.example.test/team/app.git"),
        head: "main".into(),
    });
    app.pipelines = PipelineState::Ready(vec![run(500), run(501)]);
    app.workspace.preferences.show_git = false;
    app.workspace.preferences.show_pipelines = true;
    app.panes = pane_grid::State::new(Panel::Pipelines).0;
    app.account_username = Some("developer".into());
    app
}

fn loaded(app: &mut App) {
    let id = app.jobs.selected.as_ref().unwrap().id;
    let _ = app.update_jobs(Event::Loaded(
        app.jobs.epoch,
        app.jobs.request,
        id,
        Ok(details()),
    ));
}

fn selected() -> App {
    let mut app = app(Language::English);
    let _ = app.update_jobs(Event::Select(500));
    loaded(&mut app);
    app
}

#[test]
fn job_action_notifications_wait_for_provider_acceptance_and_ignore_duplicate_results() {
    for (action, key) in [
        (Action::Job(3, JobAction::Play), Key::JobRunRequested),
        (Action::Job(1, JobAction::Retry), Key::JobRetryRequested),
        (Action::Job(2, JobAction::Cancel), Key::JobStopRequested),
        (Action::CancelPipeline, Key::PipelineStopRequested),
    ] {
        let mut app = selected();
        let _ = app.update_jobs(Event::Control(500, action));
        assert!(app.notifications.items.is_empty());
        let event = Event::Controlled(app.jobs.epoch, app.jobs.action_request, 500, Ok(()));
        let _ = app.update_jobs(event.clone());
        assert_eq!(app.notifications.items.len(), 1);
        assert_eq!(app.notifications.items[0].message, key);
        let _ = app.update_jobs(event);
        assert_eq!(app.notifications.items.len(), 1);
    }
    let mut app = selected();
    let _ = app.update_jobs(Event::Control(500, Action::Job(3, JobAction::Play)));
    let event = Event::Controlled(app.jobs.epoch, app.jobs.action_request, 500, Ok(()));
    app.reset_jobs();
    let _ = app.update_jobs(event);
    assert!(app.notifications.items.is_empty());
}

#[test]
fn job_refresh_keeps_details_and_collapse_state_and_rejects_old_selection_results() {
    let mut app = selected();
    let _ = app.update_jobs(Event::ToggleStage("build".into()));
    let _ = app.update_jobs(Event::Refresh);
    assert!(app.jobs.loading);
    assert_eq!(app.jobs.details.as_ref().unwrap().stages.len(), 3);
    let request = app.jobs.request;
    let _ = app.update_jobs(Event::Tick);
    assert_eq!(app.jobs.request, request);
    let _ = app.update_jobs(Event::Loaded(
        app.jobs.epoch,
        request,
        500,
        Err(Failure::Unavailable),
    ));
    assert!(app.jobs.details.is_some());
    assert!(!app.jobs.loading);
    assert!(app.jobs.collapsed.contains("build"));
    let _ = app.update_jobs(Event::Tick);
    assert_eq!(app.jobs.request, request);
    let _ = app.update_jobs(Event::Select(501));
    let _ = app.update_jobs(Event::Loaded(app.jobs.epoch, request, 500, Ok(details())));
    assert!(app.jobs.details.is_none());
    assert!(app.jobs.loading);
    loaded(&mut app);
    assert!(app.jobs.details.is_some());
    let epoch = app.jobs.epoch;
    let request = app.jobs.request;
    app.reset_jobs();
    let _ = app.update_jobs(Event::Select(501));
    let _ = app.update_jobs(Event::Loaded(epoch, request, 501, Ok(details())));
    assert!(app.jobs.details.is_none());
}

#[test]
fn job_actions_require_current_capabilities_and_block_duplicate_submissions() {
    let mut app = selected();
    for (id, action) in [
        (501, Action::Job(3, JobAction::Play)),
        (500, Action::Job(99, JobAction::Play)),
        (500, Action::Job(3, JobAction::Cancel)),
    ] {
        let _ = app.update_jobs(Event::Control(id, action));
        assert!(app.jobs.pending.is_none());
    }
    let action = Action::Job(3, JobAction::Play);
    let _ = app.update_jobs(Event::Refresh);
    let reading = app.jobs.request;
    let _ = app.update_jobs(Event::Control(500, action));
    let request = app.jobs.action_request;
    assert_eq!(app.jobs.pending, Some((500, action)));
    let _ = app.update_jobs(Event::Loaded(
        app.jobs.epoch,
        reading,
        500,
        Ok(Details::default()),
    ));
    assert_eq!(app.jobs.details.as_ref().unwrap().stages.len(), 3);
    assert!(!app.jobs.loading);
    let _ = app.update_jobs(Event::Control(500, Action::CancelPipeline));
    let _ = app.update_jobs(Event::Tick);
    assert_eq!(app.jobs.action_request, request);
    let _ = app.update_jobs(Event::Controlled(app.jobs.epoch, request, 501, Ok(())));
    assert!(app.jobs.pending.is_some());
    let _ = app.update_jobs(Event::Controlled(app.jobs.epoch, request, 500, Ok(())));
    assert!(app.jobs.pending.is_none());
    assert!(app.jobs.loading);
    assert!(app.jobs.details.is_some());
    loaded(&mut app);
    let _ = app.update_jobs(Event::Control(500, Action::Job(2, JobAction::Cancel)));
    let _ = app.update_jobs(Event::Controlled(
        app.jobs.epoch,
        app.jobs.action_request,
        500,
        Err(Failure::PermissionDenied),
    ));
    assert!(app.jobs.pending.is_none());
    assert_eq!(app.jobs.action_error, Some(Failure::PermissionDenied));
    let request = app.jobs.action_request;
    let _ = app.update_jobs(Event::Control(500, action));
    let _ = app.update_jobs(Event::Tick);
    assert_eq!(app.jobs.action_request, request);
    assert!(!app.jobs.loading);
    assert!(app.jobs.details.is_some());
}

#[test]
fn action_results_do_not_replace_a_different_pipeline_or_project() {
    let mut app = selected();
    let _ = app.update_jobs(Event::Control(500, Action::Job(3, JobAction::Play)));
    let epoch = app.jobs.epoch;
    let request = app.jobs.action_request;
    let _ = app.update_jobs(Event::Back);
    let _ = app.update_jobs(Event::Select(501));
    assert!(!app.jobs.loading);
    let _ = app.update_jobs(Event::Controlled(
        epoch,
        request,
        500,
        Err(Failure::Unavailable),
    ));
    assert_eq!(app.jobs.selected.as_ref().unwrap().id, 501);
    assert!(app.jobs.loading);
    assert!(app.jobs.action_error.is_none());
    assert!(
        app.notifications
            .items
            .iter()
            .any(|item| item.kind == crate::app::notifications::Kind::Error)
    );
    app.reset_jobs();
    let _ = app.update_jobs(Event::Controlled(epoch, request, 500, Ok(())));
    assert!(app.jobs.selected.is_none());
    assert!(!app.jobs.loading);
}

#[test]
fn signing_in_refreshes_permissions_and_invalidates_anonymous_job_responses() {
    let mut app = selected();
    app.jobs.action_error = Some(Failure::PermissionDenied);
    let _ = app.refresh_jobs();
    let previous = app.jobs.request;
    let _ = app.update(Message::Auth(crate::app::auth::Message::Finished(
        app.auth_request,
        Ok("new-user".into()),
    )));
    assert!(app.jobs.action_error.is_none());
    assert!(app.jobs.loading);
    assert!(app.jobs.request > previous);
    let _ = app.update_jobs(Event::Loaded(
        app.jobs.epoch,
        previous,
        500,
        Err(Failure::Access),
    ));
    assert!(app.jobs.error.is_none());
    assert!(app.jobs.loading);
    let epoch = app.jobs.epoch;
    let _ = app.update(Message::Auth(crate::app::auth::Message::Disconnect));
    assert!(app.jobs.selected.is_none());
    assert!(app.jobs.epoch > epoch);
}

fn settings() -> iced::Settings {
    iced::Settings {
        default_font: theme::font(),
        ..Default::default()
    }
}

#[test]
fn pipeline_rows_show_commits_and_open_the_provider_page() {
    for (provider, url) in [
        (
            Provider::GitHub,
            "https://github.com/team/app/actions/runs/500",
        ),
        (
            Provider::GitLab,
            "https://gitlab.example.test/team/app/-/pipelines/500",
        ),
    ] {
        let mut app = app(Language::Russian);
        app.repository
            .as_mut()
            .unwrap()
            .remote
            .as_mut()
            .unwrap()
            .provider = Some(provider);
        let PipelineState::Ready(runs) = &mut app.pipelines else {
            panic!("fixture pipelines must be ready");
        };
        runs[0].commit = Some("1234567890abcdef1234567890abcdef12345678".into());
        runs[0].url = url.into();
        app.jobs.overview.remember(500, "running", &details());
        for (visual_theme, suffix) in [(Theme::Dark, "dark"), (Theme::Light, "light")] {
            let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
            assert!(ui.find("12345678").unwrap().visible_bounds().is_some());
            assert!(ui.find(Id::new("pipeline-commit-501")).is_err());
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&visual_theme)
                    .unwrap()
                    .matches_image(output.join(format!("pipeline-commits-{provider:?}-{suffix}")))
                    .unwrap();
            }
            ui.click(Id::new("pipeline-500")).unwrap();
            assert!(matches!(
                ui.into_messages().collect::<Vec<_>>().as_slice(),
                [Message::OpenUrl(target)] if target == url
            ));
        }
    }
}

#[test]
fn pipeline_jobs_interface_supports_selection_stages_actions_and_three_languages() {
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = app(language);
        app.jobs.overview.remember(500, "running", &details());
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("pipeline-node-500-stage-0")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("pipeline-popup-details")).unwrap();
        for message in ui.into_messages() {
            assert!(matches!(message, Message::Jobs(Event::Select(500))));
            let _ = app.update(message);
        }
        loaded(&mut app);
        for (width, height) in [(1080., 720.), (1440., 920.)] {
            let mut ui = iced_test::Simulator::with_size(settings(), (width, height), app.view());
            for id in [
                "job-stage-0",
                "job-action-1-Retry",
                "job-action-2-Cancel",
                "job-action-3-Play",
                "cancel-pipeline",
                "jobs-back",
            ] {
                let bounds = ui.find(Id::new(id)).unwrap().visible_bounds().unwrap();
                assert!(bounds.width > 10. && bounds.height > 10.);
                assert!(bounds.x + bounds.width <= width && bounds.y + bounds.height <= height);
            }
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&Theme::Dark)
                    .unwrap()
                    .matches_image(output.join(format!("jobs-{suffix}-{width}")))
                    .unwrap();
            }
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("job-stage-0")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        assert!(ui.find(Id::new("job-1")).is_err());
        ui.click(Id::new("job-action-3-Play")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(
            app.jobs.pending,
            Some((500, Action::Job(3, JobAction::Play)))
        );
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        assert!(ui.find(app.i18n.text(Key::JobActionPending)).is_ok());
        ui.click(Id::new("job-action-3-Play")).unwrap();
        assert!(ui.into_messages().next().is_none());
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("jobs-back")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.jobs.selected.is_none());
    }
}

#[test]
fn github_retry_labels_include_dependents_and_errors_keep_job_rows_visible() {
    let mut app = selected();
    app.repository
        .as_mut()
        .unwrap()
        .remote
        .as_mut()
        .unwrap()
        .provider = Some(Provider::GitHub);
    let mut details = details();
    details.stages.truncate(1);
    details.stages[0].name = None;
    app.jobs.details = Some(details);
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    assert!(ui.find(app.i18n.text(Key::RetryJobWithDependents)).is_ok());
    assert!(ui.find(app.i18n.text(Key::CancelJob)).is_err());
    assert!(ui.find(app.i18n.text(Key::CancelPipeline)).is_ok());
    drop(ui);
    app.jobs.action_error = Some(Failure::PermissionDenied);
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    assert!(ui.find(app.i18n.text(Key::JobActionAccess)).is_ok());
    assert!(ui.find(Id::new("job-1")).is_ok());
    ui.click(Id::new("job-action-1-Retry")).unwrap();
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn pipeline_stage_popovers_support_both_providers_themes_and_languages() {
    use super::overview::{Event as OverviewEvent, Target};
    use crate::domain::workspace::Appearance;
    for provider in [Provider::GitLab, Provider::GitHub] {
        for (language, suffix) in [
            (Language::English, "en"),
            (Language::Russian, "ru"),
            (Language::Spanish, "es"),
        ] {
            for (appearance, visual_theme, theme_name) in [
                (Appearance::Dark, Theme::Dark, "dark"),
                (Appearance::Light, Theme::Light, "light"),
            ] {
                let mut app = app(language);
                app.workspace.preferences.appearance = appearance;
                app.repository
                    .as_mut()
                    .unwrap()
                    .remote
                    .as_mut()
                    .unwrap()
                    .provider = Some(provider);
                let mut data = details();
                if provider == Provider::GitHub {
                    let jobs = data
                        .stages
                        .into_iter()
                        .flat_map(|stage| stage.jobs)
                        .map(|mut job| {
                            job.name = format!("Tests / {}", job.name);
                            job.status = "success".into();
                            job.actions = vec![JobAction::Retry];
                            job
                        })
                        .collect();
                    data.stages = vec![Stage {
                        name: None,
                        provider_status: None,
                        jobs,
                    }];
                } else {
                    let mut parallel = data.stages[0].jobs[0].clone();
                    parallel.id = 4;
                    parallel.name = "compile-windows".into();
                    data.stages[0].jobs.push(parallel);
                }
                app.jobs.overview.remember(500, "running", &data);
                app.jobs.overview.remember(501, "running", &data);
                let node = if provider == Provider::GitHub {
                    "pipeline-node-500-job-1"
                } else {
                    "pipeline-node-500-stage-0"
                };
                let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
                if provider == Provider::GitHub {
                    for id in [
                        "pipeline-node-500-job-1",
                        "pipeline-node-500-job-2",
                        "pipeline-node-500-job-3",
                    ] {
                        assert!(ui.find(Id::new(id)).is_ok());
                    }
                    assert!(ui.find(Id::new("pipeline-node-500-stage-0")).is_err());
                }
                ui.click(Id::new(node)).unwrap();
                for message in ui.into_messages() {
                    let _ = app.update(message);
                }
                assert!(app.jobs.selected.is_none());
                assert!(app.jobs.overview.popup.is_some());
                let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
                let bounds = ui
                    .find(Id::new("pipeline-stage-popup"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(bounds.width >= 300. && bounds.height >= 100.);
                assert!(
                    bounds.x >= 0.
                        && bounds.y >= 0.
                        && bounds.x + bounds.width <= 1080.
                        && bounds.y + bounds.height <= 720.
                );
                assert!(ui.find(Id::new("pipeline-popup-job-1")).is_ok());
                assert_eq!(
                    ui.find(Id::new("pipeline-popup-job-4")).is_ok(),
                    provider == Provider::GitLab
                );
                assert!(ui.find(Id::new("pipeline-popup-job-2")).is_err());
                if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                    ui.snapshot(&visual_theme)
                        .unwrap()
                        .matches_image(output.join(format!(
                            "pipeline-popover-{provider:?}-{suffix}-{theme_name}"
                        )))
                        .unwrap();
                }
                ui.click(Id::new("pipeline-popup-action-1-Retry")).unwrap();
                for message in ui.into_messages() {
                    let _ = app.update(message);
                }
                assert_eq!(
                    app.jobs.pending,
                    Some((500, Action::Job(1, JobAction::Retry)))
                );
                let request = app.jobs.action_request;
                let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
                ui.click(Id::new("pipeline-popup-action-1-Retry")).unwrap();
                for message in ui.into_messages() {
                    let _ = app.update(message);
                }
                assert_eq!(app.jobs.action_request, request);
                let _ = app.update(Message::DismissDialog);
                assert!(app.jobs.overview.popup.is_none());
                app.jobs.pending = None;
                let target = if provider == Provider::GitHub {
                    Target::Job(1)
                } else {
                    Target::Stage("build".into())
                };
                let _ = app.update_jobs(Event::Overview(OverviewEvent::Open(
                    500,
                    target,
                    iced::Point::new(1070., 710.),
                )));
                let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
                let bounds = ui
                    .find(Id::new("pipeline-stage-popup"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(bounds.x + bounds.width <= 1080. && bounds.y + bounds.height <= 720.);
                ui.click(Id::new("pipeline-popup-details")).unwrap();
                for message in ui.into_messages() {
                    let _ = app.update(message);
                }
                assert_eq!(app.jobs.selected.as_ref().unwrap().id, 500);
                assert!(app.jobs.details.is_some());
                assert!(app.jobs.overview.popup.is_none());
            }
        }
    }
}

#[test]
fn long_pipeline_chains_and_stage_popovers_scroll_to_the_last_job() {
    use super::overview::{Event as OverviewEvent, Target};
    use iced::{
        Event as IcedEvent, Point,
        mouse::{Event as MouseEvent, ScrollDelta},
    };
    let mut app = app(Language::Russian);
    let data = Details {
        stages: vec![Stage {
            name: Some("test".into()),
            provider_status: None,
            jobs: (1..=40)
                .map(|id| Job {
                    id,
                    name: format!("Test / target-{id} with a long job name"),
                    status: "success".into(),
                    url: String::new(),
                    actions: vec![JobAction::Retry],
                })
                .collect(),
        }],
        cancelable: false,
    };
    app.repository
        .as_mut()
        .unwrap()
        .remote
        .as_mut()
        .unwrap()
        .provider = Some(Provider::GitHub);
    app.jobs.overview.remember(500, "running", &data);
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    let first = ui
        .find(Id::new("pipeline-node-500-job-1"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    assert!(
        ui.find(Id::new("pipeline-node-500-job-40"))
            .unwrap()
            .visible_bounds()
            .is_none()
    );
    ui.point_at(first.center());
    ui.simulate([IcedEvent::Mouse(MouseEvent::WheelScrolled {
        delta: ScrollDelta::Pixels { x: -10000., y: 0. },
    })]);
    ui.click(Id::new("pipeline-node-500-job-40")).unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    assert_eq!(
        app.jobs.overview.popup.as_ref().unwrap().target,
        Target::Job(40)
    );
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    ui.click(Id::new("workspace-heading")).unwrap();
    for message in ui.into_messages() {
        assert!(!matches!(message, Message::DragWindow));
        let _ = app.update(message);
    }
    assert!(app.jobs.overview.popup.is_none());
    app.repository
        .as_mut()
        .unwrap()
        .remote
        .as_mut()
        .unwrap()
        .provider = Some(Provider::GitLab);
    let _ = app.update_jobs(Event::Overview(OverviewEvent::Open(
        500,
        Target::Stage("test".into()),
        Point::new(1000., 680.),
    )));
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    let first = ui
        .find(Id::new("pipeline-popup-job-1"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    assert!(
        ui.find(Id::new("pipeline-popup-job-40"))
            .unwrap()
            .visible_bounds()
            .is_none()
    );
    ui.point_at(first.center());
    ui.simulate([IcedEvent::Mouse(MouseEvent::WheelScrolled {
        delta: ScrollDelta::Pixels { x: 0., y: -10000. },
    })]);
    ui.click(Id::new("pipeline-popup-action-40-Retry")).unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    assert_eq!(
        app.jobs.pending,
        Some((500, Action::Job(40, JobAction::Retry)))
    );
}
