//! Bounded parser for Linux's cached device/configuration descriptor stream.
//! Candidate selection is conservative policy, never authority to open/claim USB.
use crate::session::{IN_PIPE, OUT_PIPE};
use std::{collections::BTreeSet, fmt};

pub const MAX_DESCRIPTOR_BYTES: usize = 256 * 1024;
pub const MAX_DESCRIPTORS: usize = 4096;
pub const VENDOR: u16 = 0x33AE;
pub const PRODUCT: u16 = 0x4003;
pub const INTERFACE: u8 = 3;
pub const ALTERNATE: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub offset: usize,
    pub reason: &'static str,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "USB descriptors at byte {}: {}",
            self.offset, self.reason
        )
    }
}
impl std::error::Error for Error {}
fn error(offset: usize, reason: &'static str) -> Error {
    Error { offset, reason }
}
fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub number: u8,
    pub alternate: u8,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub endpoints: Vec<Endpoint>,
    declared_endpoints: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub value: u8,
    pub interfaces: Vec<Interface>,
    offset: usize,
    declared_length: u16,
    declared_interfaces: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptors {
    pub vendor: u16,
    pub product: u16,
    pub revision: u16,
    pub usb_version: u16,
    pub configurations: Vec<Configuration>,
}

fn finish_configuration(config: &Configuration, end: usize) -> Result<(), Error> {
    if end - config.offset != usize::from(config.declared_length) {
        return Err(error(
            config.offset,
            "configuration total length disagrees with cached bytes",
        ));
    }
    let numbers: BTreeSet<_> = config.interfaces.iter().map(|i| i.number).collect();
    if numbers.len() != usize::from(config.declared_interfaces) {
        return Err(error(
            config.offset,
            "interface count disagrees with descriptors",
        ));
    }
    for interface in &config.interfaces {
        if interface.endpoints.len() != usize::from(interface.declared_endpoints) {
            return Err(error(
                config.offset,
                "endpoint count disagrees with descriptors",
            ));
        }
        if !config
            .interfaces
            .iter()
            .any(|i| i.number == interface.number && i.alternate == 0)
        {
            return Err(error(config.offset, "interface lacks alternate zero"));
        }
    }
    Ok(())
}

/// Walk using each descriptor's bLength, not untrusted wTotalLength. A mismatch
/// is refused rather than repaired; unknown descriptor bodies are never retained.
pub fn parse(bytes: &[u8]) -> Result<Descriptors, Error> {
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err(error(0, "input exceeds 256 KiB"));
    }
    if bytes.len() < 18 || bytes[..2] != [18, 1] {
        return Err(error(0, "expected complete 18-byte device descriptor"));
    }
    let mut result = Descriptors {
        vendor: le16(&bytes[8..10]),
        product: le16(&bytes[10..12]),
        revision: le16(&bytes[12..14]),
        usb_version: le16(&bytes[2..4]),
        configurations: Vec::new(),
    };
    let declared_configurations = bytes[17];
    let mut offset = 18;
    let mut count = 1;
    while offset < bytes.len() {
        count += 1;
        if count > MAX_DESCRIPTORS {
            return Err(error(offset, "too many descriptors"));
        }
        if bytes.len() - offset < 2 {
            return Err(error(offset, "truncated descriptor header"));
        }
        let length = usize::from(bytes[offset]);
        if length < 2 || length > bytes.len() - offset {
            return Err(error(offset, "invalid or truncated descriptor length"));
        }
        let descriptor = &bytes[offset..offset + length];
        match descriptor[1] {
            2 => {
                if length != 9 {
                    return Err(error(offset, "configuration header must be nine bytes"));
                }
                if let Some(previous) = result.configurations.last() {
                    finish_configuration(previous, offset)?;
                }
                let value = descriptor[5];
                if value == 0 || result.configurations.iter().any(|c| c.value == value) {
                    return Err(error(offset, "zero or duplicate configuration value"));
                }
                result.configurations.push(Configuration {
                    value,
                    interfaces: Vec::new(),
                    offset,
                    declared_length: le16(&descriptor[2..4]),
                    declared_interfaces: descriptor[4],
                });
            }
            4 => {
                if length != 9 {
                    return Err(error(offset, "interface header must be nine bytes"));
                }
                let config = result
                    .configurations
                    .last_mut()
                    .ok_or(error(offset, "interface outside configuration"))?;
                let (number, alternate) = (descriptor[2], descriptor[3]);
                if config
                    .interfaces
                    .iter()
                    .any(|i| (i.number, i.alternate) == (number, alternate))
                {
                    return Err(error(offset, "duplicate interface/alternate"));
                }
                config.interfaces.push(Interface {
                    number,
                    alternate,
                    class: descriptor[5],
                    subclass: descriptor[6],
                    protocol: descriptor[7],
                    declared_endpoints: descriptor[4],
                    endpoints: Vec::new(),
                });
            }
            5 => {
                if ![7, 9].contains(&length) {
                    return Err(error(offset, "endpoint must be seven or nine bytes"));
                }
                let interface = result
                    .configurations
                    .last_mut()
                    .and_then(|c| c.interfaces.last_mut())
                    .ok_or(error(offset, "endpoint outside interface"))?;
                let address = descriptor[2];
                if address & 0x70 != 0
                    || address & 0x0F == 0
                    || interface.endpoints.iter().any(|e| e.address == address)
                {
                    return Err(error(offset, "invalid or duplicate endpoint address"));
                }
                interface.endpoints.push(Endpoint {
                    address,
                    attributes: descriptor[3],
                    max_packet_size: le16(&descriptor[4..6]),
                    interval: descriptor[6],
                });
            }
            1 | 3 | 6 | 7 | 15 => {
                return Err(error(
                    offset,
                    "unexpected descriptor in cached configuration stream",
                ))
            }
            _ => {
                if result.configurations.is_empty() {
                    return Err(error(offset, "descriptor outside configuration"));
                }
                // Class-specific/IAD/companion descriptors: skip only after bounds
                // checks. This is not a full USB class/companion validator.
            }
        }
        offset += length;
    }
    if let Some(last) = result.configurations.last() {
        finish_configuration(last, offset)?;
    }
    if result.configurations.is_empty()
        || result.configurations.len() != usize::from(declared_configurations)
    {
        return Err(error(17, "configuration count disagrees with descriptors"));
    }
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub configuration: u8,
    pub interface: u8,
    pub alternate: u8,
    pub out: Endpoint,
    pub input: Endpoint,
}
impl Descriptors {
    /// `active_configuration` is explicit observed metadata, not a config index.
    /// Vendor class and exactly two ordinary bulk endpoints are policy gates,
    /// not yet observed BEACN descriptors. No fallback to audio/other settings.
    pub fn candidate(&self, active_configuration: u8) -> Result<Candidate, Error> {
        if (self.vendor, self.product) != (VENDOR, PRODUCT) {
            return Err(error(8, "VID/PID is not the static target"));
        }
        let config = self
            .configurations
            .iter()
            .find(|c| c.value == active_configuration)
            .ok_or(error(0, "active configuration absent or unconfigured"))?;
        let interface = config
            .interfaces
            .iter()
            .find(|i| (i.number, i.alternate) == (INTERFACE, ALTERNATE))
            .ok_or(error(
                config.offset,
                "candidate interface 3 alternate 1 absent",
            ))?;
        if interface.class != 0xFF || interface.endpoints.len() != 2 {
            return Err(error(
                config.offset,
                "candidate must be vendor class with exactly two endpoints",
            ));
        }
        let endpoint = |address| -> Result<Endpoint, Error> {
            let ep = interface
                .endpoints
                .iter()
                .find(|e| e.address == address)
                .ok_or(error(config.offset, "candidate pipe address absent"))?;
            if ep.attributes != 2 || ep.max_packet_size == 0 || ep.max_packet_size > 1024 {
                return Err(error(
                    config.offset,
                    "candidate endpoint is not ordinary bounded bulk",
                ));
            }
            if config
                .interfaces
                .iter()
                .filter(|i| i.number != INTERFACE)
                .any(|i| i.endpoints.iter().any(|e| e.address == address))
            {
                return Err(error(
                    config.offset,
                    "candidate address also occurs on another interface",
                ));
            }
            Ok(ep.clone())
        };
        Ok(Candidate {
            configuration: config.value,
            interface: INTERFACE,
            alternate: ALTERNATE,
            out: endpoint(OUT_PIPE)?,
            input: endpoint(IN_PIPE)?,
        })
    }
}
