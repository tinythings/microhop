// Analyser of the current system.
// This is used to update/refresh microhop on a current system,
// regenerating it after kernel update.

use std::{
    fs::File,
    io::{BufRead, BufReader, Error},
    path::PathBuf,
};

use kmoddep::{
    kerman::KernelInfo,
    modinfo::{lsmod, ModInfo},
};
use nix::sys::utsname::uname;
use profile::cfg::MhConfig;
use syslib::blk::{BlkDev, BlkInfo};

pub struct SysAnalyser {}

impl SysAnalyser {
    pub fn new() -> SysAnalyser {
        SysAnalyser {}
    }

    /// Get a device that are mounted to a root
    fn get_root_device_path(&self) -> Result<String, Error> {
        for data in BufReader::new(File::open("/proc/mounts")?).lines().flatten() {
            let mpt = data.split_whitespace().collect::<Vec<&str>>();
            if mpt[1].eq("/") {
                return Ok(mpt[0].to_string());
            }
        }

        Err(Error::new(std::io::ErrorKind::NotFound, "No root devices found"))
    }

    /// Look at currently running modules and find out
    /// main ones
    fn get_main_modules(&self, kinfo: &KernelInfo) -> Result<Vec<ModInfo>, Error> {
        Ok(lsmod().iter().filter(|mi| !kinfo.is_dep(&mi.name)).cloned().collect::<Vec<ModInfo>>())
    }

    /// Find current disks
    fn get_root_disk(&self) -> Result<BlkDev, Error> {
        let root_device = PathBuf::from(self.get_root_device_path()?);

        let mut blk = BlkInfo::new();
        blk.probe_devices()?;
        for d in blk.get_devices() {
            if d.get_path().eq(&root_device) {
                return Ok(d.to_owned());
            }
        }

        Err(Error::new(std::io::ErrorKind::NotFound, "No root disk has been found"))
    }

    /// Return a composed configuration
    pub fn get_config(&self, kinfos: Vec<KernelInfo>) -> Result<MhConfig, Error> {
        let krelease = uname()?.release().to_str().unwrap().to_string();
        let kinfo = kinfos.iter().find(|nfo| nfo.version.eq(&krelease));
        if kinfo.is_none() {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("Current kernel is {}, but no information has been found", krelease),
            ));
        }

        // Print out default config for further configuration
        let root_disk = self.get_root_disk()?;
        println!(
            "modules:\n{}\n\ndisks:\n  {}: {},/,rw\n\ninit: /sbin/init\nsysroot: /sysroot\nlog: debug\n",
            self.get_main_modules(kinfo.unwrap())?
                .iter()
                .map(|m| format!("  - {}", m.name))
                .collect::<Vec<String>>()
                .join("  \n"),
            root_disk.get_mount_criterion(),
            root_disk.get_fstype()
        );

        Ok(MhConfig::default())
    }
}
