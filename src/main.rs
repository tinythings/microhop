mod conf;
mod logger;
mod rfsutils;

use crate::rfsutils::kmodprobe;
use conf::MhConfig;
use nix::{sys::stat, unistd};
use std::{ffi::CString, io::Error, path::Path};

static VERSION: &str = "0.0.1";
static LOGGER: logger::STDOUTLogger = logger::STDOUTLogger;

// Initial greetings
fn greet(cfg: &MhConfig) -> Result<(), Error> {
    // Say hello
    log::info!("Welcome to the Microhop {}!", VERSION);

    // Debug itsel
    log::debug!("Init program: {}", cfg.get_init_path());
    for dsk in cfg.get_disks()? {
        log::debug!(
            "Disk device: {}, fs type: {}, mountpoint: {:?}, mode: {}",
            dsk.get_device(),
            dsk.get_fstype(),
            dsk.as_pathbuf(),
            dsk.get_mode()
        );
    }
    log::debug!("Kernel modules:");
    for m in cfg.get_modules() {
        log::debug!("- {}", m);
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    // Set logger
    let cfg = conf::get_mh_config()?;
    log::set_logger(&LOGGER).map(|()| log::set_max_level(cfg.get_log_level())).unwrap();

    greet(&cfg)?;

    // Load required modules
    let mpb = kmodprobe::KModProbe::new();
    for mname in cfg.get_modules() {
        mpb.modprobe(mname);
    }
    if !cfg.get_modules().is_empty() {
        log::info!("loaded required kernel modules");
    }

    // Create sysroot entry point
    let temp_mpt = &cfg.get_sysroot_path();
    if !Path::new(temp_mpt).exists() {
        unistd::mkdir(temp_mpt.as_str(), stat::Mode::S_IRUSR)?;
        log::debug!("Temp path: {}", temp_mpt);
    }

    // Mount required dirs
    let mut mountpoints: Vec<(String, String, String)> = vec![
        ("proc".into(), "none".into(), "/proc".into()), // Has to go always first
        ("sysfs".into(), "none".into(), "/sys".into()),
        ("devtmpfs".into(), "devtmpfs".into(), "/dev".into()),
    ];

    let mut root_fstype = String::new();
    for dev in cfg.get_disks()? {
        let mpt = dev.get_mountpoint().trim_end_matches('/').to_string();
        if mpt.is_empty() {
            root_fstype = dev.get_fstype().into();
        }
        mountpoints.push((dev.get_fstype().into(), dev.get_device().into(), format!("{}{}", temp_mpt, mpt)));
    }

    assert!(!root_fstype.is_empty(), "Filesystem type for root was not found");

    for t in &mountpoints {
        match rfsutils::fs::mount(&t.0, &t.1, &t.2) {
            Ok(_) => (),
            Err(err) => log::error!("Error mounting {}: {}", t.2, err),
        }
    }

    let sysfs = ["proc", "sysfs", "devtmpfs"];
    for t in &mountpoints {
        if sysfs.contains(&t.0.as_str()) {
            rfsutils::fs::umount(&t.2)?;
            rfsutils::fs::mount(&t.0, &t.1, format!("{}{}", temp_mpt, t.2).as_str())?;
        }
    }

    // Switch root
    rfsutils::fs::pivot(temp_mpt, root_fstype.as_str())?;
    log::debug!("enter the main init");

    // Start external init
    unistd::execv(&CString::new(cfg.get_init_path()).unwrap(), &Vec::<CString>::default())?;

    Ok(())
}
