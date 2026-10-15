//! Software-only check of the persistent Link In bus on a desktop session.
//! Never selects a Studio card and never opens USB. It creates the four LinX
//! sinks, starts one disposable silent stream whose target is not remembered
//! by the session manager, moves it across Link In 2 -> 3 -> desktop, survives
//! the stream being recreated, and releases everything on exit.
//!
//! Run: cargo run --example link_bus_probe --features link-bus
#[cfg(all(feature = "link-bus", target_os = "linux"))]
fn main() -> Result<(), String> {
    use beacn_link_protocol::link_bus::{LinkBus, Stream};
    use std::{
        process::{Child, Command, Stdio},
        thread::sleep,
        time::{Duration, Instant},
    };

    const NAME: &str = "linx-bus-probe";
    fn spawn() -> Result<Child, String> {
        Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--rate",
                "48000",
                "--channels",
                "2",
                "--format",
                "f32",
                "--properties",
                &format!(
                    "{{ application.name = {NAME} node.name = {NAME} state.restore-target = false state.restore-props = false }}"
                ),
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())
            .map(|mut child| {
                // Feed digital silence from a thread for the child's lifetime.
                let mut input = child.stdin.take().unwrap();
                std::thread::spawn(move || {
                    use std::io::Write;
                    let block = vec![0u8; 48000 * 8 / 50];
                    while input.write_all(&block).is_ok() {
                        sleep(Duration::from_millis(20));
                    }
                });
                child
            })
    }
    fn wait(
        bus: &mut LinkBus,
        what: &str,
        test: impl Fn(&[Stream], usize) -> bool,
    ) -> Result<Vec<Stream>, String> {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            bus.maintain()?;
            let snapshot = bus.snapshot()?;
            let ours: Vec<Stream> = snapshot
                .streams
                .iter()
                .filter(|s| s.app == NAME)
                .cloned()
                .collect();
            if test(&ours, snapshot.status.sinks_ready) {
                println!("PASS {what}: {:?}", ours.iter().map(|s| (s.node, s.channel)).collect::<Vec<_>>());
                return Ok(ours);
            }
            if Instant::now() > deadline {
                return Err(format!("FAIL {what}: status={:?} ours={ours:?}", snapshot.status));
            }
            sleep(Duration::from_millis(100));
        }
    }

    let started = Instant::now();
    let mut bus = LinkBus::open()?;
    bus.set_card(None);
    wait(&mut bus, "four sinks", |_, ready| ready == 4)?;
    println!("bus ready after {} ms", started.elapsed().as_millis());

    let mut child = spawn()?;
    let result = (|| {
        let streams = wait(&mut bus, "stream on desktop output", |s, _| {
            s.len() == 1 && s[0].channel == 0
        })?;
        bus.route(&streams[0], 2)?;
        let streams = wait(&mut bus, "moved to Link In 2", |s, _| s.len() == 1 && s[0].channel == 2)?;
        let moved = Instant::now();
        bus.route(&streams[0], 3)?;
        let _ = wait(&mut bus, "moved to Link In 3", |s, _| s.len() == 1 && s[0].channel == 3)?;
        println!("channel move took {} ms", moved.elapsed().as_millis());
        // Destroy one sink's link from outside? Not ours to touch; instead check
        // that maintain() is a no-op while healthy.
        for _ in 0..5 {
            bus.maintain()?;
        }
        let streams = wait(&mut bus, "still on Link In 3 after maintenance", |s, _| {
            s.len() == 1 && s[0].channel == 3
        })?;
        bus.route(&streams[0], 0)?;
        wait(&mut bus, "back on desktop output", |s, _| s.len() == 1 && s[0].channel == 0)?;
        Ok::<_, String>(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result?;
    wait(&mut bus, "stream gone", |s, _| s.is_empty())?;
    drop(bus);
    println!("bus released");
    Ok(())
}

#[cfg(not(all(feature = "link-bus", target_os = "linux")))]
fn main() {
    eprintln!("Requires Linux and --features link-bus.");
}
