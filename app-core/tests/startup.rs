use beacn_link_protocol::link_session::{Phase, Status};
use beacn_linx_core::{
    model::{Command, Request, Snapshot},
    startup::{State, Supervisor},
};

#[test]
fn launch_wait_attach_pause_and_reattach() {
    let mut supervisor = Supervisor::new(true);
    let idle = Status::default();
    assert!(!supervisor.update(&idle, false, true));
    assert_eq!(supervisor.state, State::Waiting);
    assert!(!supervisor.update(&idle, true, false));
    assert_eq!(supervisor.state, State::WaitingApplications);
    assert!(supervisor.update(&idle, true, true));
    assert!(!supervisor.update(&idle, true, true));
    supervisor.pause();
    assert!(!supervisor.update(&idle, false, true));
    assert!(!supervisor.update(&idle, true, true));
    assert_eq!(supervisor.state, State::Paused);
    supervisor.set_automatic(true);
    assert!(supervisor.update(&idle, true, true));
    assert!(!supervisor.update(&idle, false, true));
    assert!(supervisor.update(&idle, true, true));
}

#[test]
fn failed_attempt_and_bounded_completion_do_not_loop() {
    for error in [None, Some("transport failed".into())] {
        let mut supervisor = Supervisor::new(true);
        assert!(supervisor.update(&Status::default(), true, true));
        let stopped = Status {
            phase: Phase::Stopped,
            error,
            ..Status::default()
        };
        for _ in 0..100 {
            assert!(!supervisor.update(&stopped, true, true));
        }
        supervisor.set_automatic(true);
        assert!(supervisor.update(&stopped, true, true));
    }
}

#[test]
fn cleanup_review_wins_over_toggle_and_manual_intent() {
    let mut supervisor = Supervisor::new(true);
    let review = Status {
        phase: Phase::NeedsReview,
        ..Status::default()
    };
    supervisor.set_automatic(true);
    assert!(!supervisor.update(&review, true, true));
    supervisor.manual_start();
    assert!(!supervisor.update(&review, true, true));
    assert_eq!(supervisor.state, State::Attention);
}

#[test]
fn manual_mode_does_not_start_but_running_session_remains_running() {
    let mut supervisor = Supervisor::new(false);
    assert!(!supervisor.update(&Status::default(), true, true));
    assert_eq!(supervisor.state, State::Ready);
    supervisor.manual_start();
    supervisor.set_automatic(false);
    let running = Status {
        phase: Phase::Running,
        ..Status::default()
    };
    assert!(!supervisor.update(&running, true, true));
    assert_eq!(supervisor.state, State::Running);
    supervisor.set_automatic(true);
    assert!(!supervisor.update(&running, true, true));
    assert!(!supervisor.update(
        &Status {
            phase: Phase::Stopped,
            ..Status::default()
        },
        true,
        true
    ));
}

#[test]
fn pause_and_preference_work_without_a_catalog() {
    let view = Snapshot::default();
    assert!(Request::from_view(Command::AutoConnect(true), &view).is_ok());
    assert!(Request::from_view(Command::Stop, &view).is_ok());
    assert!(Request::from_view(Command::Start, &view).is_err());
}
