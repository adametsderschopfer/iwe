use super::*;

#[test]
fn actions_require_a_matching_job_and_its_current_capability() {
    let details = Details {
        stages: vec![Stage {
            name: Some("deploy".into()),
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
