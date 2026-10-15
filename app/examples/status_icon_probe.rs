//! Private-bus qualification only; no desktop, routing or device access.
#[cfg(all(feature = "status-tray", target_os = "linux"))]
fn main() {
    use beacn_linx_app::{
        icon::{self, Status},
        tray::Tray,
    };
    use std::{
        io::{self, BufRead, Write},
        time::{Duration, Instant},
    };
    let tray = Tray::spawn();
    let end = Instant::now() + Duration::from_secs(5);
    while tray.available().is_none() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    println!("READY {:?}", tray.available());
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines().take(20) {
        let line = line.unwrap();
        let status = match line.as_str() {
            "gray" => Some(Status::Disconnected),
            "amber" => Some(Status::Connecting),
            "teal" => Some(Status::Connected),
            "red" => Some(Status::Error),
            "blue" => Some(Status::PrivateTest),
            _ => None,
        };
        if let Some(status) = status {
            tray.update(
                status,
                matches!(status, Status::Connected | Status::PrivateTest),
            );
        } else if line == "actions" {
            println!("ACTIONS {}", tray.actions());
        } else if line == "quit" {
            break;
        } else if let Some(path) = line.strip_prefix("image ") {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .unwrap();
            write!(file, "P6\n640 128\n255\n").unwrap();
            let images = [
                Status::Disconnected,
                Status::Connecting,
                Status::Connected,
                Status::Error,
                Status::PrivateTest,
            ]
            .map(|s| icon::raster(s, 128));
            for y in 0..128 {
                for image in &images {
                    for x in 0..128 {
                        let p = &image.rgba[(y * 128 + x) * 4..][..4];
                        let color = p[..3]
                            .iter()
                            .map(|v| {
                                ((*v as u32 * p[3] as u32 + 19 * (255 - p[3] as u32)) / 255) as u8
                            })
                            .collect::<Vec<_>>();
                        file.write_all(&color).unwrap();
                    }
                }
            }
        } else {
            panic!("unknown qualification command");
        }
        println!("OK");
        io::stdout().flush().unwrap();
    }
    drop(tray);
    println!("STOPPED");
}
#[cfg(not(all(feature = "status-tray", target_os = "linux")))]
fn main() {
    eprintln!("requires Linux status-tray");
    std::process::exit(2);
}
