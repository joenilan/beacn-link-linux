use std::{env, path::PathBuf, process::Command};

fn main() {
    if env::var_os("CARGO_FEATURE_NATIVE_USB").is_some() {
        assert_eq!(
            env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap(),
            "64",
            "native-usb currently requires a 64-bit target"
        );
        assert_eq!(
            env::var("CARGO_CFG_TARGET_OS").unwrap(),
            "linux",
            "native-usb requires Linux"
        );
        assert_eq!(
            env::var("HOST").unwrap(),
            env::var("TARGET").unwrap(),
            "native USB cross compilation is unsupported"
        );
    }
    println!("cargo:rerun-if-changed=native/pipewire.c");
    println!("cargo:rerun-if-changed=native/playback.inc");
    println!("cargo:rerun-if-changed=native/desktop_target.inc");
    println!("cargo:rerun-if-changed=native/meter.inc");
    println!("cargo:rerun-if-changed=native/link_bus.c");
    if env::var_os("CARGO_FEATURE_NATIVE_PIPEWIRE").is_none() {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_OS").unwrap(),
        "linux",
        "native-pipewire requires Linux"
    );
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "native cross compilation is unsupported"
    );
    println!("cargo:rerun-if-env-changed=BEACN_PIPEWIRE_SYSROOT");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let object = out.join("pipewire.o");
    let mut cc = Command::new("cc");
    cc.args([
        "-std=gnu11",
        "-D_GNU_SOURCE",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-fPIC",
        "-c",
        "native/pipewire.c",
        "-o",
    ])
    .arg(&object);
    if env::var_os("CARGO_FEATURE_NATIVE_TEST_FAULTS").is_some() {
        cc.arg("-DBCL_TEST_FAULTS");
    }
    if env::var_os("CARGO_FEATURE_NATIVE_PLAYBACK").is_some() {
        cc.arg("-DBCL_NATIVE_PLAYBACK");
    }
    if env::var_os("CARGO_FEATURE_NATIVE_DESKTOP_TARGET").is_some() {
        cc.arg("-DBCL_DESKTOP_TARGET");
    }
    if let Some(root) = env::var_os("BEACN_PIPEWIRE_SYSROOT") {
        let root = PathBuf::from(root);
        assert!(root.is_absolute(), "header sysroot must be absolute");
        cc.arg("-I").arg(root.join("usr/include/pipewire-0.3"));
        cc.arg("-I").arg(root.join("usr/include/spa-0.2"));
    } else {
        let flags = Command::new("pkg-config")
            .args(["--cflags", "libpipewire-0.3 >= 1.0"])
            .output()
            .expect("pkg-config required");
        assert!(
            flags.status.success(),
            "PipeWire development headers >= 1.0 required"
        );
        for flag in std::str::from_utf8(&flags.stdout)
            .unwrap()
            .split_whitespace()
        {
            cc.arg(flag);
        }
    }
    assert!(
        cc.status().expect("C compiler required").success(),
        "native adapter compilation failed"
    );
    let mut objects = vec![object];
    if env::var_os("CARGO_FEATURE_LINK_BUS").is_some() {
        // Persistent Link In channel bus; same headers/flags as the adapter.
        let bus = out.join("link_bus.o");
        let mut cc = Command::new("cc");
        cc.args([
            "-std=gnu11",
            "-D_GNU_SOURCE",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fPIC",
            "-c",
            "native/link_bus.c",
            "-o",
        ])
        .arg(&bus)
        .args(pipewire_flags());
        assert!(
            cc.status().expect("C compiler required").success(),
            "link bus compilation failed"
        );
        objects.push(bus);
    }
    assert!(Command::new("ar")
        .arg("crs")
        .arg(out.join("libbcl_pipewire.a"))
        .args(&objects)
        .status()
        .unwrap()
        .success());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=bcl_pipewire");
    println!("cargo:rustc-link-lib=dylib:+verbatim=libpipewire-0.3.so.0");
}

fn pipewire_flags() -> Vec<String> {
    if let Some(root) = env::var_os("BEACN_PIPEWIRE_SYSROOT") {
        let root = PathBuf::from(root);
        return vec![
            format!("-I{}", root.join("usr/include/pipewire-0.3").display()),
            format!("-I{}", root.join("usr/include/spa-0.2").display()),
        ];
    }
    let flags = Command::new("pkg-config")
        .args(["--cflags", "libpipewire-0.3 >= 1.0"])
        .output()
        .expect("pkg-config required");
    assert!(
        flags.status.success(),
        "PipeWire development headers >= 1.0 required"
    );
    std::str::from_utf8(&flags.stdout)
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}
