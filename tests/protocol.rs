use beacn_link_protocol::*;

fn record(value: u8, name: &[u8]) -> Record {
    Record {
        assignment: Assignment::from_byte(value),
        name: name.to_vec(),
    }
}
fn from_hex(text: &str) -> Vec<u8> {
    let s: String = text.split_whitespace().collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn fixture() -> Vec<u8> {
    from_hex(include_str!("../fixtures/ascii.frame.hex"))
}

#[test]
fn synthetic_ascii_roundtrip_and_opaque_zero() {
    let records = vec![record(1, b"game"), record(0, b"voice")];
    assert_eq!(records[0].assignment.known_link(), Some(1));
    assert_eq!(records[1].assignment.known_link(), None);
    let payload = encode_records(&records).unwrap();
    let bytes = encode_frame(&Frame::Data(payload.clone())).unwrap();
    assert_eq!(bytes, fixture());
    assert_eq!(decode_frame(&bytes), Ok(Frame::Data(payload.clone())));
    assert_eq!(decode_records(&payload), Ok(records));
    assert_eq!(encode_records(&[]), Ok(vec![0]));
    assert_eq!(decode_records(&[0]), Ok(vec![]));
}
#[test]
fn observed_control_headers_roundtrip_without_semantics() {
    for value in [
        ControlWord::A0,
        ControlWord::A1,
        ControlWord::A2,
        ControlWord::Ac01,
    ] {
        let header = Header::Control(value);
        let bytes = header.encode().unwrap();
        assert_eq!(Header::decode(&bytes), Ok(header));
        assert_eq!(encode_frame(&Frame::Control(value)), Ok(bytes.to_vec()));
        assert_eq!(decode_frame(&bytes), Ok(Frame::Control(value)));
    }
    assert_eq!(
        Header::Control(ControlWord::A0).encode().unwrap().to_vec(),
        from_hex(include_str!("../fixtures/a0.header.hex"))
    );
    let mut response = vec![0; 512];
    response[..4].copy_from_slice(&0xA2000000u32.to_le_bytes());
    assert!(matches!(
        decode_frame(&response),
        Err(Error::TrailingData {
            expected: 4,
            actual: 512
        })
    ));
}
#[test]
fn every_truncated_frame_prefix_is_rejected() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        assert!(
            matches!(decode_frame(&bytes[..length]), Err(Error::Truncated { .. })),
            "prefix {length}"
        );
    }
    assert!(matches!(
        Header::decode(&[0, 0, 0]),
        Err(Error::Truncated { .. })
    ));
    assert!(matches!(decode_records(&[4]), Err(Error::Truncated { .. })));
    assert!(matches!(
        decode_records(&[4, 1, b'g']),
        Err(Error::Truncated { .. })
    ));
    assert_eq!(decode_records(&[]), Err(Error::MissingTerminator));
    assert_eq!(decode_records(&[1, 1, b'g']), Err(Error::MissingTerminator));
}
#[test]
fn invalid_unsupported_and_rejected_headers() {
    assert_eq!(
        Header::decode(&0xAB000001u32.to_le_bytes()),
        Err(Error::InvalidHeader { word: 0xAB000001 })
    );
    assert_eq!(
        Header::decode(&0xAC020001u32.to_le_bytes()),
        Err(Error::UnsupportedHeader { word: 0xAC020001 })
    );
    assert_eq!(
        Header::decode(&0xACFFFFFFu32.to_le_bytes()),
        Err(Error::RejectedHeader)
    );
    for length in [0u16, 509, 65535] {
        assert_eq!(
            Header::Data { length }.encode(),
            Err(Error::PayloadLength {
                length: length as usize
            })
        );
        assert_eq!(
            Header::decode(&(0xAC000000 | u32::from(length)).to_le_bytes()),
            Err(Error::PayloadLength {
                length: length as usize
            })
        );
    }
}
#[test]
fn exact_frame_and_record_lengths_reject_trailing_data() {
    let mut bytes = fixture();
    bytes.push(0);
    assert!(matches!(
        decode_frame(&bytes),
        Err(Error::TrailingData { .. })
    ));
    assert!(matches!(
        decode_records(&[0, 0]),
        Err(Error::TrailingData {
            expected: 1,
            actual: 2
        })
    ));
    assert!(matches!(
        Header::decode(&[1, 0, 0, 0xAC, 0]),
        Err(Error::TrailingData { .. })
    ));
    let mut shortened = fixture();
    shortened[0] -= 1;
    assert!(matches!(
        decode_frame(&shortened),
        Err(Error::TrailingData { .. })
    ));
}
#[test]
fn conservative_payload_and_name_boundaries() {
    let records = vec![record(1, &vec![b'a'; 255]), record(4, &vec![b'b'; 248])];
    let payload = encode_records(&records).unwrap();
    assert_eq!(payload.len(), MAX_PAYLOAD);
    let frame = encode_frame(&Frame::Data(payload.clone())).unwrap();
    assert_eq!(frame.len(), MAX_FRAME);
    assert_eq!(decode_frame(&frame), Ok(Frame::Data(payload.clone())));
    assert_eq!(decode_records(&payload), Ok(records));
    assert_eq!(
        encode_records(&[record(1, &vec![b'a'; 255]), record(4, &vec![b'b'; 249])]),
        Err(Error::PayloadLength { length: 509 })
    );
    assert_eq!(
        encode_records(&[record(1, &vec![b'a'; 256])]),
        Err(Error::NameTooLong {
            record: 0,
            length: 256
        })
    );
    assert_eq!(
        encode_records(&[record(1, b"")]),
        Err(Error::EmptyName { record: 0 })
    );
    assert_eq!(
        encode_frame(&Frame::Data(vec![])),
        Err(Error::PayloadLength { length: 0 })
    );
    assert_eq!(
        encode_frame(&Frame::Data(vec![0; 509])),
        Err(Error::PayloadLength { length: 509 })
    );
    assert_eq!(
        decode_records(&vec![0; 509]),
        Err(Error::PayloadLength { length: 509 })
    );
}
#[test]
fn raw_decode_preserves_names_but_ascii_encode_rejects_unsupported_bytes() {
    let frame = from_hex(include_str!("../fixtures/opaque.frame.hex"));
    let Frame::Data(payload) = decode_frame(&frame).unwrap() else {
        panic!("expected data")
    };
    let records = decode_records(&payload).unwrap();
    assert_eq!(records, vec![record(255, &[0xC3, 0xA9])]);
    assert_eq!(records[0].assignment.known_link(), None);
    assert!(matches!(
        encode_records(&records),
        Err(Error::UnsupportedName { .. })
    ));
    for name in [vec![0], vec![0x1B], vec![0x7F], vec![0xFF]] {
        assert!(matches!(
            encode_records(&[record(0, &name)]),
            Err(Error::UnsupportedName { .. })
        ));
    }
    assert!(encode_records(&[record(5, b"Voice App")]).is_ok());
}
#[test]
fn offline_cli_fixture_and_errors() {
    use std::process::Command;
    let exe = env!("CARGO_BIN_EXE_link-protocol");
    let out = Command::new(exe)
        .args(["encode", "1:game", "0:voice"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        include_str!("../fixtures/ascii.frame.hex").trim()
    );
    let out = Command::new(exe)
        .args(["decode", include_str!("../fixtures/ascii.frame.hex").trim()])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("assignment=0 (opaque/unknown)"));
    for args in [
        vec!["decode", "xyz"],
        vec!["decode", "0"],
        vec!["encode", "1:"],
        vec!["encode", "256:game"],
        vec!["encode", "1:voix\u{e9}"],
    ] {
        let out = Command::new(exe).args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
}
