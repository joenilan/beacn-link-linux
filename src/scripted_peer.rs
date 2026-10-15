//! Explicit test-only USB backend: protocol bytes/guards, no native/device I/O.
use crate::{
    encode_frame, encode_records,
    usb_handshake::{Backend, Observation},
    Frame, Record,
};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
#[derive(Clone)]
pub enum Reply {
    Records(Vec<Record>),
    NoPayload,
    Rejected,
    Malformed,
}
#[derive(Default)]
pub struct Journal {
    pub writes: Vec<Vec<u8>>,
    pub reads: usize,
    pub releases: usize,
    pub closes: usize,
    pub fail_write: Option<usize>,
    pub short_write: Option<usize>,
    pub bad_initialization: bool,
    pub release_fails: bool,
    pub changed: bool,
}
pub struct ScriptedPeer {
    journal: Rc<RefCell<Journal>>,
    alternate: u8,
    script: VecDeque<Reply>,
    last: Reply,
}
impl ScriptedPeer {
    pub fn new(script: Vec<Reply>) -> Result<(Self, Rc<RefCell<Journal>>), String> {
        if script.is_empty() || script.len() > 40 {
            return Err("peer script requires 1..40 replies".into());
        }
        for reply in &script {
            if let Reply::Records(records) = reply {
                encode_records(records).map_err(|e| e.to_string())?;
            }
        }
        let journal = Rc::new(RefCell::new(Journal::default()));
        Ok((
            Self {
                journal: journal.clone(),
                alternate: 0,
                script: script.into(),
                last: Reply::NoPayload,
            },
            journal,
        ))
    }
}
impl Backend for ScriptedPeer {
    fn observe(&mut self) -> Result<Observation, i32> {
        Ok(Observation {
            instance: if self.journal.borrow().changed {
                45
            } else {
                44
            },
            descriptors: include_str!("../fixtures/usb-handshake.synthetic.hex")
                .split_whitespace()
                .map(|s| u8::from_str_radix(s, 16).unwrap())
                .collect(),
            configuration: 1,
            alternate: self.alternate,
            kernel_bound: false,
            audio_interfaces: (0..3).map(|n| (n, 0, "snd-usb-audio".into())).collect(),
        })
    }
    fn claim_three(&mut self) -> Result<(), i32> {
        Ok(())
    }
    fn select_one(&mut self) -> Result<(), i32> {
        self.alternate = 1;
        Ok(())
    }
    fn write(&mut self, bytes: &[u8], _: u32) -> Result<usize, i32> {
        let mut journal = self.journal.borrow_mut();
        journal.writes.push(bytes.to_vec());
        if journal.fail_write == Some(journal.writes.len()) {
            Err(-7)
        } else if journal.short_write == Some(journal.writes.len()) {
            Ok(bytes.len() - 1)
        } else {
            Ok(bytes.len())
        }
    }
    fn read(&mut self, bytes: &mut [u8], _: u32) -> Result<usize, i32> {
        let mut journal = self.journal.borrow_mut();
        journal.reads += 1;
        if journal.reads == 1 {
            bytes.fill(0);
            bytes[..4].copy_from_slice(
                &(if journal.bad_initialization {
                    0xA1000000u32
                } else {
                    0xA2000000u32
                })
                .to_le_bytes(),
            );
            return Ok(512);
        }
        if let Some(reply) = self.script.pop_front() {
            self.last = reply;
        }
        let frame = match &self.last {
            Reply::Records(records) => {
                encode_frame(&Frame::Data(encode_records(records).map_err(|_| -1)?))
                    .map_err(|_| -1)?
            }
            Reply::NoPayload => 0xAC000000u32.to_le_bytes().to_vec(),
            Reply::Rejected => 0xACFFFFFFu32.to_le_bytes().to_vec(),
            Reply::Malformed => vec![1, 2, 3],
        };
        bytes[..frame.len()].copy_from_slice(&frame);
        Ok(frame.len())
    }
    fn release_three(&mut self) -> Result<(), i32> {
        let mut journal = self.journal.borrow_mut();
        journal.releases += 1;
        if journal.release_fails {
            Err(-9)
        } else {
            self.alternate = 0;
            Ok(())
        }
    }
    fn close(&mut self) {
        self.journal.borrow_mut().closes += 1;
    }
}
