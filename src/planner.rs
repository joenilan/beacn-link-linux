//! Pure proposals over a caller-supplied snapshot, never applied routes.
use crate::{encode_records, Assignment, Error, Record};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_APPLICATIONS: usize = 256;
pub const MAX_STREAMS: usize = 4096;
pub const MAX_RECORDS: usize = crate::MAX_PAYLOAD / 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApplicationId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Live,
    Stale,
    Disappeared,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub id: StreamId,
    pub state: StreamState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    pub id: ApplicationId,
    /// Caller-assigned stable advertised identity; exact printable ASCII bytes.
    pub advertised_name: Vec<u8>,
    pub streams: Vec<Stream>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    /// Caller-supplied version, not an OS sequence number or freshness guarantee.
    pub snapshot_id: u64,
    pub applications: Vec<Application>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    One,
    Two,
    Three,
    Four,
}
impl Link {
    pub const fn assignment(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
        }
    }
    fn from_byte(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            4 => Some(Self::Four),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedRoute {
    pub application_id: ApplicationId,
    pub link: Link,
    pub stream_ids: Vec<StreamId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Proposed(ProposedRoute),
    UnknownName,
    AmbiguousName {
        application_ids: Vec<ApplicationId>,
    },
    ConflictingAssignments {
        values: Vec<u8>,
    },
    OpaqueAssignment {
        value: u8,
    },
    UnsupportedName {
        error: Error,
    },
    NoStreams {
        application_id: ApplicationId,
    },
    UnavailableStreams {
        application_id: ApplicationId,
        streams: Vec<Stream>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub name: Vec<u8>,
    /// Identical duplicates are counted and coalesced, never multiplied routes.
    pub occurrences: usize,
    pub outcome: Outcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub snapshot_id: u64,
    pub resolutions: Vec<Resolution>,
}
impl Plan {
    pub fn proposed_routes(&self) -> impl Iterator<Item = &ProposedRoute> {
        self.resolutions.iter().filter_map(|r| match &r.outcome {
            Outcome::Proposed(route) => Some(route),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    TooManyApplications {
        count: usize,
    },
    TooManyStreams {
        count: usize,
    },
    TooManyRecords {
        count: usize,
    },
    EmptyApplicationId,
    DuplicateApplicationId {
        id: ApplicationId,
    },
    EmptyStreamId {
        application_id: ApplicationId,
    },
    DuplicateStreamId {
        id: StreamId,
    },
    UnsupportedAdvertisedName {
        application_id: ApplicationId,
        error: Error,
    },
}
impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid planner input: {self:?}")
    }
}
impl std::error::Error for InventoryError {}

fn validate_name(name: &[u8]) -> Result<(), Error> {
    // Reuse the codec's ASCII/length policy. One name always fits the payload.
    encode_records(&[Record {
        assignment: Assignment::from_byte(0),
        name: name.to_vec(),
    }])
    .map(|_| ())
}

/// Invalid inventory aborts planning. Individual assignment failures produce
/// explicit outcomes. Outputs are sorted independently of input order.
/// No record means no proposal for that app, never an implied unroute.
pub fn plan(inventory: &Inventory, records: &[Record]) -> Result<Plan, InventoryError> {
    if inventory.applications.len() > MAX_APPLICATIONS {
        return Err(InventoryError::TooManyApplications {
            count: inventory.applications.len(),
        });
    }
    if records.len() > MAX_RECORDS {
        return Err(InventoryError::TooManyRecords {
            count: records.len(),
        });
    }
    let mut app_ids = BTreeSet::new();
    let mut stream_ids = BTreeSet::new();
    let mut names: BTreeMap<Vec<u8>, Vec<&Application>> = BTreeMap::new();
    let mut stream_count = 0;
    // Stable traversal; invalid snapshots abort before any proposals.
    let mut applications: Vec<_> = inventory.applications.iter().collect();
    applications.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then(a.advertised_name.cmp(&b.advertised_name))
    });
    for app in applications {
        if app.id.0.is_empty() {
            return Err(InventoryError::EmptyApplicationId);
        }
        if !app_ids.insert(app.id.clone()) {
            return Err(InventoryError::DuplicateApplicationId { id: app.id.clone() });
        }
        validate_name(&app.advertised_name).map_err(|error| {
            InventoryError::UnsupportedAdvertisedName {
                application_id: app.id.clone(),
                error,
            }
        })?;
        stream_count += app.streams.len();
        if stream_count > MAX_STREAMS {
            return Err(InventoryError::TooManyStreams {
                count: stream_count,
            });
        }
        let mut streams: Vec<_> = app.streams.iter().collect();
        streams.sort_by(|a, b| a.id.cmp(&b.id));
        for stream in streams {
            if stream.id.0.is_empty() {
                return Err(InventoryError::EmptyStreamId {
                    application_id: app.id.clone(),
                });
            }
            if !stream_ids.insert(stream.id.clone()) {
                return Err(InventoryError::DuplicateStreamId {
                    id: stream.id.clone(),
                });
            }
        }
        names
            .entry(app.advertised_name.clone())
            .or_default()
            .push(app);
    }
    let mut grouped: BTreeMap<Vec<u8>, (usize, BTreeSet<u8>)> = BTreeMap::new();
    for record in records {
        let (count, values) = grouped.entry(record.name.clone()).or_default();
        *count += 1;
        values.insert(record.assignment.byte());
    }
    let mut resolutions = Vec::new();
    for (name, (occurrences, values)) in grouped {
        let outcome = match validate_name(&name) {
            Err(error) => Outcome::UnsupportedName { error },
            Ok(()) if values.len() > 1 => Outcome::ConflictingAssignments {
                values: values.into_iter().collect(),
            },
            Ok(()) => {
                let value = *values.iter().next().expect("group contains a record");
                match Link::from_byte(value) {
                    None => Outcome::OpaqueAssignment { value },
                    Some(link) => match names.get(&name) {
                        None => Outcome::UnknownName,
                        Some(apps) if apps.len() > 1 => Outcome::AmbiguousName {
                            application_ids: apps.iter().map(|app| app.id.clone()).collect(),
                        },
                        Some(apps) => {
                            let app = apps[0];
                            let mut unavailable: Vec<_> = app
                                .streams
                                .iter()
                                .filter(|stream| stream.state != StreamState::Live)
                                .cloned()
                                .collect();
                            unavailable.sort_by(|a, b| a.id.cmp(&b.id));
                            if app.streams.is_empty() {
                                Outcome::NoStreams {
                                    application_id: app.id.clone(),
                                }
                            } else if !unavailable.is_empty() {
                                // Deliberately block all streams, even if some are live.
                                Outcome::UnavailableStreams {
                                    application_id: app.id.clone(),
                                    streams: unavailable,
                                }
                            } else {
                                let mut ids: Vec<_> =
                                    app.streams.iter().map(|stream| stream.id.clone()).collect();
                                ids.sort();
                                Outcome::Proposed(ProposedRoute {
                                    application_id: app.id.clone(),
                                    link,
                                    stream_ids: ids,
                                })
                            }
                        }
                    },
                }
            }
        };
        resolutions.push(Resolution {
            name,
            occurrences,
            outcome,
        });
    }
    Ok(Plan {
        snapshot_id: inventory.snapshot_id,
        resolutions,
    })
}
