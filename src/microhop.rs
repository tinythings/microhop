use crate::rfsutils::{self, blk::BlkInfo};
use lazy_static::lazy_static;
use profile::cfg::MhConfig;
use std::io::Error;
use uuid::Uuid;

static VERSION: &str = "0.0.7";

// Mount required system dirs
lazy_static! {
    pub static ref SYS_MPT: Vec<(String, String, String)> = vec![
        ("proc".into(), "none".into(), "/proc".into()), // Has to go always first
        ("sysfs".into(), "none".into(), "/sys".into()),
        ("devtmpfs".into(), "devtmpfs".into(), "/dev".into())
    ];
}

// Initial greetings
pub fn greet(cfg: &MhConfig) -> Result<(), Error> {
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

/// Mount configured filesystems in a batch
pub fn mount_fs(filesystems: &Vec<(String, String, String)>) {
    for t in filesystems {
        match rfsutils::fs::mount(&t.0, &t.1, &t.2) {
            Ok(_) => (),
            Err(err) => log::error!("Error mounting {}: {}", t.2, err),
        }
    }
}

#[allow(clippy::type_complexity)]
/// Get block devices
pub fn get_blk_devices(cfg: &MhConfig) -> Result<(String, Vec<(String, String, String)>), Error> {
    let mut root_fstype = String::new();
    let mut blkid = BlkInfo::new();

    blkid.probe_devices()?;

    for d in blkid.get_devices() {
        if !d.get_fstype().is_empty() {
            log::info!("{} partition at {:?} ({}) \"{}\"", d.get_fstype(), d.get_path(), d.get_uuid(), d.get_label());
        }
    }

    let mut blk_mpt: Vec<(String, String, String)> = Vec::default();
    for dev in cfg.get_disks()? {
        let mpt = dev.get_mountpoint().trim_end_matches('/').to_string();
        if mpt.is_empty() && root_fstype.is_empty() {
            root_fstype = dev.get_fstype().into();
        }

        let mut devpath = "";
        if Uuid::parse_str(dev.get_device()).is_ok() {
            if let Some(blkdev) = blkid.by_uuid(dev.get_device()) {
                devpath = blkdev.get_path().to_str().unwrap();
            }
        } else if dev.get_device().starts_with("/dev") {
            devpath = dev.get_device();
        } else {
            // label
            if let Some(blkdev) = blkid.by_label(dev.get_device()) {
                devpath = blkdev.get_path().to_str().unwrap();
            }
        }

        if devpath.is_empty() {
            log::warn!("Unknown device: {}", dev.get_device());
        } else {
            blk_mpt.push((dev.get_fstype().into(), devpath.into(), format!("{}{}", &cfg.get_sysroot_path(), mpt)));
        }
    }

    // Sort mountpoints, so the "/" goes always first
    blk_mpt.sort_by(|ela, elb| ela.1.cmp(&elb.1));

    Ok((root_fstype, blk_mpt))
}
