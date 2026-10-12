use beacn_link_protocol::{
    session::{IN_PIPE, OUT_PIPE},
    usb_descriptors::*,
};
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn fixture() -> Vec<u8> {
    include_str!("../fixtures/usb-composite.synthetic.hex")
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}
fn candidate(bytes: &[u8]) -> Result<Candidate, Error> {
    parse(bytes)?.candidate(7)
}
#[test]
fn composite_config_value_and_interface_numbers_are_not_array_indexes() {
    let descriptors = parse(&fixture()).unwrap();
    assert_eq!(
        (
            descriptors.vendor,
            descriptors.product,
            descriptors.revision
        ),
        (VENDOR, PRODUCT, 0x0100)
    );
    let found = descriptors.candidate(7).unwrap();
    assert_eq!(
        (found.configuration, found.interface, found.alternate),
        (7, 3, 1)
    );
    assert_eq!(
        (found.out.address, found.input.address),
        (OUT_PIPE, IN_PIPE)
    );
    assert_eq!(
        (found.out.max_packet_size, found.input.max_packet_size),
        (512, 64)
    );
    for wrong in [0, 1, 8] {
        assert!(descriptors.candidate(wrong).is_err());
    }
    // No strings or raw extension bodies survive in the parsed report.
    assert!(!format!("{descriptors:?}").contains("Serial"));
}
#[test]
fn every_truncated_prefix_and_inconsistent_total_length_is_refused() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        assert!(parse(&bytes[..length]).is_err(), "prefix {length}");
    }
    for length in [0u16, 9, 53, 55, 65535] {
        let mut bytes = fixture();
        bytes[20..22].copy_from_slice(&length.to_le_bytes());
        assert!(parse(&bytes).is_err());
    }
    let mut bytes = fixture();
    bytes.push(0);
    assert!(parse(&bytes).is_err());
}
#[test]
fn malformed_structure_and_counts_fail_before_candidate_selection() {
    for (offset, value) in [
        (0, 17),
        (17, 0),
        (17, 2),
        (18, 8),
        (22, 3),
        (23, 0),
        (27, 8),
        (40, 0),
        (40, 1),
        (49, 8),
        (53, 1),
        (58, 6),
        (60, 0),
        (60, 0x13),
    ] {
        let mut bytes = fixture();
        bytes[offset] = value;
        assert!(parse(&bytes).is_err(), "offset {offset} value {value}");
    }
    let mut duplicate = fixture();
    duplicate[67] = OUT_PIPE;
    assert!(parse(&duplicate).is_err());
    let mut duplicate = fixture();
    duplicate[52] = 0;
    assert!(parse(&duplicate).is_err());
    let mut no_alt_zero = fixture();
    no_alt_zero[43] = 2;
    assert!(parse(&no_alt_zero).is_err());
    let mut orphan = fixture();
    orphan.splice(27..27, [7, 5, 1, 2, 64, 0, 0]);
    assert!(parse(&orphan).is_err());
}
#[test]
fn wrong_device_class_setting_or_endpoint_type_cannot_select_an_audio_fallback() {
    for (offset, value) in [
        (8, 0),
        (10, 0),
        (51, 2),
        (52, 2),
        (54, 1),
        (60, 4),
        (67, 0x84),
        (61, 1),
        (68, 3),
        (61, 0x82),
        (63, 0),
    ] {
        let mut bytes = fixture();
        bytes[offset] = value;
        // Some mutations are structural; all must fail selection.
        assert!(candidate(&bytes).is_err(), "offset {offset} value {value}");
    }
    let mut zero_packet = fixture();
    zero_packet[62..64].fill(0);
    assert!(candidate(&zero_packet).is_err());
    let mut shared = fixture();
    shared[31] = 1;
    shared.splice(36..36, [7, 5, 3, 2, 64, 0, 0]);
    shared[20..22].copy_from_slice(&61u16.to_le_bytes());
    assert!(parse(&shared).is_ok());
    assert!(candidate(&shared)
        .unwrap_err()
        .reason
        .contains("another interface"));
}
#[test]
fn configurations_are_separate_and_never_repaired_or_guessed() {
    let mut bytes = fixture();
    bytes[17] = 2;
    let mut second = fixture()[18..].to_vec();
    second[5] = 8;
    bytes.extend(&second);
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.candidate(8).unwrap().configuration, 8);
    bytes[72 + 5] = 7;
    assert!(parse(&bytes).is_err());
    let mut bytes = fixture();
    bytes[20..22].copy_from_slice(&108u16.to_le_bytes());
    bytes.extend(second);
    assert!(parse(&bytes).is_err()); // Do not trust an oversized total over the next config.
}
#[test]
fn input_and_descriptor_count_are_bounded_and_unknown_extensions_not_retained() {
    assert!(parse(&vec![0; MAX_DESCRIPTOR_BYTES + 1]).is_err());
    let mut bytes = fixture();
    bytes.extend(std::iter::repeat_n([2, 0x24], MAX_DESCRIPTORS).flatten());
    assert!(parse(&bytes).unwrap_err().reason.contains("too many"));
    let mut bytes = fixture();
    bytes.extend([2, 0x24]);
    bytes[20..22].copy_from_slice(&56u16.to_le_bytes());
    assert!(candidate(&bytes).is_ok());
}
#[test]
fn actual_cli_accepts_only_bounded_binary_stdin_and_reports_candidate_limits() {
    fn run(args: &[&str], bytes: &[u8]) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_link-protocol"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        child.wait_with_output().unwrap()
    }
    let good = run(&["inspect-usb-descriptors", "7"], &fixture());
    assert!(good.status.success());
    let text = String::from_utf8(good.stdout).unwrap();
    assert!(text.contains("STATIC CANDIDATE ONLY"));
    assert!(text.contains("no claim, transfer or routing authority"));
    let bad = run(&["inspect-usb-descriptors", "1"], &fixture());
    assert_eq!(bad.status.code(), Some(2));
    let bad = run(
        &["inspect-usb-descriptors", "7"],
        &vec![0; MAX_DESCRIPTOR_BYTES + 1],
    );
    assert_eq!(bad.status.code(), Some(2));
}
