use beacn_link_protocol::link_session::{Phase, Status};
use beacn_link_protocol::{catalog::Catalog, planner::ApplicationId};
use beacn_linx_core::{
    model::{Command, Model, Request, Snapshot},
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
#[test]
fn waiting_for_media_does_not_override_owner_pause_or_cleanup_review() {
    let mut supervisor = Supervisor::new(false);
    supervisor.manual_start();
    let stopped = Status {
        phase: Phase::Stopped,
        ..Status::default()
    };
    supervisor.update(&stopped, true, true);
    supervisor.waiting_audio();
    assert_eq!(supervisor.state, State::WaitingAudio);
    supervisor.pause();
    supervisor.update(&stopped, true, true);
    supervisor.waiting_audio();
    assert_eq!(supervisor.state, State::Paused);
    supervisor.manual_start();
    supervisor.update(
        &Status {
            phase: Phase::NeedsReview,
            ..Status::default()
        },
        true,
        true,
    );
    supervisor.waiting_audio();
    assert_eq!(supervisor.state, State::Attention);
}

fn assignment_catalog(generation: u64, first_stream: u64) -> Catalog {
    beacn_link_protocol::catalog::parse(
        generation,
        &format!(
            "catalog\t1\t2\t74657374\napp\t3\t4\t{first_stream}\tlive\t46697265666f78\t66697265666f78\napp\t6\t7\t8\tlive\t4d75736963\t6d75736963\n"
        ),
    )
    .unwrap()
}

fn assignment_view(catalog: Catalog) -> Snapshot {
    Snapshot {
        connected: true,
        catalog: Some(catalog),
        ..Snapshot::default()
    }
}

#[test]
fn assign_request_rejects_invalid_pairs_and_replacing_or_renaming_selection() {
    let catalog = assignment_catalog(1, 5);
    let target = ApplicationId("pw-client:3:pid:4".into());
    let other = ApplicationId("pw-client:6:pid:7".into());
    let view = assignment_view(catalog.clone());
    for pair in [0, 5, 255] {
        assert!(Request::from_view(
            Command::Assign(target.clone(), "Firefox".into(), pair),
            &view
        )
        .is_err());
    }
    let mut other_selected = assignment_view(catalog.clone());
    other_selected.selected.insert(other, "Music".into());
    assert!(Request::from_view(
        Command::Assign(target.clone(), "Firefox".into(), 1),
        &other_selected
    )
    .is_err());
    let mut renamed = assignment_view(catalog);
    renamed
        .selected
        .insert(target.clone(), "Firefox Alias".into());
    assert!(Request::from_view(Command::Assign(target, "Firefox".into(), 1), &renamed).is_err());
}

#[test]
fn assign_request_is_rejected_if_target_stream_changed_after_display() {
    let target = ApplicationId("pw-client:3:pid:4".into());
    let request = Request::from_view(
        Command::Assign(target, "Firefox".into(), 1),
        &assignment_view(assignment_catalog(1, 5)),
    )
    .unwrap();
    let mut model = Model::default();
    model.observe(assignment_catalog(1, 5)).unwrap();
    model.observe(assignment_catalog(2, 9)).unwrap();
    assert!(model.accept(request).is_err());
}
