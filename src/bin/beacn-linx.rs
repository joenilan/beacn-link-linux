use std::process::ExitCode;
const HELP:&str="Beacn LinX — Rust application service\nRead-only commands: inventory | service SECONDS (1..600)\nOptional test core: --private-runtime PATH --core NAME\nService stdin: select APP-ID=ASCII-NAME | unselect APP-ID | status | quit\nNo USB, audio capture or route application. Selections last only this process.";

#[cfg(all(feature = "native-pipewire", target_os = "linux"))]
fn run(args: &[String]) -> Result<(), String> {
    use beacn_link_protocol::{
        catalog::ApplicationService, native_catalog::NativeCatalog, planner::ApplicationId,
    };
    use std::{
        io::{self, BufRead, Read, Write},
        path::Path,
        sync::mpsc,
        time::{Duration, Instant},
    };
    let (command, tail) = if args.first().is_some_and(|s| s == "inventory") {
        ("inventory", &args[1..])
    } else if args.first().is_some_and(|s| s == "service") {
        ("service", &args[1..])
    } else {
        return Err(HELP.into());
    };
    let (seconds, tail) = if command == "service" {
        let seconds = tail
            .first()
            .ok_or("service requires finite seconds")?
            .parse::<u64>()
            .map_err(|_| "invalid service duration")?;
        if !(1..=600).contains(&seconds) {
            return Err("service duration must be 1..600 seconds".into());
        }
        (seconds, &tail[1..])
    } else {
        (0, tail)
    };
    let mut catalog = match tail {
        [] => NativeCatalog::desktop()?,
        [flag, runtime, core, name] if flag == "--private-runtime" && core == "--core" => {
            NativeCatalog::connect(Path::new(runtime), name)?
        }
        _ => return Err("invalid read-only connection arguments".into()),
    };
    #[cfg(feature = "native-test-faults")]
    if !catalog.verify_mutation_refusal() {
        return Err("catalog mutation gate failed".into());
    }
    fn status(service: &ApplicationService) {
        let c = service.current().expect("observed before status");
        println!(
            "STATUS generation={} revision={} applications={} selected={} apply=disabled core={:?}",
            c.generation,
            service.revision(),
            c.applications.len(),
            service.selected().len(),
            c.core
        );
        for app in c.applications.values() {
            println!(
                "APP {} pid={} names={:?} binaries={:?} streams={} selected={:?}",
                app.id.0,
                app.pid,
                app.names,
                app.binaries,
                app.streams.len(),
                service.selected().get(&app.id)
            );
        }
        let _ = io::stdout().flush();
    }
    let mut service = ApplicationService::default();
    service.update(catalog.observe(1)?)?;
    status(&service);
    if command == "inventory" {
        return Ok(());
    }
    let (tx, rx) = mpsc::sync_channel::<Result<String, String>>(8);
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        loop {
            let mut line = String::new();
            let event = match (&mut input).take(1026).read_line(&mut line) {
                Ok(0) => Ok("quit".into()),
                Ok(_) if line.len() <= 1024 && line.ends_with('\n') => {
                    Ok(line.trim_end_matches(['\n', '\r']).into())
                }
                _ => Err("stdin command exceeds bound or is unterminated".into()),
            };
            let end = event.as_ref().is_err() || event.as_ref().is_ok_and(|s| s == "quit");
            if tx.send(event).is_err() || end {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut poll = Instant::now() + Duration::from_millis(500);
    let mut generation = 1u64;
    while Instant::now() < deadline {
        let wait = poll.min(deadline).saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(event) => {
                let line = event?;
                if line == "quit" {
                    println!("STOP requested; read-only resources released");
                    return Ok(());
                }
                // Every selection command is anchored to a new native catalog.
                generation = generation.checked_add(1).ok_or("generation exhausted")?;
                let update = service.update(catalog.observe(generation)?)?;
                for id in update.revoked {
                    println!("REVOKED {} departed/stale/core-changed", id.0);
                }
                let result = if line == "status" {
                    Ok(())
                } else if let Some(value) = line.strip_prefix("select ") {
                    match value.split_once('=') {
                        Some((id, name)) => service.select(&ApplicationId(id.into()), name),
                        None => Err("select requires APP-ID=ASCII-NAME".into()),
                    }
                } else if let Some(id) = line.strip_prefix("unselect ") {
                    service.unselect(&ApplicationId(id.into()))
                } else {
                    Err("unknown command; use select, unselect, status or quit".into())
                };
                match result {
                    Ok(()) => println!("OK read-only selection; apply=disabled"),
                    Err(error) => println!("REFUSED {error}"),
                };
                status(&service);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
        if Instant::now() >= poll {
            generation = generation.checked_add(1).ok_or("generation exhausted")?;
            let update = service.update(catalog.observe(generation)?)?;
            if update.catalog_changed || !update.revoked.is_empty() {
                println!(
                    "UPDATE revoked={:?} core_changed={}",
                    update.revoked, update.core_changed
                );
                status(&service);
            }
            poll = Instant::now() + Duration::from_millis(500);
        }
    }
    println!("STOP deadline; read-only resources released");
    Ok(())
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    #[cfg(all(feature = "native-pipewire", target_os = "linux"))]
    let result = run(&args);
    #[cfg(not(all(feature = "native-pipewire", target_os = "linux")))]
    let result: Result<(), String> = Err(
        "native application inventory requires Linux and native-pipewire; apply disabled".into(),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Beacn LinX stopped: {error}");
            ExitCode::from(2)
        }
    }
}
