//! One worker-facing adapter, retaining the existing session owners.
use crate::{private_session, worker::Connection};
use beacn_link_protocol::{
    catalog::ApplicationService, link_session::Status, routing_control::StopReason,
};

pub enum Owner {
    Disabled(Status),
    Private(private_session::Configuration, private_session::Session),
    #[cfg(feature = "studio-session-test")]
    Studio(crate::studio_session::Session),
}
impl Owner {
    pub fn new(connection: &Connection) -> Self {
        match connection {
            Connection::PrivateSession(config) => Self::Private(config.clone(), Default::default()),
            #[cfg(feature = "studio-session-test")]
            Connection::StudioSession(config) => {
                Self::Studio(crate::studio_session::Session::new(config.clone()))
            }
            _ => Self::Disabled(Default::default()),
        }
    }
    pub fn available(&self) -> bool {
        !matches!(self, Self::Disabled(_))
    }
    pub fn private(&self) -> bool {
        matches!(self, Self::Private(..))
    }
    pub fn status(&self) -> &Status {
        match self {
            Self::Disabled(status) => status,
            Self::Private(_, link) => link.status(),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.status(),
        }
    }
    pub fn start(&mut self, service: &ApplicationService) -> Result<(), String> {
        match self {
            Self::Disabled(_) => Err("Device connection is not enabled in this preview.".into()),
            Self::Private(config, link) => private_session::start(link, service, config),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.start(service),
        }
    }
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        match self {
            Self::Disabled(_) => Ok(()),
            Self::Private(_, link) => link.check(service),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.check(service),
        }
    }
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        match self {
            Self::Disabled(_) => Ok(()),
            Self::Private(_, link) => link.tick(service),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.tick(service),
        }
    }
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        match self {
            Self::Disabled(_) => Ok(()),
            Self::Private(_, link) => link.stop(reason),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.stop(reason),
        }
    }
}
