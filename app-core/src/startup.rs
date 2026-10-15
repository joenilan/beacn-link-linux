//! App intent only. Session, transport and receipt owners still decide authority.
use beacn_link_protocol::link_session::{Phase, Status};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Waiting,
    WaitingApplications,
    Ready,
    Running,
    Paused,
    Attention,
}
impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for connection",
            Self::WaitingApplications => "Waiting for applications",
            Self::Ready => "Ready to connect",
            Self::Running => "Link active",
            Self::Paused => "Link paused",
            Self::Attention => "Link needs attention",
        }
    }
}
pub struct Supervisor {
    pub automatic: bool,
    pub state: State,
    paused: bool,
    attempted: bool,
    available: bool,
}
impl Supervisor {
    pub fn new(automatic: bool) -> Self {
        Self {
            automatic,
            state: State::Waiting,
            paused: false,
            attempted: false,
            available: false,
        }
    }
    pub fn set_automatic(&mut self, enabled: bool) {
        self.automatic = enabled;
        self.paused = false;
        self.attempted = false;
    }
    pub fn manual_start(&mut self) {
        self.paused = false;
        self.attempted = true;
    }
    pub fn pause(&mut self) {
        self.paused = true;
    }
    /// One attempt per availability period. Errors and bounded completion never
    /// cause an automatic restart loop. Explicit re-arm is required after either.
    pub fn update(&mut self, link: &Status, available: bool, selected: bool) -> bool {
        if self.available && !available {
            self.attempted = false;
        }
        self.available = available;
        self.state = if link.phase == Phase::NeedsReview {
            State::Attention
        } else if link.phase == Phase::Running {
            self.attempted = true;
            State::Running
        } else if self.paused {
            State::Paused
        } else if self.attempted {
            if link.error.is_some() {
                State::Attention
            } else {
                State::Paused
            }
        } else if !available {
            State::Waiting
        } else if !selected {
            State::WaitingApplications
        } else {
            State::Ready
        };
        if self.state == State::Ready && self.automatic {
            self.attempted = true;
            return true;
        }
        false
    }
}
