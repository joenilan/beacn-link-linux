//! Selected/pinned USB gate; handshake-only by default, optional one publication/poll.
use crate::{
    session::{Transport, TransportError, IN_PIPE, OUT_PIPE},
    usb_descriptors, MAX_FRAME,
};
use std::{fmt, time::Duration};

pub const MAX_TIMEOUT: Duration = Duration::from_millis(500);
/// One explicit application snapshot and its canonical initial list. Zero is
/// retained opaquely; this does not assign a route or infer an unroute command.
pub struct Publication {
    records: Vec<crate::Record>,
    frame: Vec<u8>,
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
        Ok(Self { records, frame })
    }
    pub fn records(&self) -> &[crate::Record] {
        &self.records
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
    fn claim_three(&mut self) -> Result<(), i32>;
    fn select_one(&mut self) -> Result<(), i32>;
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
    PollRequest,
    PollReply,
    Complete,
    Halted,
}
pub struct Handshake<B: Backend> {
    backend: B,
    baseline: Option<Observation>,
    claimed: bool,
    closed: bool,
    phase: Phase,
    publication: Option<Publication>,
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
    if timeout.is_zero() || timeout > MAX_TIMEOUT {
        return None;
    }
    Some(timeout.as_nanos().div_ceil(1_000_000) as u32)
}
impl<B: Backend> Handshake<B> {
    /// Mutation-bearing setup. Caller must have explicit authority for claim,
    /// alternate selection, two writes/read and release restoration.
    pub fn prepare(backend: B) -> Result<Self, SetupFailure> {
        let mut this = Self {
            backend,
            baseline: None,
            claimed: false,
            closed: false,
            phase: Phase::A0,
            publication: None,
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
            this.backend.select_one().map_err(|code| Error::Backend {
                stage: "alternate",
                code,
            })?;
            this.check(1)
        })();
        match result {
            Ok(()) => Ok(this),
            Err(error) => {
                let cleanup = this.cleanup();
                Err(SetupFailure { error, cleanup })
            }
        }
    }
    /// Explicit opt-in to one exact list and one poll after a valid handshake.
    /// The supplied inventory is caller-observed, not proof of OS freshness.
    pub fn prepare_publication(backend: B, publication: Publication) -> Result<Self, SetupFailure> {
        Self::prepare(backend).map(|mut owner| {
            owner.publication = Some(publication);
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
            return Err(Error::Changed);
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
        let Some(ms) = timeout_ms(timeout) else {
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
                    if self.publication.is_some() {
                        Phase::Publish
                    } else {
                        Phase::Complete
                    }
                } else if phase == Phase::PollReply
                    && n <= MAX_FRAME
                    && crate::session::decode_poll_reply(&bytes[..n]).is_ok()
                {
                    Phase::Complete
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
