//! Read-only proof for one explicitly authorized native PCM owner. No ALSA opens.
use std::{
    fs::{self, File},
    io::Read,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

type Failure = ();
fn text(path: &Path) -> Result<String, Failure> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| ())?
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > 16384 {
        return Err(());
    }
    String::from_utf8(bytes).map_err(|_| ())
}
fn identity(path: &Path) -> Result<(u64, u64, u64), Failure> {
    let m = fs::metadata(path).map_err(|_| ())?;
    if !m.file_type().is_char_device() {
        return Err(());
    }
    Ok((m.dev(), m.ino(), m.rdev()))
}
fn generation(pid: u32) -> Result<u64, Failure> {
    let data = text(&PathBuf::from(format!("/proc/{pid}/stat")))?;
    data.rsplit_once(')')
        .ok_or(())?
        .1
        .split_whitespace()
        .nth(19)
        .ok_or(())?
        .parse()
        .map_err(|_| ())
}
fn field<'a>(data: &'a str, name: &str) -> Result<&'a str, Failure> {
    if data.len() > 16384 {
        return Err(());
    }
    let mut values = data
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, _)| key.trim() == name);
    let value = values.next().ok_or(())?.1.trim();
    if values.next().is_some() || value.is_empty() {
        return Err(());
    }
    Ok(value)
}
fn pcm(status: &str, params: &str, pid: u32) -> Result<u64, Failure> {
    pcm_state(status, params, pid, false).map(|(_, pointer)| pointer)
}
fn pcm_state(status: &str, params: &str, pid: u32, starting: bool) -> Result<(bool, u64), Failure> {
    let state = field(status, "state")?;
    if (state != "RUNNING" && !(starting && state == "PREPARED"))
        || field(status, "owner_pid")?.parse::<u32>().map_err(|_| ())? != pid
    {
        return Err(());
    }
    for (name, expected) in [("format", "FLOAT_LE"), ("channels", "8"), ("rate", "96000")] {
        if field(params, name)?.split_whitespace().next() != Some(expected) {
            return Err(());
        }
    }
    Ok((
        state == "RUNNING",
        field(status, "hw_ptr")?.parse().map_err(|_| ())?,
    ))
}

pub(crate) struct PinnedPlayback {
    #[cfg(feature = "native-playback")]
    card: u32,
    pid: u32,
    start: u64,
    device: PathBuf,
    stamp: (u64, u64, u64),
    pcm: PathBuf,
    capture: PathBuf,
    last: Option<(u64, Instant)>,
    first: Option<u64>,
    checks: u32,
}
impl PinnedPlayback {
    pub(crate) fn pin(sysfs: &Path, pid: u32) -> Result<Self, Failure> {
        if pid == 0 {
            return Err(());
        }
        let selected = sysfs.canonicalize().map_err(|_| ())?;
        let mut cards = Vec::new();
        for entry in fs::read_dir("/sys/class/sound").map_err(|_| ())? {
            let entry = entry.map_err(|_| ())?;
            let name = entry.file_name();
            let Some(card) = name
                .to_str()
                .and_then(|n| n.strip_prefix("card"))
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            if entry
                .path()
                .join("device")
                .canonicalize()
                .is_ok_and(|p| p.starts_with(&selected))
            {
                cards.push(card);
            }
        }
        if cards.len() != 1 {
            return Err(());
        }
        let card = cards[0];
        let pcm = PathBuf::from(format!("/proc/asound/card{card}/pcm0p/sub0"));
        let capture = PathBuf::from(format!("/proc/asound/card{card}/pcm0c/sub0/status"));
        if text(&pcm.join("status"))?.trim() != "closed" || text(&capture)?.trim() != "closed" {
            return Err(());
        }
        let device = PathBuf::from(format!("/dev/snd/pcmC{card}D0p"));
        let start = generation(pid)?;
        let stamp = identity(&device)?;
        if generation(pid)? != start {
            return Err(());
        }
        Ok(Self {
            #[cfg(feature = "native-playback")]
            card,
            pid,
            start,
            device,
            stamp,
            pcm,
            capture,
            last: None,
            first: None,
            checks: 0,
        })
    }
    pub(crate) fn validate(&mut self) -> Result<(), Failure> {
        let last = self.observe(false)?.ok_or(())?;
        self.advance_clock(last)
    }
    /// One creation-time PREPARED -> RUNNING transition, with full identity,
    /// format and held-device proof even while pending. Never used by refresh.
    #[cfg(feature = "native-playback")]
    pub(crate) fn startup_probe(&mut self) -> Result<bool, Failure> {
        if self.last.is_some() || self.checks != 0 {
            return Err(());
        }
        match self.observe(true)? {
            Some(last) => {
                self.advance_clock(last)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
    fn observe(&self, starting: bool) -> Result<Option<u64>, Failure> {
        if generation(self.pid)? != self.start
            || identity(&self.device)? != self.stamp
            || text(&self.capture)?.trim() != "closed"
        {
            return Err(());
        }
        let sample = || {
            let status = text(&self.pcm.join("status"))?;
            let params = text(&self.pcm.join("hw_params"))?;
            if starting {
                pcm_state(&status, &params, self.pid, true)
            } else {
                pcm(&status, &params, self.pid).map(|pointer| (true, pointer))
            }
        };
        let first = sample()?;
        let mut found = false;
        for (count, entry) in fs::read_dir(format!("/proc/{}/fd", self.pid))
            .map_err(|_| ())?
            .enumerate()
        {
            if count >= 256 {
                return Err(());
            }
            let entry = entry.map_err(|_| ())?;
            found |= identity(&entry.path()).is_ok_and(|stamp| stamp == self.stamp);
        }
        let last = sample()?;
        if !found
            || last.1 < first.1
            || (first.0 && !last.0)
            || generation(self.pid)? != self.start
            || identity(&self.device)? != self.stamp
            || text(&self.capture)?.trim() != "closed"
        {
            return Err(());
        }
        Ok(last.0.then_some(last.1))
    }
    fn advance_clock(&mut self, last: u64) -> Result<(), Failure> {
        let now = Instant::now();
        if let Some((pointer, advanced)) = self.last {
            if last < pointer
                || (last == pointer && now.duration_since(advanced) > Duration::from_secs(2))
            {
                return Err(());
            }
            if last > pointer {
                self.last = Some((last, now));
            }
        } else {
            self.last = Some((last, now));
        }
        self.first.get_or_insert(last);
        self.checks = self
            .checks
            .checked_add(1)
            .filter(|n| *n <= 4096)
            .ok_or(())?;
        Ok(())
    }
    #[cfg(feature = "native-playback")]
    pub(crate) fn card(&self) -> u32 {
        self.card
    }
    #[cfg(feature = "native-playback")]
    pub(crate) fn closed(&self) -> Result<(), Failure> {
        if generation(self.pid)? != self.start
            || identity(&self.device)? != self.stamp
            || text(&self.pcm.join("status"))?.trim() != "closed"
            || text(&self.capture)?.trim() != "closed"
        {
            return Err(());
        }
        Ok(())
    }
    pub(crate) fn stats(&self) -> (u32, u64) {
        (
            self.checks,
            self.last
                .zip(self.first)
                .map_or(0, |((last, _), first)| last.saturating_sub(first)),
        )
    }
    /// Best-effort metadata at a rejected proof, never authority to accept it.
    /// A second observation can differ; preserve the original failed decision.
    #[cfg(feature = "native-playback")]
    pub(crate) fn diagnostic(&self) -> String {
        let status = text(&self.pcm.join("status"));
        let params = text(&self.pcm.join("hw_params"));
        let value = |data: &Result<String, Failure>, key| {
            data.as_ref()
                .ok()
                .and_then(|s| field(s, key).ok())
                .map(|s| s.chars().take(80).collect::<String>())
                .unwrap_or_else(|| "unavailable".into())
        };
        let state = if status.as_deref().is_ok_and(|s| s.trim() == "closed") {
            "closed".into()
        } else {
            value(&status, "state")
        };
        let mut fd_count = 0;
        let mut held = false;
        let fd_read = if let Ok(entries) = fs::read_dir(format!("/proc/{}/fd", self.pid)) {
            for entry in entries.take(257) {
                fd_count += 1;
                if let Ok(entry) = entry {
                    held |= identity(&entry.path()).is_ok_and(|stamp| stamp == self.stamp);
                }
            }
            true
        } else {
            false
        };
        format!(
            "metadata recheck: state={state:?} owner={:?} expected_owner={} format={:?} channels={:?} rate={:?} server_generation={} pcm_identity={} capture_closed={} fd_read={} fd_count={} pcm_fd_held={}",
            value(&status, "owner_pid"), self.pid, value(&params, "format"),
            value(&params, "channels"), value(&params, "rate"),
            generation(self.pid).is_ok_and(|start| start == self.start),
            identity(&self.device).is_ok_and(|stamp| stamp == self.stamp),
            text(&self.capture).is_ok_and(|s| s.trim() == "closed"),
            fd_read, fd_count, held,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_proof_rejects_replaced_generation_inode_owner_closed_fd_and_stalled_clock() {
        let pid = std::process::id();
        let folder = std::env::temp_dir().join(format!(
            "beacn-audio-proof-{pid}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&folder).unwrap();
        let status = folder.join("status");
        let params = folder.join("hw_params");
        let capture = folder.join("capture");
        fs::write(
            &status,
            format!("state: RUNNING\nowner_pid : {pid}\nhw_ptr : 96000\n"),
        )
        .unwrap();
        fs::write(&params, "format: FLOAT_LE\nchannels: 8\nrate: 96000\n").unwrap();
        fs::write(&capture, "closed\n").unwrap();
        let held = File::open("/dev/zero").unwrap(); // Metadata/FD proof only; no reads.
        let device = PathBuf::from("/dev/zero");
        let mut proof = PinnedPlayback {
            #[cfg(feature = "native-playback")]
            card: 0,
            pid,
            start: generation(pid).unwrap(),
            stamp: identity(&device).unwrap(),
            device,
            pcm: folder.clone(),
            capture: capture.clone(),
            last: None,
            first: None,
            checks: 0,
        };
        #[cfg(feature = "native-playback")]
        {
            let prepared = format!("state: PREPARED\nowner_pid : {pid}\nhw_ptr : 0\n");
            fs::write(&status, &prepared).unwrap();
            assert_eq!(proof.startup_probe(), Ok(false));
            assert_eq!(proof.stats(), (0, 0));
            assert!(proof.validate().is_err(), "PREPARED is not running proof");
            fs::write(
                &status,
                prepared.replace(&pid.to_string(), &(pid + 1).to_string()),
            )
            .unwrap();
            assert!(proof.startup_probe().is_err(), "wrong owner is not pending");
            fs::write(&status, &prepared).unwrap();
            fs::write(&params, "format: S16_LE\nchannels: 8\nrate: 96000\n").unwrap();
            assert!(
                proof.startup_probe().is_err(),
                "wrong format is not pending"
            );
            fs::write(&params, "format: FLOAT_LE\nchannels: 8\nrate: 96000\n").unwrap();
            fs::write(&capture, "state: RUNNING\n").unwrap();
            assert!(proof.startup_probe().is_err(), "capture must remain closed");
            fs::write(&capture, "closed\n").unwrap();
            proof.start += 1;
            assert!(proof.startup_probe().is_err());
            proof.start -= 1;
            fs::write(
                &status,
                format!("state: RUNNING\nowner_pid : {pid}\nhw_ptr : 96000\n"),
            )
            .unwrap();
            assert_eq!(proof.startup_probe(), Ok(true));
            assert!(
                proof.startup_probe().is_err(),
                "startup cannot be reentered"
            );
        }
        assert!(proof.validate().is_ok());
        #[cfg(feature = "native-playback")]
        {
            let diagnostic = proof.diagnostic();
            assert!(diagnostic.contains("state=\"RUNNING\""));
            assert!(diagnostic.contains("format=\"FLOAT_LE\""));
            assert!(diagnostic.contains("pcm_fd_held=true"));
            assert!(diagnostic.contains("server_generation=true"));
        }
        proof.start += 1;
        assert!(proof.validate().is_err());
        proof.start -= 1;
        let stamp = proof.stamp;
        proof.stamp = (0, 0, 0);
        assert!(proof.validate().is_err());
        proof.stamp = stamp;
        fs::write(&capture, "state: RUNNING\n").unwrap();
        assert!(proof.validate().is_err());
        fs::write(&capture, "closed\n").unwrap();
        fs::write(
            &status,
            format!("state: RUNNING\nowner_pid : {}\nhw_ptr : 96000\n", pid + 1),
        )
        .unwrap();
        assert!(proof.validate().is_err());
        fs::write(
            &status,
            format!("state: RUNNING\nowner_pid : {pid}\nhw_ptr : 95000\n"),
        )
        .unwrap();
        assert!(proof.validate().is_err());
        fs::write(
            &status,
            format!("state: RUNNING\nowner_pid : {pid}\nhw_ptr : 96000\n"),
        )
        .unwrap();
        proof.last = Some((96000, Instant::now() - Duration::from_secs(3)));
        assert!(proof.validate().is_err());
        proof.last = None;
        drop(held);
        assert!(proof.validate().is_err());
        #[cfg(feature = "native-playback")]
        {
            proof.last = None;
            proof.checks = 0;
            fs::write(
                &status,
                format!("state: PREPARED\nowner_pid : {pid}\nhw_ptr : 0\n"),
            )
            .unwrap();
            assert!(
                proof.startup_probe().is_err(),
                "pending still needs the PCM FD"
            );
        }
        for path in [status, params, capture] {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn playback_requires_exact_owner_running_format_and_unambiguous_bounded_fields() {
        let status = "state: RUNNING\nowner_pid : 42\nhw_ptr : 96000\n";
        let params = "format: FLOAT_LE\nchannels: 8\nrate: 96000 (96000/1)\n";
        assert_eq!(pcm(status, params, 42), Ok(96000));
        for state in ["PREPARED", "OPEN", "SETUP", "XRUN", "DRAINING"] {
            let value = status.replace("RUNNING", state);
            assert!(pcm(&value, params, 42).is_err());
            assert_eq!(
                pcm_state(&value, params, 42, true).is_ok(),
                state == "PREPARED"
            );
        }
        for invalid in [
            status.replace("42", "43"),
            status.replace("RUNNING", "DRAINING"),
            status.replace("96000", "bad"),
            format!("{status}owner_pid : 42\n"),
            "x".repeat(16385),
        ] {
            assert!(pcm(&invalid, params, 42).is_err());
        }
        for invalid in [
            params.replace("FLOAT_LE", "S16_LE"),
            params.replace("8", "2"),
            params.replace("96000", "48000"),
            format!("{params}rate: 96000\n"),
        ] {
            assert!(pcm(status, &invalid, 42).is_err());
        }
    }
}
