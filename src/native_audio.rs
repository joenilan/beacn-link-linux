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
    if field(status, "state")? != "RUNNING"
        || field(status, "owner_pid")?.parse::<u32>().map_err(|_| ())? != pid
    {
        return Err(());
    }
    for (name, expected) in [("format", "FLOAT_LE"), ("channels", "8"), ("rate", "96000")] {
        if field(params, name)?.split_whitespace().next() != Some(expected) {
            return Err(());
        }
    }
    field(status, "hw_ptr")?.parse().map_err(|_| ())
}

pub(crate) struct PinnedPlayback {
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
        if generation(self.pid)? != self.start
            || identity(&self.device)? != self.stamp
            || text(&self.capture)?.trim() != "closed"
        {
            return Err(());
        }
        let first = pcm(
            &text(&self.pcm.join("status"))?,
            &text(&self.pcm.join("hw_params"))?,
            self.pid,
        )?;
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
        let last = pcm(
            &text(&self.pcm.join("status"))?,
            &text(&self.pcm.join("hw_params"))?,
            self.pid,
        )?;
        if !found
            || last < first
            || generation(self.pid)? != self.start
            || identity(&self.device)? != self.stamp
        {
            return Err(());
        }
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
    pub(crate) fn stats(&self) -> (u32, u64) {
        (
            self.checks,
            self.last
                .zip(self.first)
                .map_or(0, |((last, _), first)| last.saturating_sub(first)),
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
        assert!(proof.validate().is_ok());
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
