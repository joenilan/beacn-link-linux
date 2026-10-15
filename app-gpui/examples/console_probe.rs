//! Read-only Console layout check; never constructs a physical session owner.
#[cfg(all(feature = "ui-qualification", target_os = "linux"))]
fn main() {
    use std::path::PathBuf;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "--read-only-console-preview");
    let root = PathBuf::from(&args[1]);
    std::fs::create_dir(&root).expect("exclusive evidence");
    unsafe {
        std::env::set_var("LINX_PRIVATE_GUI_EVIDENCE", &root);
    }
    beacn_linx_gpui::ui::run_qualification(
        beacn_linx_core::worker::Connection::Desktop,
        beacn_linx_gpui::qualification::Configuration {
            first: args[2].clone(),
            second: String::new(),
            root,
            studio: false,
            managed: false,
            native_playback: false,
            studio_recovery: false,
            desktop: false,
            console: true,
        },
    );
}
#[cfg(not(all(feature = "ui-qualification", target_os = "linux")))]
fn main() {
    eprintln!("requires Linux ui-qualification feature");
    std::process::exit(2);
}
