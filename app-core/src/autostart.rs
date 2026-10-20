//! Launch at login through the desktop's standard autostart directory
//! (`$XDG_CONFIG_HOME/autostart`). The entry's presence is the setting.
use std::{fs, io::Write, path::PathBuf};

const FILE: &str = "io.github.linx.desktop";

fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .filter(|p| p.is_absolute())
}

pub fn path() -> Option<PathBuf> {
    config_home().map(|c| c.join("autostart").join(FILE))
}

/// Whether LinX's own autostart entry exists and is not disabled.
pub fn enabled() -> bool {
    path()
        .and_then(|p| fs::read_to_string(p).ok())
        .is_some_and(|text| is_ours(&text) && !text.contains("Hidden=true"))
}

/// Whether the login entry starts LinX headless (no window, no interface).
pub fn headless() -> bool {
    path()
        .and_then(|p| fs::read_to_string(p).ok())
        .is_some_and(|text| is_ours(&text) && text.lines().any(|l| l.starts_with("Exec=") && l.contains(" --headless")))
}

fn is_ours(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "X-LinX-Autostart=1")
}

/// Quote one argument for a desktop entry `Exec` line.
fn quote(arg: &str) -> String {
    if !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=+:,@".contains(c))
    {
        return arg.to_owned();
    }
    let mut out = String::from("\"");
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        if c == '%' {
            out.push('%');
        }
        out.push(c);
    }
    out.push('"');
    out
}

pub fn entry(program: &str, args: &[String]) -> String {
    let exec = std::iter::once(program.to_owned())
        .chain(args.iter().cloned())
        .map(|a| quote(&a))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "[Desktop Entry]\nType=Application\nName=LinX\nComment=Unofficial BEACN Link for Linux\nExec={exec}\nIcon=io.github.linx\nTerminal=false\nX-GNOME-Autostart-enabled=true\nX-LinX-Autostart=1\n"
    )
}

/// Create or remove the entry. `args` are the launch arguments to use at
/// login (LinX adds `--start-hidden`). Another program's file of the same
/// name is never overwritten or removed.
pub fn set(enable: bool, args: &[String]) -> Result<(), String> {
    let path = path().ok_or("No user configuration directory.")?;
    if let Ok(existing) = fs::read_to_string(&path) {
        if !is_ours(&existing) {
            return Err("Another autostart entry with LinX's name exists; left unchanged.".into());
        }
    }
    if !enable {
        return match fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let program = std::env::current_exe().map_err(|e| e.to_string())?;
    let program = program.to_str().ok_or("LinX's path is not valid text.")?;
    let mut args = args.to_vec();
    if !args.iter().any(|a| a == "--start-hidden" || a == "--headless") {
        args.push("--start-hidden".into());
    }
    let parent = path.parent().ok_or("No autostart directory.")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".{FILE}.{}", std::process::id()));
    let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(entry(program, &args).as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(&temp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entry_quotes_paths_and_marks_ownership() {
        let text = entry(
            "/home/me/My Apps/linx",
            &["--studio".into(), "--start-hidden".into(), "50%".into()],
        );
        assert!(text.contains("Exec=\"/home/me/My Apps/linx\" --studio --start-hidden \"50%%\"\n"));
        assert!(is_ours(&text));
        assert!(!is_ours("[Desktop Entry]\nName=LinX\n"));
    }
}
