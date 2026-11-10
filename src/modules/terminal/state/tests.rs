use super::*;

fn add(state: &mut State) -> u64 {
    let _ = state.update(Event::Start, Some(Path::new("project")));
    state.active.unwrap()
}

#[test]
fn new_tabs_keep_independent_screens_sizes_and_active_selection() {
    let mut state = State::default();
    let first = add(&mut state);
    state.tabs[0].parser.process(b"first-output");
    let second = add(&mut state);
    state.tabs[1].parser.process(b"second-output");
    assert_ne!(first, second);
    let _ = state.update(Event::Resize(first, 12, 60), None);
    assert_eq!(state.tabs[0].parser.screen().size(), (12, 60));
    assert_eq!(state.tabs[1].parser.screen().size(), (24, 80));
    assert_eq!(state.active, Some(second));
    let _ = state.update(Event::Select(first), None);
    assert_eq!(
        state.active_tab().unwrap().parser.screen().contents(),
        "first-output"
    );
    let _ = state.update(Event::Select(second), None);
    assert_eq!(
        state.active_tab().unwrap().parser.screen().contents(),
        "second-output"
    );
}

#[test]
fn hiding_keeps_tabs_output_and_pending_sessions() {
    let mut state = State::default();
    let first = add(&mut state);
    state.tabs[0].parser.process(b"retained");
    let second = add(&mut state);
    let _ = state.update(Event::Hide, None);
    assert!(!state.open);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.active, Some(second));
    assert_eq!(state.tabs[0].request, first);
    assert_eq!(state.tabs[0].status, Status::Starting);
    assert_eq!(state.tabs[0].parser.screen().contents(), "retained");
}

#[test]
fn closing_tabs_preserves_another_selection_and_hides_only_after_the_last_tab() {
    let mut state = State::default();
    let first = add(&mut state);
    let second = add(&mut state);
    let third = add(&mut state);
    let _ = state.update(Event::Close(first), None);
    assert_eq!(state.active, Some(third));
    assert!(state.open);
    let _ = state.update(Event::Close(third), None);
    assert_eq!(state.active, Some(second));
    assert!(state.open);
    let _ = state.update(Event::Close(second), None);
    assert!(state.tabs.is_empty());
    assert_eq!(state.active, None);
    assert!(!state.open);
}

#[test]
fn closed_tabs_and_previous_projects_cannot_receive_late_results() {
    let mut state = State::default();
    let closed = add(&mut state);
    let current = add(&mut state);
    let _ = state.update(Event::Close(closed), None);
    let _ = state.update(Event::Started(closed, Err(())), None);
    let _ = state.update(Event::Resize(closed, 1, 1), None);
    assert_eq!(state.active_tab().unwrap().status, Status::Starting);
    assert_eq!(state.active_tab().unwrap().parser.screen().size(), (24, 80));
    let _ = state.update(Event::Started(current, Err(())), None);
    assert_eq!(state.active_tab().unwrap().status, Status::Failed);
    state.reset();
    let next = add(&mut state);
    assert!(next > current);
    assert_eq!(state.active_tab().unwrap().number, 1);
    let _ = state.update(Event::Started(current, Err(())), None);
    assert_eq!(state.active_tab().unwrap().status, Status::Starting);
}
