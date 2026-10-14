//! Selected/pinned USB gate; handshake-only by default, opt-in bounded publication.
use crate::{
    session::{Transport, TransportError, IN_PIPE, OUT_PIPE},
    usb_descriptors, MAX_FRAME,
};
use std::{fmt, time::Duration};

pub const MAX_TIMEOUT: Duration = Duration::from_millis(500);
pub const DIAGNOSTIC_INITIALIZATION_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_PUBLICATION_CYCLES: usize = 40;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Startup {
    #[default]
    SelectedAlternate,
    /// Explicit experiment matching the observed Windows constructor order:
    /// alternate zero, 1 ms wait, alternate one, clear vendor input halt/toggle.
    /// This is preparation before any bulk I/O, never recovery after failure.
    ObservedWindows,
}
/// One explicit application snapshot and its canonical initial list. Zero is
/// retained opaquely; this does not assign a route or infer an unroute command.
pub struct Publication {
    records: Vec<crate::Record>,
    frame: Vec<u8>,
    tracker: crate::lifecycle::InventoryTracker,
    remaining: usize,
    periodic: bool,
    poll_policy: crate::session::PollPolicy,
    initialization_timeout: Duration,
    verified_routes: bool,
}
impl Publication {
    pub fn from_inventory(inventory: &crate::planner::Inventory) -> Result<Self, Error> {
        let mut tracker = crate::lifecycle::InventoryTracker::default();
        tracker.advance(inventory).map_err(|_| Error::Publication)?;
        if inventory.applications.len() != 1 {
            return Err(Error::Publication);
        }
        let app = &inventory.applications[0];
        if app.streams.is_empty()
            || app
                .streams
                .iter()
                .any(|s| s.state != crate::planner::StreamState::Live)
        {
            return Err(Error::Publication);
        }
        let records = vec![crate::Record {
            name: app.advertised_name.clone(),
            assignment: crate::Assignment::from_byte(0),
        }];
        let payload = crate::encode_records(&records).map_err(|_| Error::Publication)?;
        let frame =
            crate::encode_frame(&crate::Frame::Data(payload)).map_err(|_| Error::Publication)?;
        Ok(Self {
            records,
            frame,
            tracker,
            remaining: 1,
            periodic: false,
            poll_policy: crate::session::PollPolicy::Strict,
            initialization_timeout: MAX_TIMEOUT,
            verified_routes: false,
        })
    }
    /// Explicit test budget; all cycles, including the first, require a newer
    /// inventory after initialization. A version is not proof of OS freshness.
    pub fn bounded(inventory: &crate::planner::Inventory, cycles: usize) -> Result<Self, Error> {
        Self::bounded_with_poll_policy(inventory, cycles, crate::session::PollPolicy::Strict)
    }
    pub fn bounded_with_poll_policy(
        inventory: &crate::planner::Inventory,
        cycles: usize,
        policy: crate::session::PollPolicy,
    ) -> Result<Self, Error> {
        if !(1..=MAX_PUBLICATION_CYCLES).contains(&cycles) {
            return Err(Error::Publication);
        }
        let mut publication = Self::from_inventory(inventory)?;
        publication.remaining = cycles;
        publication.periodic = true;
        publication.poll_policy = policy;
        Ok(publication)
    }
    pub fn records(&self) -> &[crate::Record] {
        &self.records
    }
    /// Explicit route-observed mode; every publication needs verified data.
    pub fn with_verified_routes(mut self) -> Self {
        self.verified_routes = true;
        self.periodic = true;
        self
    }
    /// Fixed bounded experiment, chosen before opening the device. This only
    /// changes the initial A2 read; it is not retry or an unlimited wait.
    pub fn with_initialization_diagnostic(mut self) -> Self {
        self.initialization_timeout = DIAGNOSTIC_INITIALIZATION_TIMEOUT;
        self
    }
    pub(crate) fn initialization_timeout(&self) -> Duration {
        self.initialization_timeout
    }
    pub(crate) fn verified_routes(&self) -> bool {
        self.verified_routes
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Observation {
    /// Provider stamp backed by the retained handle, never standalone authority.
    pub instance: u64,
    pub descriptors: Vec<u8>,
    pub configuration: u8,
    pub alternate: u8,
    pub kernel_bound: bool,
    pub audio_interfaces: Vec<(u8, u8, String)>,
}
/// Implementors must retain one exact device handle and never enumerate a
/// fallback, auto-detach a driver, retry transfers or claim audio interfaces.
pub trait Backend {
    fn observe(&mut self) -> Result<Observation, i32>;
    /// Default-denied exception for explicitly pinned, live owned playback.
    /// All device/configuration/vendor/driver checks still apply.
    fn owned_playback_valid(&mut self) -> bool {
        false
    }
    fn claim_three(&mut self) -> Result<(), i32>;
    fn select_one(&mut self) -> Result<(), i32>;
    /// Optional observed-startup operations. Unsupported by default; neither
    /// accepts an arbitrary interface/endpoint nor grants authority to retry.
    fn select_zero(&mut self) -> Result<(), i32> {
        Err(-12)
    }
    fn clear_input_halt(&mut self) -> Result<(), i32> {
        Err(-12)
    }
    fn write(&mut self, bytes: &[u8], timeout_ms: u32) -> Result<usize, i32>;
    fn read(&mut self, bytes: &mut [u8], timeout_ms: u32) -> Result<usize, i32>;
    /// Releases only this backend's successful claim; restores alternate zero.
    fn release_three(&mut self) -> Result<(), i32>;
    fn close(&mut self);
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Profile,
    Publication,
    Changed,
    Backend { stage: &'static str, code: i32 },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "USB handshake: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleanup {
    pub release: Option<Result<(), i32>>,
    pub baseline_restored: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupFailure {
    pub error: Error,
    pub cleanup: Cleanup,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    A0,
    A1,
    A2,
    Publish,
    AwaitInventory,
    PollRequest,
    PollReply,
    Complete,
    Halted,
}
impl<B: Backend> crate::session::PublicationCycles for Handshake<B> {
    fn refresh_verified_publication(
        &mut self,
        verified: &crate::route_publication::VerifiedRoutePublication,
    ) -> Result<(), TransportError> {
        let accepted = (|| {
            if self.closed || self.phase != Phase::AwaitInventory || self.check(1).is_err() {
                return None;
            }
            let publication = self.publication.as_mut()?;
            if !publication.verified_routes
                || verified.inventory().applications != publication.tracker.current()?.applications
            {
                return None;
            }
            let payload = crate::encode_records(verified.records()).ok()?;
            let frame = crate::encode_frame(&crate::Frame::Data(payload)).ok()?;
            publication.tracker.advance(verified.inventory()).ok()?;
            publication.records = verified.records().to_vec();
            publication.frame = frame;
            Some(())
        })();
        if accepted.is_some() {
            self.phase = Phase::Publish;
            Ok(())
        } else {
            self.phase = Phase::Halted;
            Err(TransportError::NoTransfer)
        }
    }
    fn refresh_publication(
        &mut self,
        inventory: &crate::planner::Inventory,
    ) -> Result<(), TransportError> {
        let accepted = (|| {
            if self.closed || self.phase != Phase::AwaitInventory || self.check(1).is_err() {
                return None;
            }
            let publication = self.publication.as_mut()?;
            if publication.verified_routes {
                return None;
            }
            let pinned = publication.tracker.current()?;
            // No new process, stream, name or state may replace the pinned test
            // source, even when the outgoing wire frame would be identical.
            if inventory.applications != pinned.applications {
                return None;
            }
            publication.tracker.advance(inventory).ok()?;
            Some(())
        })();
        if accepted.is_some() {
            self.phase = Phase::Publish;
            Ok(())
        } else {
            self.phase = Phase::Halted;
            Err(TransportError::NoTransfer)
        }
    }
}
pub struct Handshake<B: Backend> {
    backend: B,
    baseline: Option<Observation>,
    claimed: bool,
    closed: bool,
    phase: Phase,
    publication: Option<Publication>,
    initialization_timeout: Duration,
}
pub(crate) fn descriptor_profile(bytes: &[u8], configuration: u8) -> Result<(), Error> {
    let d = usb_descriptors::parse(bytes).map_err(|_| Error::Profile)?;
    let c = d.candidate(configuration).map_err(|_| Error::Profile)?;
    if d.usb_version != 0x0200
        || d.revision != 0x0100
        || c.configuration != 1
        || c.out.max_packet_size != 512
        || c.input.max_packet_size != 512
    {
        return Err(Error::Profile);
    }
    let vendor = d
        .configurations
        .iter()
        .find(|c| c.value == 1)
        .unwrap()
        .interfaces
        .iter()
        .find(|i| i.number == 3 && i.alternate == 1)
        .unwrap();
    if vendor.subclass != 0 || vendor.protocol != 0 {
        return Err(Error::Profile);
    }
    let interfaces = &d
        .configurations
        .iter()
        .find(|c| c.value == 1)
        .unwrap()
        .interfaces;
    let numbers: std::collections::BTreeSet<_> = interfaces.iter().map(|i| i.number).collect();
    if numbers != [0, 1, 2, 3].into_iter().collect()
        || d.configurations.len() != 1
        || (0..3).any(|n| {
            !interfaces
                .iter()
                .any(|i| i.number == n && i.alternate == 0 && i.class == 1)
        })
    {
        return Err(Error::Profile);
    }
    if !interfaces.iter().any(|i| {
        i.number == 3
            && i.alternate == 0
            && i.class == 0xff
            && i.subclass == 0
            && i.protocol == 0
            && i.endpoints.is_empty()
    }) {
        return Err(Error::Profile);
    }
    Ok(())
}
/// Validate only the confirmed descriptor profile; no OS or hardware access.
pub fn check_descriptors(bytes: &[u8]) -> Result<(), Error> {
    descriptor_profile(bytes, 1)
}
fn profile(o: &Observation) -> Result<(), Error> {
    descriptor_profile(&o.descriptors, o.configuration)?;
    if o.alternate != 0
        || o.kernel_bound
        || o.audio_interfaces
            .iter()
            .map(|(n, a, _)| (*n, *a))
            .collect::<Vec<_>>()
            != [(0, 0), (1, 0), (2, 0)]
    {
        return Err(Error::Profile);
    }
    Ok(())
}
/// Round positive sub-millisecond timeouts upward; never pass libusb's infinite 0.
pub fn timeout_ms(timeout: Duration) -> Option<u32> {
    timeout_ms_with_limit(timeout, MAX_TIMEOUT)
}
fn timeout_ms_with_limit(timeout: Duration, maximum: Duration) -> Option<u32> {
    if timeout.is_zero() || timeout > maximum {
        return None;
    }
    Some(timeout.as_nanos().div_ceil(1_000_000) as u32)
}
impl<B: Backend> Handshake<B> {
    /// Mutation-bearing setup. Caller must have explicit authority for claim,
    /// alternate selection, two writes/read and release restoration.
    pub fn prepare(backend: B) -> Result<Self, SetupFailure> {
        Self::prepare_with_startup(backend, Startup::SelectedAlternate)
    }
    pub fn prepare_with_startup(backend: B, startup: Startup) -> Result<Self, SetupFailure> {
        let mut this = Self {
            backend,
            baseline: None,
            claimed: false,
            closed: false,
            phase: Phase::A0,
            publication: None,
            initialization_timeout: MAX_TIMEOUT,
        };
        let result = (|| {
            let observation = this.backend.observe().map_err(|code| Error::Backend {
                stage: "observe",
                code,
            })?;
            profile(&observation)?;
            this.baseline = Some(observation);
            this.backend.claim_three().map_err(|code| Error::Backend {
                stage: "claim",
                code,
            })?;
            this.claimed = true;
            this.check(0)?;
            if startup == Startup::ObservedWindows {
                this.backend.select_zero().map_err(|code| Error::Backend {
                    stage: "alternate-zero",
                    code,
                })?;
                this.check(0)?;
                // A conservative 1 ms wait models the observed next-tick wait,
                // not the original Windows scheduler or a protocol requirement.
                std::thread::sleep(Duration::from_millis(1));
                this.check(0)?;
            }
            this.backend.select_one().map_err(|code| Error::Backend {
                stage: "alternate",
                code,
            })?;
            this.check(1)?;
            if startup == Startup::ObservedWindows {
                this.backend
                    .clear_input_halt()
                    .map_err(|code| Error::Backend {
                        stage: "clear-input-halt",
                        code,
                    })?;
                this.check(1)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => Ok(this),
            Err(error) => {
                let cleanup = this.cleanup();
                Err(SetupFailure { error, cleanup })
            }
        }
    }
    /// Explicit opt-in to the pinned list policy after a valid handshake.
    /// The supplied inventory is caller-observed, not proof of OS freshness.
    pub fn prepare_publication(backend: B, publication: Publication) -> Result<Self, SetupFailure> {
        Self::prepare(backend).map(|mut owner| {
            owner.publication = Some(publication);
            owner
        })
    }
    /// Handshake-only diagnostic: permit one A2 read up to five seconds.
    /// No publication/poll authority; writes and every other read remain capped.
    pub fn prepare_initialization_diagnostic(
        backend: B,
        startup: Startup,
    ) -> Result<Self, SetupFailure> {
        Self::prepare_with_startup(backend, startup).map(|mut owner| {
            owner.initialization_timeout = DIAGNOSTIC_INITIALIZATION_TIMEOUT;
            owner
        })
    }
    #[cfg(all(feature = "native-usb", target_os = "linux"))]
    pub(crate) fn enable_publication(mut self, publication: Publication) -> Self {
        // Only the native opener calls this immediately after prepare.
        self.publication = Some(publication);
        self
    }
    fn check(&mut self, alternate: u8) -> Result<(), Error> {
        let current = self.backend.observe().map_err(|code| Error::Backend {
            stage: "revalidate",
            code,
        })?;
        let mut expected = self.baseline.clone().ok_or(Error::Changed)?;
        expected.alternate = alternate;
        if current != expected {
            let mut playing = expected.clone();
            for (number, alternate, _) in &mut playing.audio_interfaces {
                if *number == 1 || *number == 2 {
                    *alternate = 1;
                }
            }
            let periodic = self.publication.as_ref().is_some_and(|p| p.verified_routes);
            let exchange = matches!(
                self.phase,
                Phase::Publish | Phase::AwaitInventory | Phase::PollRequest | Phase::PollReply
            );
            if current != playing || !periodic || !exchange || !self.backend.owned_playback_valid()
            {
                return Err(Error::Changed);
            }
        }
        Ok(())
    }
    fn cleanup(&mut self) -> Cleanup {
        if self.closed {
            return Cleanup {
                release: None,
                baseline_restored: false,
            };
        }
        let release = if self.claimed {
            Some(self.backend.release_three())
        } else {
            None
        };
        self.claimed = false;
        let baseline_restored = self.baseline.as_ref().is_some_and(|before| {
            release.as_ref().is_none_or(|r| r.is_ok())
                && self.backend.observe().ok().as_ref() == Some(before)
        });
        self.backend.close();
        self.closed = true;
        Cleanup {
            release,
            baseline_restored,
        }
    }
    /// Explicit completion report; Drop is only a best-effort safety net.
    pub fn close(mut self) -> Cleanup {
        self.cleanup()
    }
    fn ready(&mut self, phase: Phase, timeout: Duration) -> Result<u32, TransportError> {
        let maximum = if phase == Phase::A2 {
            self.publication
                .as_ref()
                .map_or(self.initialization_timeout, |p| p.initialization_timeout())
        } else {
            MAX_TIMEOUT
        };
        let Some(ms) = timeout_ms_with_limit(timeout, maximum) else {
            self.phase = Phase::Halted;
            return Err(TransportError::NoTransfer);
        };
        if self.closed || self.phase != phase || self.check(1).is_err() {
            self.phase = Phase::Halted;
            return Err(TransportError::NoTransfer);
        }
        Ok(ms)
    }
}
impl<B: Backend> Transport for Handshake<B> {
    fn write(
        &mut self,
        pipe: u8,
        bytes: &[u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        let (phase, expected, next) = match self.phase {
            Phase::A0 => (Phase::A0, 0xA0000000u32.to_le_bytes().to_vec(), Phase::A1),
            Phase::A1 => (Phase::A1, 0xA1000000u32.to_le_bytes().to_vec(), Phase::A2),
            Phase::Publish => (
                Phase::Publish,
                self.publication
                    .as_ref()
                    .expect("publication phase requires policy")
                    .frame
                    .clone(),
                Phase::PollRequest,
            ),
            Phase::PollRequest => (
                Phase::PollRequest,
                0xAC010000u32.to_le_bytes().to_vec(),
                Phase::PollReply,
            ),
            _ => {
                self.phase = Phase::Halted;
                return Err(TransportError::NoTransfer);
            }
        };
        if pipe != OUT_PIPE || bytes != expected {
            self.phase = Phase::Halted;
            return Err(TransportError::NoTransfer);
        }
        let ms = self.ready(phase, timeout)?;
        match self.backend.write(bytes, ms) {
            Ok(n) => {
                self.phase = if n == expected.len() {
                    next
                } else {
                    Phase::Halted
                };
                Ok(n)
            }
            Err(_) => {
                self.phase = Phase::Halted;
                Err(TransportError::Unknown)
            }
        }
    }
    fn read(
        &mut self,
        pipe: u8,
        bytes: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        if pipe != IN_PIPE || bytes.len() != MAX_FRAME {
            self.phase = Phase::Halted;
            return Err(TransportError::NoTransfer);
        }
        let phase = self.phase;
        if !matches!(phase, Phase::A2 | Phase::PollReply) {
            self.phase = Phase::Halted;
            return Err(TransportError::NoTransfer);
        }
        let ms = self.ready(phase, timeout)?;
        match self.backend.read(bytes, ms) {
            Ok(n) => {
                self.phase = if phase == Phase::A2
                    && n == MAX_FRAME
                    && bytes[..4] == 0xA2000000u32.to_le_bytes()
                {
                    if let Some(publication) = &self.publication {
                        if publication.periodic {
                            Phase::AwaitInventory
                        } else {
                            Phase::Publish
                        }
                    } else {
                        Phase::Complete
                    }
                } else if phase == Phase::PollReply
                    && n <= MAX_FRAME
                    && crate::session::decode_poll_reply_with_policy(
                        &bytes[..n],
                        self.publication
                            .as_ref()
                            .expect("poll requires policy")
                            .poll_policy,
                    )
                    .is_ok()
                {
                    let publication = self.publication.as_mut().expect("poll requires policy");
                    publication.remaining -= 1;
                    if publication.periodic && publication.remaining > 0 {
                        Phase::AwaitInventory
                    } else {
                        Phase::Complete
                    }
                } else {
                    Phase::Halted
                };
                Ok(n)
            }
            Err(_) => {
                self.phase = Phase::Halted;
                Err(TransportError::Unknown)
            }
        }
    }
}
impl<B: Backend> Drop for Handshake<B> {
    fn drop(&mut self) {
        if !self.closed {
            self.cleanup();
        }
    }
}

#[cfg(test)]
mod playback_tests {
    use super::*;
    struct Fake {
        current: Observation,
        valid: bool,
        validations: usize,
    }
    impl Backend for Fake {
        fn observe(&mut self) -> Result<Observation, i32> {
            Ok(self.current.clone())
        }
        fn owned_playback_valid(&mut self) -> bool {
            self.validations += 1;
            self.valid
        }
        fn claim_three(&mut self) -> Result<(), i32> {
            Ok(())
        }
        fn select_one(&mut self) -> Result<(), i32> {
            self.current.alternate = 1;
            Ok(())
        }
        fn release_three(&mut self) -> Result<(), i32> {
            self.current.alternate = 0;
            Ok(())
        }
        fn close(&mut self) {}
        fn write(&mut self, _: &[u8], _: u32) -> Result<usize, i32> {
            panic!("no transfers in gate test")
        }
        fn read(&mut self, _: &mut [u8], _: u32) -> Result<usize, i32> {
            panic!("no transfers in gate test")
        }
    }
    fn owner(verified: bool, valid: bool) -> Handshake<Fake> {
        let current = Observation {
            instance: 44,
            configuration: 1,
            alternate: 0,
            kernel_bound: false,
            descriptors: include_str!("../fixtures/usb-handshake.synthetic.hex")
                .split_whitespace()
                .map(|s| u8::from_str_radix(s, 16).unwrap())
                .collect(),
            audio_interfaces: (0..3).map(|n| (n, 0, "snd-usb-audio".into())).collect(),
        };
        let inventory =
            crate::snapshot::inventory_from_tsv(1, "test\ttest\tstream\tlive\n").unwrap();
        let publication = Publication::bounded(&inventory, 3).unwrap();
        Handshake::prepare_publication(
            Fake {
                current,
                valid,
                validations: 0,
            },
            if verified {
                publication.with_verified_routes()
            } else {
                publication
            },
        )
        .unwrap()
    }
    #[test]
    fn playback_exception_is_explicit_phase_limited_and_preserves_every_other_guard() {
        for (verified, valid, phase, pass) in [
            (true, true, Phase::PollRequest, true),
            (true, false, Phase::PollRequest, false),
            (false, true, Phase::PollRequest, false),
            (true, true, Phase::A2, false),
        ] {
            let mut owner = owner(verified, valid);
            owner.phase = phase;
            owner.backend.current.audio_interfaces[1].1 = 1;
            owner.backend.current.audio_interfaces[2].1 = 1;
            assert_eq!(owner.check(1).is_ok(), pass);
        }
        for field in 0..8 {
            let mut owner = owner(true, true);
            owner.phase = Phase::PollReply;
            owner.backend.current.audio_interfaces[1].1 = 1;
            owner.backend.current.audio_interfaces[2].1 = 1;
            match field {
                0 => owner.backend.current.instance += 1,
                1 => owner.backend.current.configuration = 2,
                2 => owner.backend.current.alternate = 0,
                3 => owner.backend.current.kernel_bound = true,
                4 => owner.backend.current.audio_interfaces[0].1 = 1,
                5 => owner.backend.current.audio_interfaces[1].2 = "foreign".into(),
                6 => owner.backend.current.audio_interfaces[2].1 = 0,
                _ => owner.backend.current.descriptors[12] ^= 1,
            }
            assert!(owner.check(1).is_err());
            assert_eq!(owner.backend.validations, 0);
        }
    }
}
