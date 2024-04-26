mod logger;
mod rfsutils;

use crate::rfsutils::kmodprobe;
use nix::{sys::stat, unistd};
use std::{ffi::CString, io::Error, path::Path};

static VERSION: &str = "0.0.1";
static LOGGER: logger::STDOUTLogger = logger::STDOUTLogger;

/// Initialise logger
fn init(debug: &bool) -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(if *debug { log::LevelFilter::Trace } else { log::LevelFilter::Info }))
}

fn main() -> Result<(), Error> {
    if let Err(err) = init(&false) {
        return Err(Error::new(std::io::ErrorKind::Other, err.to_string()));
    }

    // Say hello
    log::info!("Welcome to the Microhop {}!", VERSION);

    // Load required modules
    let mpb = kmodprobe::KModProbe::new();
    #[allow(clippy::single_element_loop)] // this is a PoC
    for mname in ["virtio_blk"] {
        mpb.modprobe(mname);
    }

    // Create sysroot entry point
    let temp_mpt = "/sysroot";
    if !Path::new(temp_mpt).exists() {
        unistd::mkdir(temp_mpt, stat::Mode::S_IRUSR)?;
    }

    // Mount required dirs
    let mountpoints: Vec<(&str, &str, &str)> = vec![
        ("proc", "none", "/proc"), // Has to go always first
        ("sysfs", "none", "/sys"),
        ("devtmpfs", "devtmpfs", "/dev"),
        // External disks. Here just main for now
        ("ext4", "/dev/vda3", temp_mpt),
        // The other partitions should go like:
        // ("fstype", "/dev/device", temp_mpt + "/mountpoint"),
    ];

    for t in &mountpoints {
        match rfsutils::fs::mount(t.0, t.1, t.2) {
            Ok(_) => (),
            Err(err) => log::error!("Error: {}", err),
        }
    }

    for t in &mountpoints {
        if t.0 != "ext4" {
            rfsutils::fs::umount(t.2)?;
            rfsutils::fs::mount(t.0, t.1, format!("{}{}", temp_mpt, t.2).as_str())?;
        }
    }

    // Switch root
    rfsutils::fs::pivot(temp_mpt, "ext4")?;
    log::debug!("enter the main init");

    // Start external init
    unistd::execv(&CString::new("/usr/bin/bash").unwrap(), &Vec::<CString>::default())?;

    Ok(())
}
