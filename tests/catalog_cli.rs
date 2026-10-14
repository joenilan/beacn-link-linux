use std::process::Command;

#[test]
fn help_and_unknown_apply_mode_do_not_open_a_native_connection() {
    let binary = env!("CARGO_BIN_EXE_beacn-linx");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("Read-only commands"));
    let apply = Command::new(binary).arg("--apply").output().unwrap();
    assert_eq!(apply.status.code(), Some(2));
}

#[cfg(not(all(feature = "native-pipewire", target_os = "linux")))]
#[test]
fn default_binary_refuses_os_discovery_without_native_feature() {
    let output = Command::new(env!("CARGO_BIN_EXE_beacn-linx"))
        .arg("inventory")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("apply disabled"));
}
