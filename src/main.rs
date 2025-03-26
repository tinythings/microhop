mod kmodprobe;
mod logger;
mod microhop;

use crate::microhop::{get_blk_devices, greet, mount_fs, SYS_MPT};
use nix::{mount::MsFlags, sys::stat, unistd};
use std::{ffi::CString, io::Error, path::Path};

static LOGGER: logger::STDOUTLogger = logger::STDOUTLogger;

fn main() -> Result<(), Error> {
    // Set logger
    let cfg = profile::cfg::get_mh_config(None)?;
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
        log::debug!("Init sysroot path: {}", temp_mpt);
    }

    mount_fs(&SYS_MPT);

    let (root_fstype, blk_mpt) = get_blk_devices(&cfg)?;
    if root_fstype.is_empty() {
        log::error!("Type of the root filesystem was not detected. Please double-check the configuration!");
    }
    mount_fs(&blk_mpt);

    // Remount sysfs, switch root
    log::debug!("switching root");
    for t in &*SYS_MPT {
        let tgt = format!("{}{}", temp_mpt, t.2);
        nix::mount::mount(Some(t.2.as_str()), tgt.as_str(), Some(t.0.as_str()), MsFlags::MS_MOVE, Option::<&str>::None)?;
    }

    // Pivot the system
    syslib::fs::pivot(temp_mpt, root_fstype.as_str())?;

    // Start external init
    log::info!("Launching init at {}", cfg.get_init_path());

    let argv: Vec<CString> = vec![CString::new(cfg.get_init_path()).unwrap()];

    #[allow(irrefutable_let_patterns)]
    if let Err(err) = unistd::execv(&CString::new(cfg.get_init_path()).unwrap(), &argv) {
        log::error!("{:?}", err);
    }

    Ok(())
}
