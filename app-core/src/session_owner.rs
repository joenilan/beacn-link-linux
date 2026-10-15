//! One worker-facing adapter, retaining the existing session owners.
use crate::{private_session, worker::Connection};
use beacn_link_protocol::{
    catalog::ApplicationService, link_session::Status, routing_control::StopReason,
};

pub enum Owner {
    #[cfg(feature = "desktop-session-test")]
    Desktop(crate::desktop_session::Session),
    Disabled(Status),
    Private(
        private_session::Configuration,
        private_session::Session,
        bool,
    ),
    #[cfg(feature = "studio-session-test")]
    Studio(crate::studio_session::Session),
    #[cfg(feature = "native-playback-test")]
    NativePlayback(
        crate::studio_session::Session<beacn_link_protocol::scripted_peer::ScriptedPeer>,
    ),
}
impl Owner {
    pub fn new(connection: &Connection) -> Self {
        match connection {
            #[cfg(feature = "desktop-session-test")]
            Connection::Studio => Self::Desktop(crate::desktop_session::Session::managed()),
            #[cfg(feature = "desktop-session-test")]
            Connection::DesktopStudioSession(config) => {
                Self::Desktop(crate::desktop_session::Session::new(config.clone()))
            }
            Connection::PrivateSession(config) => {
                Self::Private(config.clone(), Default::default(), false)
            }
            Connection::ManagedPrivateSession(config) => {
                Self::Private(config.clone(), Default::default(), true)
            }
            #[cfg(feature = "studio-session-test")]
            Connection::StudioSession(config) => {
                Self::Studio(crate::studio_session::Session::new(config.clone()))
            }
            #[cfg(feature = "native-playback-test")]
            Connection::NativePlaybackSession(config) => {
                Self::NativePlayback(crate::studio_session::Session::new(config.clone()))
            }
            _ => Self::Disabled(Default::default()),
        }
    }
    pub fn available(&self) -> bool {
        !matches!(self, Self::Disabled(_))
    }
    #[cfg(feature = "desktop-session-test")]
    pub fn desktop_cleanup(&self) -> Option<Result<(), String>> {
        match self {
            Self::Desktop(link) => link.cleanup(),
            _ => None,
        }
    }
    pub fn private(&self) -> bool {
        #[cfg(feature = "native-playback-test")]
        if matches!(self, Self::NativePlayback(_)) {
            return true;
        }
        matches!(self, Self::Private(..))
    }
    pub fn status(&self) -> &Status {
        match self {
            #[cfg(feature = "desktop-session-test")]
            Self::Desktop(link) => link.status(),
            Self::Disabled(status) => status,
            Self::Private(_, link, _) => link.status(),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.status(),
            #[cfg(feature = "native-playback-test")]
            Self::NativePlayback(link) => link.status(),
        }
    }
    pub fn start(&mut self, service: &ApplicationService) -> Result<(), String> {
        match self {
            #[cfg(feature = "desktop-session-test")]
            Self::Desktop(link) => link.start(service),
            Self::Disabled(_) => Err("Device connection is not enabled in this preview.".into()),
            Self::Private(config, link, true) => {
                private_session::start_managed(link, service, config)
            }
            Self::Private(config, link, false) => private_session::start(link, service, config),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.start(service),
            #[cfg(feature = "native-playback-test")]
            Self::NativePlayback(link) => link.start_synthetic(service),
        }
    }
    pub fn assign(&mut self, service: &ApplicationService, pair: u8) -> Result<(), String> {
        match self {
            #[cfg(feature = "desktop-session-test")]
            Self::Desktop(link) => link.assign(service, pair),
            _ => Err("Local assignment is available only for a desktop Studio session.".into()),
        }
    }
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        match self {
            #[cfg(feature = "desktop-session-test")]
            Self::Desktop(link) => link.check(service),
            Self::Disabled(_) => Ok(()),
            Self::Private(_, link, _) => link.check(service),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.check(service),
            #[cfg(feature = "native-playback-test")]
            Self::NativePlayback(link) => link.check(service),
        }
    }
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        match self {
            #[cfg(feature = "desktop-session-test")]
            Self::Desktop(link) => link.tick(service),
            Self::Disabled(_) => Ok(()),
            Self::Private(_, link, _) => link.tick(service),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.tick(service),
            #[cfg(feature = "native-playback-test")]
            Self::NativePlayback(link) => link.tick(service),
        }
    }
    pub fn stop(&mut self, reason: StopReason) -> Result<(), String> {
        match self {
            #[cfg(feature = "desktop-session-test")]
            Self::Desktop(link) => link.stop(reason),
            Self::Disabled(_) => Ok(()),
            Self::Private(_, link, _) => link.stop(reason),
            #[cfg(feature = "studio-session-test")]
            Self::Studio(link) => link.stop(reason),
            #[cfg(feature = "native-playback-test")]
            Self::NativePlayback(link) => link.stop(reason),
        }
    }
}
