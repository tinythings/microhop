mod kmodprobe;
mod logger;

use nix::{mount::MsFlags, sys::stat, unistd};
use std::{ffi::CString, io::Error, path::Path};
use sys_mount::Mount;

static VERSION: &str = "0.0.1";
static LOGGER: logger::STDOUTLogger = logger::STDOUTLogger;

/// Initialise logger
fn init(debug: &bool) -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(if *debug { log::LevelFilter::Trace } else { log::LevelFilter::Info }))
}

fn naive_mount(fstype: &str, dev: &str, mpt: &str) -> Result<(), Error> {
    if let Err(err) = Mount::builder().fstype(fstype).mount(dev, mpt) {
        return Err(Error::new(std::io::ErrorKind::NotConnected, format!("Failed to mount {}: {}", fstype, err)));
    } else {
        log::info!("Mounted {} at {} as {}", dev, mpt, fstype);
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    if let Err(err) = init(&false) {
        return Err(Error::new(std::io::ErrorKind::Other, err.to_string()));
    }

    // Say hello
    log::info!("Microjump {}", VERSION);

    // Load required modules
    let mpb = kmodprobe::KModProbe::new();
    #[allow(clippy::single_element_loop)] // this is a PoC
    for mname in ["virtio_blk"] {
        mpb.modprobe(mname);
    }

    // Create sysroot entry point
    let mj_rfsp = "/sysroot";
    if !Path::new(mj_rfsp).exists() {
        unistd::mkdir(mj_rfsp, stat::Mode::S_IRUSR)?;
    }

    // Mount required dirs
    let mountpoints: Vec<(&str, &str, &str)> = vec![
        ("proc", "none", "/proc"), // Has to go always first
        ("sysfs", "none", "/sys"),
        ("devtmpfs", "devtmpfs", "/dev"),
        // External disks. Here just main for now
        ("ext4", "/dev/vda3", mj_rfsp),
    ];

    for t in &mountpoints {
        match naive_mount(t.0, t.1, t.2) {
            Ok(_) => (),
            Err(err) => log::error!("Error: {}", err),
        }
    }

    for t in &mountpoints {
        if t.0 != "ext4" {
            nix::mount::umount(t.2)?;
            naive_mount(t.0, t.1, format!("{}{}", mj_rfsp, t.2).as_str())?;
        }
    }

    // Switch root
    nix::unistd::chdir(mj_rfsp)?;
    nix::mount::mount(Some("."), "/", Some("ext4"), MsFlags::MS_MOVE, Option::<&str>::None)?;
    nix::unistd::chroot(".")?;

    // Start external init
    unistd::execv(&CString::new("/usr/bin/bash").unwrap(), &Vec::<CString>::default())?;

    Ok(())
}
