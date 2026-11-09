use super::*;

#[test]
fn actions_require_a_matching_job_and_its_current_capability() {
    let details = Details {
        stages: vec![Stage {
            name: Some("deploy".into()),
            provider_status: None,
            jobs: vec![Job {
                id: 42,
                name: "preview".into(),
                status: "manual".into(),
                url: String::new(),
                actions: vec![JobAction::Play],
            }],
        }],
        cancelable: false,
    };
    assert!(details.allows(Action::Job(42, JobAction::Play)));
    assert!(!details.allows(Action::Job(42, JobAction::Cancel)));
    assert!(!details.allows(Action::Job(43, JobAction::Play)));
    assert!(!details.allows(Action::CancelPipeline));
    assert!(!JobAction::Cancel.supported(Provider::GitHub));
    assert!(JobAction::Retry.supported(Provider::GitHub));
}

#[test]
fn stage_status_keeps_failures_and_unfinished_jobs_visible() {
    for (values, expected) in [
        (vec![], Status::Skipped),
        (vec!["success", "skipped", "neutral"], Status::Passed),
        (vec!["success", "in_progress"], Status::Running),
        (vec!["running", "failed"], Status::Failed),
        (vec!["passed", "manual"], Status::Manual),
        (vec!["success", "waiting_for_resource"], Status::Pending),
        (vec!["success", "canceled"], Status::Cancelled),
        (vec!["success", "unrecognized"], Status::Unknown),
        (vec!["running", "canceling"], Status::Canceling),
        (vec!["skipped", "neutral"], Status::Skipped),
    ] {
        let mut stage = Stage {
            name: Some("test".into()),
            provider_status: None,
            jobs: values
                .iter()
                .enumerate()
                .map(|(id, status)| Job {
                    id: id as u64,
                    name: format!("job-{id}"),
                    status: (*status).into(),
                    url: String::new(),
                    actions: Vec::new(),
                })
                .collect(),
        };
        assert_eq!(stage.status(), expected, "{values:?}");
        stage.jobs.reverse();
        assert_eq!(
            stage.status(),
            expected,
            "Order must not change the aggregate"
        );
    }
}
