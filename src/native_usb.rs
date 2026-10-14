//! Opt-in Linux libusb adapter. Only a selected fd is wrapped; no discovery API.
use crate::usb_handshake::{Backend, Handshake, Observation, SetupFailure, Startup};
use std::{
    ffi::{c_int, c_uint, c_void},
    fs::{File, OpenOptions},
    io::Read,
    marker::PhantomData,
    os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
};

#[repr(C)]
union OptionValue {
    integer: c_int,
    callback: *const c_void,
}
#[repr(C)]
struct InitOption {
    option: c_int,
    value: OptionValue,
}
#[repr(C)]
struct Version {
    major: u16,
    minor: u16,
    micro: u16,
    nano: u16,
    rc: *const u8,
    description: *const u8,
}
#[repr(C)]
#[derive(Default)]
struct DeviceDescriptor {
    length: u8,
    kind: u8,
    usb: u16,
    class: u8,
    subclass: u8,
    protocol: u8,
    packet: u8,
    vendor: u16,
    product: u16,
    revision: u16,
    manufacturer: u8,
    name: u8,
    serial: u8,
    configurations: u8,
}
impl DeviceDescriptor {
    fn bytes(&self) -> Vec<u8> {
        let mut out = vec![self.length, self.kind];
        out.extend(self.usb.to_le_bytes());
        out.extend([self.class, self.subclass, self.protocol, self.packet]);
        for word in [self.vendor, self.product, self.revision] {
            out.extend(word.to_le_bytes());
        }
        out.extend([
            self.manufacturer,
            self.name,
            self.serial,
            self.configurations,
        ]);
        out
    }
}
#[link(name = "libusb-1.0.so.0", kind = "dylib", modifiers = "+verbatim")]
unsafe extern "C" {
    fn libusb_get_version() -> *const Version;
    fn libusb_init_context(
        ctx: *mut *mut c_void,
        options: *const InitOption,
        count: c_int,
    ) -> c_int;
    fn libusb_exit(ctx: *mut c_void);
    fn libusb_wrap_sys_device(ctx: *mut c_void, fd: isize, handle: *mut *mut c_void) -> c_int;
    fn libusb_close(handle: *mut c_void);
    fn libusb_get_device(handle: *mut c_void) -> *mut c_void;
    fn libusb_get_device_descriptor(device: *mut c_void, desc: *mut DeviceDescriptor) -> c_int;
    fn libusb_get_bus_number(device: *mut c_void) -> u8;
    fn libusb_get_device_address(device: *mut c_void) -> u8;
    fn libusb_get_device_speed(device: *mut c_void) -> c_int;
    fn libusb_get_configuration(handle: *mut c_void, config: *mut c_int) -> c_int;
    fn libusb_kernel_driver_active(handle: *mut c_void, interface: c_int) -> c_int;
    fn libusb_claim_interface(handle: *mut c_void, interface: c_int) -> c_int;
    fn libusb_set_interface_alt_setting(
        handle: *mut c_void,
        interface: c_int,
        alternate: c_int,
    ) -> c_int;
    fn libusb_release_interface(handle: *mut c_void, interface: c_int) -> c_int;
    fn libusb_clear_halt(handle: *mut c_void, endpoint: u8) -> c_int;
    fn libusb_bulk_transfer(
        handle: *mut c_void,
        endpoint: u8,
        bytes: *mut u8,
        length: c_int,
        transferred: *mut c_int,
        timeout: c_uint,
    ) -> c_int;
}
const FOREIGN: i32 = -9001;
const O_NOFOLLOW: i32 = 0x20000;
fn result(code: i32) -> Result<(), i32> {
    if code < 0 {
        Err(code)
    } else {
        Ok(())
    }
}
fn kernel_ownership(kernel: i32, driver: &str, claimed: bool) -> Result<bool, i32> {
    if kernel < 0 {
        return Err(kernel);
    }
    // Linux exposes our usbfs userspace claim as a sysfs driver, while
    // libusb_kernel_driver_active deliberately reports usbfs as inactive.
    // Accept it only after this retained handle acknowledged its own claim.
    match (kernel != 0, driver, claimed) {
        (false, "", false) | (false, "usbfs", true) => Ok(false),
        (true, name, _) if !name.is_empty() && name != "usbfs" => Ok(true),
        _ => Err(FOREIGN),
    }
}
fn cached(path: &Path, limit: usize) -> Result<Vec<u8>, i32> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| FOREIGN)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| FOREIGN)?;
    if bytes.len() > limit {
        return Err(FOREIGN);
    }
    Ok(bytes)
}
fn number(path: &Path, base: u32) -> Result<u16, i32> {
    let raw = cached(path, 64)?;
    let text = std::str::from_utf8(&raw).map_err(|_| FOREIGN)?.trim();
    if text.is_empty()
        || !text.bytes().all(|b| {
            if base == 16 {
                b.is_ascii_hexdigit()
            } else {
                b.is_ascii_digit()
            }
        })
    {
        return Err(FOREIGN);
    }
    u16::from_str_radix(text, base).map_err(|_| FOREIGN)
}
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    sys_dev: u64,
    sys_ino: u64,
    node_dev: u64,
    node_ino: u64,
    node_rdev: u64,
}
fn stamp(sysfs: &Path, node: &Path) -> Result<Stamp, i32> {
    let s = std::fs::metadata(sysfs).map_err(|_| FOREIGN)?;
    let n = std::fs::symlink_metadata(node).map_err(|_| FOREIGN)?;
    if !s.is_dir() || !n.file_type().is_char_device() {
        return Err(FOREIGN);
    }
    Ok(Stamp {
        sys_dev: s.dev(),
        sys_ino: s.ino(),
        node_dev: n.dev(),
        node_ino: n.ino(),
        node_rdev: n.rdev(),
    })
}
#[derive(Debug)]
pub enum OpenError {
    Refused,
    Local(i32),
    Setup(SetupFailure),
}
impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native USB: {self:?}")
    }
}
impl std::error::Error for OpenError {}
/// Library metadata only: does not initialize libusb or touch any device.
pub fn library_version() -> (u16, u16, u16) {
    unsafe {
        let v = &*libusb_get_version();
        (v.major, v.minor, v.micro)
    }
}
/// Opaque single-threaded owner. No public raw-handle or unrestricted transfer API.
pub struct NativeUsb {
    context: *mut c_void,
    handle: *mut c_void,
    file: Option<File>,
    sysfs: PathBuf,
    node: PathBuf,
    stamp: Stamp,
    expected: Vec<u8>,
    bus: u8,
    address: u8,
    claimed: bool,
    initial_read_limit_ms: u32,
    initial_read_used: bool,
    _single_threaded: PhantomData<Rc<()>>,
}
/// Explicit mutation-bearing entry point. `authorized` acknowledges external
/// owner authority; it does not infer it from a descriptor report or handoff.
pub fn open_handshake(
    sysfs: &Path,
    expected: &[u8],
    authorized: bool,
) -> Result<Handshake<NativeUsb>, OpenError> {
    open_handshake_with_startup(sysfs, expected, authorized, Startup::SelectedAlternate)
}
/// Explicit startup experiment, bounded externally like other blocking libusb
/// setup calls. No device reset, audio interface claim or post-error recovery.
pub fn open_handshake_with_startup(
    sysfs: &Path,
    expected: &[u8],
    authorized: bool,
    startup: Startup,
) -> Result<Handshake<NativeUsb>, OpenError> {
    open_selected(sysfs, expected, authorized, startup, 500)
}
/// Explicit handshake-only initial-read diagnostic, retaining all device guards.
/// A valid A2 is mandatory; no retries or publication/poll capability are added.
pub fn open_handshake_initialization_diagnostic(
    sysfs: &Path,
    expected: &[u8],
    authorized: bool,
    startup: Startup,
) -> Result<Handshake<NativeUsb>, OpenError> {
    open_selected(sysfs, expected, authorized, startup, 5000)
}
fn open_selected(
    sysfs: &Path,
    expected: &[u8],
    authorized: bool,
    startup: Startup,
    initial_read_limit_ms: u32,
) -> Result<Handshake<NativeUsb>, OpenError> {
    if !authorized {
        return Err(OpenError::Refused);
    }
    if library_version() < (1, 0, 27) {
        return Err(OpenError::Refused);
    }
    crate::usb_handshake::descriptor_profile(expected, 1).map_err(|_| OpenError::Refused)?;
    let root = sysfs.canonicalize().map_err(|_| OpenError::Refused)?;
    if !root.starts_with("/sys/devices")
        || root.join("subsystem").canonicalize().ok().as_deref() != Some(Path::new("/sys/bus/usb"))
    {
        return Err(OpenError::Refused);
    }
    let bus = u8::try_from(number(&root.join("busnum"), 10).map_err(OpenError::Local)?)
        .map_err(|_| OpenError::Refused)?;
    let address = u8::try_from(number(&root.join("devnum"), 10).map_err(OpenError::Local)?)
        .map_err(|_| OpenError::Refused)?;
    if bus == 0
        || address == 0
        || address > 127
        || cached(
            &root.join("descriptors"),
            crate::usb_descriptors::MAX_DESCRIPTOR_BYTES,
        )
        .map_err(OpenError::Local)?
            != expected
    {
        return Err(OpenError::Refused);
    }
    let node = PathBuf::from(format!("/dev/bus/usb/{bus:03}/{address:03}"));
    let before = stamp(&root, &node).map_err(OpenError::Local)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_NOFOLLOW)
        .open(&node)
        .map_err(|e| OpenError::Local(e.raw_os_error().unwrap_or(FOREIGN)))?;
    let held = file.metadata().map_err(|_| OpenError::Refused)?;
    if (held.dev(), held.ino(), held.rdev()) != (before.node_dev, before.node_ino, before.node_rdev)
        || stamp(&root, &node).map_err(OpenError::Local)? != before
    {
        return Err(OpenError::Refused);
    }
    let mut backend = NativeUsb {
        context: ptr::null_mut(),
        handle: ptr::null_mut(),
        file: Some(file),
        sysfs: root,
        node,
        stamp: before,
        expected: expected.to_vec(),
        bus,
        address,
        claimed: false,
        initial_read_limit_ms,
        initial_read_used: false,
        _single_threaded: PhantomData,
    };
    // ABI from public libusb 1.0.29 header; Linux-only NO_DEVICE_DISCOVERY=2.
    let option = InitOption {
        option: 2,
        value: OptionValue {
            callback: ptr::null(),
        },
    };
    unsafe {
        result(libusb_init_context(&mut backend.context, &option, 1)).map_err(OpenError::Local)?;
        result(libusb_wrap_sys_device(
            backend.context,
            backend.file.as_ref().unwrap().as_raw_fd() as isize,
            &mut backend.handle,
        ))
        .map_err(OpenError::Local)?;
    }
    if initial_read_limit_ms == 5000 {
        Handshake::prepare_initialization_diagnostic(backend, startup)
    } else {
        Handshake::prepare_with_startup(backend, startup)
    }
    .map_err(OpenError::Setup)
}
/// Explicit publication policy: one-shot normally, capped cycles only through
/// Publication::bounded. The default opener remains handshake-only.
/// `publication` must be built from a caller-observed inventory before device I/O.
pub fn open_publication(
    sysfs: &Path,
    expected: &[u8],
    publication: crate::usb_handshake::Publication,
    authorized: bool,
) -> Result<Handshake<NativeUsb>, OpenError> {
    open_publication_with_startup(
        sysfs,
        expected,
        publication,
        authorized,
        Startup::SelectedAlternate,
    )
}
pub fn open_publication_with_startup(
    sysfs: &Path,
    expected: &[u8],
    publication: crate::usb_handshake::Publication,
    authorized: bool,
    startup: Startup,
) -> Result<Handshake<NativeUsb>, OpenError> {
    open_selected(
        sysfs,
        expected,
        authorized,
        startup,
        publication.initialization_timeout().as_millis() as u32,
    )
    .map(|owner| owner.enable_publication(publication))
}
impl NativeUsb {
    fn guard(&self) -> Result<(), i32> {
        if self.handle.is_null() || stamp(&self.sysfs, &self.node)? != self.stamp {
            return Err(FOREIGN);
        }
        let held = self
            .file
            .as_ref()
            .ok_or(FOREIGN)?
            .metadata()
            .map_err(|_| FOREIGN)?;
        if (held.dev(), held.ino(), held.rdev())
            != (
                self.stamp.node_dev,
                self.stamp.node_ino,
                self.stamp.node_rdev,
            )
        {
            return Err(FOREIGN);
        }
        Ok(())
    }
    fn interface(&self, n: u8) -> Result<(u8, String), i32> {
        let name = self
            .sysfs
            .file_name()
            .ok_or(FOREIGN)?
            .to_str()
            .ok_or(FOREIGN)?;
        let folder = self.sysfs.join(format!("{name}:1.{n}"));
        if number(&folder.join("bInterfaceNumber"), 16)? != u16::from(n) {
            return Err(FOREIGN);
        }
        let alternate =
            u8::try_from(number(&folder.join("bAlternateSetting"), 10)?).map_err(|_| FOREIGN)?;
        let driver = match std::fs::read_link(folder.join("driver")) {
            Ok(path) => path
                .file_name()
                .and_then(|p| p.to_str())
                .ok_or(FOREIGN)?
                .to_owned(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(_) => return Err(FOREIGN),
        };
        if driver.len() > 64
            || !driver
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
        {
            return Err(FOREIGN);
        }
        Ok((alternate, driver))
    }
    fn bulk(
        &mut self,
        endpoint: u8,
        bytes: *mut u8,
        length: usize,
        ms: u32,
        maximum_ms: u32,
    ) -> Result<usize, i32> {
        self.guard().map_err(|code| {
            eprintln!("native USB bulk guard refusal: endpoint=0x{endpoint:02x} code={code}; no bulk submission");
            code
        })?;
        if length > crate::MAX_FRAME || ms == 0 || ms > maximum_ms {
            return Err(FOREIGN);
        }
        let mut transferred = 0;
        let started = std::time::Instant::now();
        let code = unsafe {
            libusb_bulk_transfer(
                self.handle,
                endpoint,
                bytes,
                length as c_int,
                &mut transferred,
                ms,
            )
        };
        if maximum_ms > 500 {
            eprintln!("native USB initial input diagnostic: elapsed_ms={} timeout_ms={ms} code={code} transferred={transferred} requested={length}", started.elapsed().as_millis());
        }
        if code < 0 {
            // Preserve diagnostic evidence without interpreting acknowledged
            // bytes as success, exposing payloads, or retrying uncertain I/O.
            eprintln!("native USB bulk failure: endpoint=0x{endpoint:02x} code={code} transferred={transferred} requested={length}");
            return Err(code);
        }
        if transferred < 0 || transferred as usize > length {
            return Err(FOREIGN);
        }
        Ok(transferred as usize)
    }
}
impl Backend for NativeUsb {
    fn observe(&mut self) -> Result<Observation, i32> {
        self.guard()?;
        let cache = cached(
            &self.sysfs.join("descriptors"),
            crate::usb_descriptors::MAX_DESCRIPTOR_BYTES,
        )?;
        if cache != self.expected {
            return Err(FOREIGN);
        }
        let mut descriptor = DeviceDescriptor::default();
        let mut configuration = 0;
        let device = unsafe { libusb_get_device(self.handle) };
        if device.is_null() {
            return Err(FOREIGN);
        }
        unsafe {
            result(libusb_get_device_descriptor(device, &mut descriptor))?;
            if descriptor.bytes() != cache[..18]
                || libusb_get_bus_number(device) != self.bus
                || libusb_get_device_address(device) != self.address
                || libusb_get_device_speed(device) != 3
            {
                return Err(FOREIGN);
            }
            result(libusb_get_configuration(self.handle, &mut configuration))?;
        }
        if configuration != 1 || number(&self.sysfs.join("bConfigurationValue"), 10)? != 1 {
            return Err(FOREIGN);
        }
        let (alternate, driver) = self.interface(3)?;
        let kernel = unsafe { libusb_kernel_driver_active(self.handle, 3) };
        let kernel_bound = kernel_ownership(kernel, &driver, self.claimed)?;
        let mut audio_interfaces = Vec::new();
        for n in 0..3 {
            let (a, d) = self.interface(n)?;
            audio_interfaces.push((n, a, d));
        }
        self.guard()?;
        Ok(Observation {
            instance: self.stamp.sys_ino,
            descriptors: cache,
            configuration: 1,
            alternate,
            kernel_bound,
            audio_interfaces,
        })
    }
    fn claim_three(&mut self) -> Result<(), i32> {
        self.guard()?;
        unsafe { result(libusb_claim_interface(self.handle, 3)) }?;
        self.claimed = true;
        Ok(())
    }
    fn select_one(&mut self) -> Result<(), i32> {
        self.guard()?;
        unsafe { result(libusb_set_interface_alt_setting(self.handle, 3, 1)) }
    }
    fn select_zero(&mut self) -> Result<(), i32> {
        self.guard()?;
        if !self.claimed || self.interface(3)?.0 != 0 {
            return Err(FOREIGN);
        }
        unsafe { result(libusb_set_interface_alt_setting(self.handle, 3, 0)) }
    }
    fn clear_input_halt(&mut self) -> Result<(), i32> {
        self.guard()?;
        if !self.claimed || self.interface(3)?.0 != 1 {
            return Err(FOREIGN);
        }
        // Fixed vendor IN endpoint from the pinned descriptor profile. No
        // pending bulk I/O exists during preparation; never clear on failure.
        unsafe { result(libusb_clear_halt(self.handle, crate::session::IN_PIPE)) }
    }
    fn write(&mut self, bytes: &[u8], ms: u32) -> Result<usize, i32> {
        // libusb's C ABI is mutable for both directions; pass an owned copy so
        // no C function can mutate Rust's shared slice.
        let mut owned = bytes.to_vec();
        self.bulk(
            crate::session::OUT_PIPE,
            owned.as_mut_ptr(),
            owned.len(),
            ms,
            500,
        )
    }
    fn read(&mut self, bytes: &mut [u8], ms: u32) -> Result<usize, i32> {
        let maximum = if self.initial_read_used {
            500
        } else {
            self.initial_read_limit_ms
        };
        // Consume before calling the backend, including unknown/refused outcomes.
        self.initial_read_used = true;
        self.bulk(
            crate::session::IN_PIPE,
            bytes.as_mut_ptr(),
            bytes.len(),
            ms,
            maximum,
        )
    }
    fn release_three(&mut self) -> Result<(), i32> {
        // Retained handle/claim is the authority even if sysfs disappears.
        // Never target a replacement device. Release restores first alt (zero).
        unsafe { result(libusb_release_interface(self.handle, 3)) }?;
        self.claimed = false;
        Ok(())
    }
    fn close(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                libusb_close(self.handle);
                self.handle = ptr::null_mut();
            }
            // Wrapped fds are caller-owned: close only after libusb's handle.
            self.file.take();
            if !self.context.is_null() {
                libusb_exit(self.context);
                self.context = ptr::null_mut();
            }
        }
    }
}
impl Drop for NativeUsb {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usbfs_is_accepted_only_for_the_acknowledged_retained_claim() {
        assert_eq!(kernel_ownership(0, "", false), Ok(false));
        assert_eq!(kernel_ownership(0, "usbfs", true), Ok(false));
        assert!(kernel_ownership(0, "usbfs", false).is_err());
        assert!(kernel_ownership(0, "", true).is_err());
        assert_eq!(kernel_ownership(1, "snd_usb_audio", false), Ok(true));
        assert_eq!(kernel_ownership(1, "snd_usb_audio", true), Ok(true));
        assert!(kernel_ownership(1, "usbfs", true).is_err());
        assert!(kernel_ownership(1, "", false).is_err());
        assert!(kernel_ownership(0, "other", true).is_err());
        assert_eq!(kernel_ownership(-4, "", false), Err(-4));
    }
    #[test]
    fn public_abi_layout_and_runtime_version_without_device_access() {
        assert_eq!(std::mem::size_of::<DeviceDescriptor>(), 18);
        assert_eq!(std::mem::offset_of!(DeviceDescriptor, vendor), 8);
        assert_eq!(std::mem::offset_of!(DeviceDescriptor, configurations), 17);
        assert_eq!(std::mem::size_of::<InitOption>(), 16);
        assert_eq!(std::mem::offset_of!(InitOption, value), 8);
        assert_eq!(std::mem::size_of::<Version>(), 24);
        assert!(library_version() >= (1, 0, 27));
        assert!(matches!(
            open_handshake(Path::new("/unused"), &[], false),
            Err(OpenError::Refused)
        ));
        assert!(matches!(
            open_handshake_with_startup(Path::new("/unused"), &[], false, Startup::ObservedWindows),
            Err(OpenError::Refused)
        ));
    }
}
