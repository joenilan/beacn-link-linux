//! A single LinX preference. Never reads or writes BEACN/desktop settings.
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct Preferences {
    path: PathBuf,
    previous: Option<Vec<u8>>,
}
impl Preferences {
    pub fn user_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))?;
        base.is_absolute().then(|| base.join("linx/preferences"))
    }
    pub fn load(path: PathBuf) -> Result<(Self, bool), String> {
        let previous = read(&path)?;
        let enabled = match previous.as_deref() {
            None | Some(b"linx-preferences=1\nauto-connect=true\n") => true,
            Some(b"linx-preferences=1\nauto-connect=false\n") => false,
            _ => {
                return Err(
                    "LinX preferences have an unsupported format; preserved unchanged.".into(),
                )
            }
        };
        Ok((Self { path, previous }, enabled))
    }
    pub fn save(&mut self, enabled: bool) -> Result<(), String> {
        if read(&self.path)? != self.previous {
            return Err("LinX preferences changed elsewhere; reopen LinX before saving.".into());
        }
        let parent = self.path.parent().ok_or("No LinX settings directory.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let data = format!("linx-preferences=1\nauto-connect={enabled}\n").into_bytes();
        let temp = parent.join(format!(
            ".preferences-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp).map_err(|e| e.to_string())?;
        let result = (|| {
            file.write_all(&data).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            if read(&self.path)? != self.previous {
                return Err("LinX preferences changed while saving.".into());
            }
            fs::rename(&temp, &self.path).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result?;
        self.previous = Some(data);
        Ok(())
    }
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    if let Some(parent) = path.parent() {
        match fs::symlink_metadata(parent) {
            Ok(m) if !m.is_dir() || m.file_type().is_symlink() => {
                return Err("LinX settings directory is not a regular directory.".into())
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
            _ => {}
        }
    }
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() <= 4096 => {}
        _ => return Err("LinX preferences cannot be safely read; preserved unchanged.".into()),
    }
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4096 {
        return Err("LinX preferences exceed the supported size.".into());
    }
    Ok(Some(bytes))
}
