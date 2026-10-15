use beacn_linx_core::preferences::Preferences;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "linx-prefs-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join("linx/preferences")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn default_and_toggle_survive_reopen() {
    let fixture = Fixture::new();
    let (mut store, enabled) = Preferences::load(fixture.path()).unwrap();
    assert!(enabled);
    assert!(!fixture.path().exists()); // viewing never writes
    store.save(false).unwrap();
    let (mut reopened, enabled) = Preferences::load(fixture.path()).unwrap();
    assert!(!enabled);
    reopened.save(true).unwrap();
    assert!(Preferences::load(fixture.path()).unwrap().1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(fixture.path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn unknown_settings_and_external_changes_are_preserved() {
    let fixture = Fixture::new();
    let (mut store, _) = Preferences::load(fixture.path()).unwrap();
    fs::create_dir_all(fixture.path().parent().unwrap()).unwrap();
    fs::write(fixture.path(), "future settings\n").unwrap();
    assert!(store.save(false).is_err());
    assert!(Preferences::load(fixture.path()).is_err());
    assert_eq!(
        fs::read_to_string(fixture.path()).unwrap(),
        "future settings\n"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_file_and_app_directory_are_refused() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join("linx")).unwrap();
    let target = fixture.0.join("other");
    fs::write(&target, "keep").unwrap();
    symlink(&target, fixture.path()).unwrap();
    assert!(Preferences::load(fixture.path()).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep");
    fs::remove_file(fixture.path()).unwrap();
    fs::remove_dir(fixture.0.join("linx")).unwrap();
    fs::create_dir(fixture.0.join("elsewhere")).unwrap();
    symlink(fixture.0.join("elsewhere"), fixture.0.join("linx")).unwrap();
    assert!(Preferences::load(fixture.path()).is_err());
}
