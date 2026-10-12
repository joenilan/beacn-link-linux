use beacn_link_protocol::{
    decode_frame, decode_records, encode_frame, encode_records, Assignment, Frame, Header, Record,
    MAX_FRAME,
};
use beacn_link_protocol::{planner, snapshot};
use std::{env, io::Read, process::ExitCode};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if digits.len() > MAX_FRAME * 2 {
        return Err("hex input exceeds 512-byte limit".into());
    }
    if digits.len() % 2 != 0 {
        return Err("hex input has an odd digit count".into());
    }
    fn nibble(b: u8) -> Result<u8, String> {
        match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err("invalid hex digit (no 0x prefix)".into()),
        }
    }
    digits
        .chunks_exact(2)
        .map(|p| Ok(nibble(p[0])? * 16 + nibble(p[1])?))
        .collect()
}
fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("inspect-usb-descriptors") if args.len() == 2 => {
            use beacn_link_protocol::usb_descriptors;
            let active = args[1]
                .parse::<u8>()
                .map_err(|_| "active configuration must be a decimal byte")?;
            let mut input = Vec::new();
            std::io::stdin()
                .take((usb_descriptors::MAX_DESCRIPTOR_BYTES + 1) as u64)
                .read_to_end(&mut input)
                .map_err(|_| "cannot read descriptor stdin")?;
            let descriptors = usb_descriptors::parse(&input).map_err(|e| e.to_string())?;
            println!("OFFLINE DESCRIPTORS: VID={:04X} PID={:04X} revision={:04X} USB={:04X}; no device access", descriptors.vendor, descriptors.product, descriptors.revision, descriptors.usb_version);
            for config in &descriptors.configurations {
                for interface in &config.interfaces {
                    println!("config={} interface={} alternate={} class={:02X} subclass={:02X} protocol={:02X} endpoints={:?}", config.value, interface.number, interface.alternate, interface.class, interface.subclass, interface.protocol, interface.endpoints);
                }
            }
            let candidate = descriptors.candidate(active).map_err(|e| e.to_string())?;
            println!(
                "STATIC CANDIDATE ONLY: {candidate:?}; no claim, transfer or routing authority"
            );
        }
        Some("decode-header") if args.len() == 2 => {
            let bytes = parse_hex(&args[1])?;
            let header = Header::decode(&bytes).map_err(|e| e.to_string())?;
            println!("{header:?} (static subset; control semantics unknown)");
        }
        Some("decode") if args.len() == 2 => {
            let bytes = parse_hex(&args[1])?;
            match decode_frame(&bytes).map_err(|e| e.to_string())? {
                Frame::Control(word) => println!(
                    "header-only control 0x{:08X}; semantics unknown",
                    word.word()
                ),
                Frame::Data(payload) => {
                    let records = decode_records(&payload).map_err(|e| e.to_string())?;
                    println!(
                        "data: {} payload bytes, {} records",
                        payload.len(),
                        records.len()
                    );
                    for record in records {
                        let label = record
                            .assignment
                            .known_link()
                            .map(|n| format!("Link {n}"))
                            .unwrap_or_else(|| "opaque/unknown".into());
                        let escaped: String = record
                            .name
                            .iter()
                            .flat_map(|b| std::ascii::escape_default(*b))
                            .map(char::from)
                            .collect();
                        println!(
                            "assignment={} ({label}) name=\"{escaped}\" name_hex={}",
                            record.assignment.byte(),
                            hex(&record.name)
                        );
                    }
                }
            }
        }
        Some("plan-snapshot") if args.len() == 3 => {
            let snapshot_id = args[1]
                .parse::<u64>()
                .map_err(|_| "snapshot ID must be u64")?;
            let frame = decode_frame(&parse_hex(&args[2])?).map_err(|e| e.to_string())?;
            let Frame::Data(payload) = frame else {
                return Err("planning requires a data frame, not a control header".into());
            };
            let records = decode_records(&payload).map_err(|e| e.to_string())?;
            let mut input = Vec::new();
            let mut reader = std::io::stdin().take((snapshot::MAX_TSV_BYTES + 1) as u64);
            reader
                .read_to_end(&mut input)
                .map_err(|e| format!("inventory stdin: {e}"))?;
            if input.len() > snapshot::MAX_TSV_BYTES {
                return Err("inventory exceeds 2 MiB limit".into());
            }
            let text = std::str::from_utf8(&input).map_err(|_| "inventory stdin must be UTF-8")?;
            let inventory = snapshot::inventory_from_tsv(snapshot_id, text)?;
            let proposed = planner::plan(&inventory, &records).map_err(|e| e.to_string())?;
            println!("{proposed:#?}");
        }
        Some("prepare-graph") if args.len() == 3 => {
            let generation = args[1]
                .parse::<u64>()
                .map_err(|_| "generation must be u64")?;
            let Frame::Data(payload) =
                decode_frame(&parse_hex(&args[2])?).map_err(|e| e.to_string())?
            else {
                return Err("preparation requires a data frame".into());
            };
            let records = decode_records(&payload).map_err(|e| e.to_string())?;
            let mut input = Vec::new();
            std::io::stdin()
                .take((beacn_link_protocol::graph_snapshot::MAX_GRAPH_TSV_BYTES + 1) as u64)
                .read_to_end(&mut input)
                .map_err(|e| format!("graph stdin: {e}"))?;
            let text = std::str::from_utf8(&input).map_err(|_| "graph stdin must be UTF-8")?;
            let data = beacn_link_protocol::graph_snapshot::preparation_from_tsv(generation, text)?;
            let mut tracker = beacn_link_protocol::lifecycle::InventoryTracker::default();
            tracker
                .advance(&data.inventory)
                .map_err(|e| e.to_string())?;
            let plan = planner::plan(&data.inventory, &records).map_err(|e| e.to_string())?;
            let prepared = beacn_link_protocol::executor::prepare(
                &tracker,
                &plan,
                &data.expected,
                &data.observed,
                &data.destinations,
                &data.ownership,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "DRY RUN: {} guarded operations; actual application disabled",
                prepared.steps.len()
            );
            for (index, step) in prepared.steps.iter().enumerate() {
                println!("{index}: {:?}", step.operation);
            }
        }
        Some("apply") | Some("--apply") => {
            return Err(
                "route application is disabled; only offline dry-run preparation is supported"
                    .into(),
            )
        }
        Some("encode") => {
            let mut records = Vec::new();
            for arg in &args[1..] {
                let (value, name) = arg.split_once(':').ok_or("expected assignment:name")?;
                let assignment = value
                    .parse::<u8>()
                    .map_err(|_| "assignment must be a byte (0..255)")?;
                records.push(Record {
                    assignment: Assignment::from_byte(assignment),
                    name: name.as_bytes().to_vec(),
                });
            }
            let payload = encode_records(&records).map_err(|e| e.to_string())?;
            let bytes = encode_frame(&Frame::Data(payload)).map_err(|e| e.to_string())?;
            println!("{}", hex(&bytes));
        }
        Some("--help") | Some("-h") if args.len() == 1 => {
            println!("Offline static-subset codec. All commands are dry runs. Route application disabled.\n\
Usage:\n  link-protocol decode <hex-frame>\n  link-protocol decode-header <8-hex-digits>\n  link-protocol encode [assignment:name ...]\n  link-protocol plan-snapshot <snapshot-id> <data-frame-hex> < inventory.tsv\n  link-protocol prepare-graph <generation> <data-frame-hex> < graph.tsv\n  link-protocol inspect-usb-descriptors <active-config-value> < cached-descriptors.bin\n\
Hex may contain quoted whitespace. Encoding requires printable ASCII names.\n\
Assignments 0 and 5..255 are opaque; zero is not interpreted as unroute.\n\
Data frames must be exact-sized and contain terminated assignment records.\n\
Control decoding is header-only, not a handshake implementation.");
        }
        _ => return Err("invalid arguments; use --help".into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}
