//! Explicit qualification only: real private routing, scripted peer, zero USB.
use beacn_link_protocol::{
    catalog::ApplicationService,
    executor::Destination,
    link_session::{LinkSession, ManagedOptions, OpenFailure, Options},
    native_routing_lease::NativeRoutingLease,
    scripted_peer::{Reply, ScriptedPeer},
    session::{PollPolicy, ReceivedNamePolicy},
    usb_handshake::{Handshake, Publication},
    Assignment, Record,
};
use std::{path::PathBuf, time::Duration};
#[derive(Clone)]
pub struct Configuration {
    pub runtime: PathBuf,
    pub core: String,
    pub cookie: u32,
    pub server: u32,
    pub owner: String,
    pub destinations: Vec<Destination>,
    pub cycles: usize,
}
pub type Session = LinkSession<NativeRoutingLease, Handshake<ScriptedPeer>>;
pub fn start(
    session: &mut Session,
    service: &ApplicationService,
    config: &Configuration,
) -> Result<(), String> {
    start_with(session, service, config, false)
}
/// No USB or physical playback: exercise the production session lifetime with
/// real retained private-core routing and an explicitly scripted peer.
pub fn start_managed(
    session: &mut Session,
    service: &ApplicationService,
    config: &Configuration,
) -> Result<(), String> {
    start_with(session, service, config, true)
}
fn start_with(
    session: &mut Session,
    service: &ApplicationService,
    config: &Configuration,
    managed: bool,
) -> Result<(), String> {
    if service.current().is_none_or(|c| {
        c.core
            != format!(
                "pw-core:{}:pid:{}:{}",
                config.cookie, config.server, config.core
            )
    }) {
        return Err("Private session core no longer matches discovery.".into());
    }
    let bindings = service
        .selected()
        .iter()
        .map(|(id, name)| (id.0.clone(), name.clone()))
        .collect();
    let records: Vec<_> = service
        .selected()
        .values()
        .enumerate()
        .map(|(n, name)| Record {
            name: name.as_bytes().to_vec(),
            assignment: Assignment::from_byte(if n % 2 == 0 { 1 } else { 4 }),
        })
        .collect();
    let routes = || {
        NativeRoutingLease::connect(
            &config.runtime,
            &config.core,
            config.cookie,
            config.server,
            bindings,
            config.owner.clone(),
        )
    };
    let transport = |inventory: &beacn_link_protocol::planner::Inventory, policy| {
        let (peer, _) =
            ScriptedPeer::new(vec![Reply::Records(records)]).map_err(|message| OpenFailure {
                message,
                cleanup: Ok(()),
            })?;
        let publication = if managed {
            Publication::managed_selected(inventory, policy)
        } else {
            Publication::bounded_selected(inventory, config.cycles, policy)
        }
        .map_err(|e| OpenFailure {
            message: e.to_string(),
            cleanup: Ok(()),
        })?;
        Handshake::prepare_publication(peer, publication).map_err(|e| OpenFailure {
            message: e.error.to_string(),
            cleanup: if e.cleanup.baseline_restored {
                Ok(())
            } else {
                Err("scripted claim cleanup not verified".into())
            },
        })
    };
    let policies = ManagedOptions {
        poll_policy: PollPolicy::ReportRejectedMarker,
        names: ReceivedNamePolicy::WindowsCString,
        initialization_timeout: Duration::from_millis(500),
    };
    if managed {
        session.start_managed(
            service,
            config.destinations.clone(),
            policies,
            routes,
            |inventory, options| transport(inventory, options.poll_policy),
        )
    } else {
        session.start(
            service,
            config.destinations.clone(),
            Options {
                cycles: config.cycles,
                poll_policy: policies.poll_policy,
                names: policies.names,
                initialization_timeout: policies.initialization_timeout,
            },
            routes,
            |inventory, options| transport(inventory, options.poll_policy),
        )
    }
}
