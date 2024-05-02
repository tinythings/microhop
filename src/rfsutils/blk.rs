use libblkid_rs::BlkidProbe;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
};

/// Block device metadata.
/// This contains its path, UUID, size and other info
pub struct BlkDev {
    path: PathBuf,
    uuid: String,
    fstype: String,
}

impl BlkDev {
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get_uuid(&self) -> &str {
        &self.uuid
    }

    pub fn get_fstype(&self) -> &str {
        &self.fstype
    }
}

/// Block device manager
pub struct BlkInfo {
    devices: Vec<BlkDev>,
}

#[allow(dead_code)]
impl BlkInfo {
    pub fn new() -> Self {
        BlkInfo { devices: Vec::default() }
    }

    /// Return device stats.
    /// This specifically gets only device names
    fn load_dev_stats(&mut self) -> Result<Vec<String>, Error> {
        let mut stats: Vec<String> = Vec::default();

        for l in BufReader::new(File::open("/proc/diskstats")?).lines().flatten() {
            let d: Vec<&str> = l.split_whitespace().collect();
            if d.len() > 2 {
                stats.push(d[2].to_owned());
            }
        }

        Ok(stats)
    }

    /// Load block devices for the given physical device
    fn load_blk_device(&mut self, dev: &str, stats: &Vec<String>) -> Result<(), Error> {
        log::debug!("Probing \"{}\" device", dev);
        for devname in stats {
            // Get only partitions, omit the physical device
            if devname.starts_with(dev) && !devname.eq(dev) {
                let dev = format!("/dev/{}", devname);
                let blkid = self.blk_id(dev.as_str())?; // uuid, fstype
                self.devices.push(BlkDev { path: PathBuf::from(dev), uuid: blkid.0, fstype: blkid.1 });
            }
        }

        Ok(())
    }

    fn blk_id(&self, dev: &str) -> Result<(String, String), Error> {
        let mut uuid = "".to_string();
        let mut fstype = "".to_string();

        if let Ok(mut pb) = BlkidProbe::new_from_filename(Path::new(dev)) {
            pb.enable_superblocks(true).unwrap_or_default();
            pb.enable_partitions(true).unwrap_or_default();
            pb.do_safeprobe().unwrap();

            if let Ok(disk_id) = pb.lookup_value("UUID") {
                uuid = disk_id;
            }

            if let Ok(disk_fst) = pb.lookup_value("TYPE") {
                fstype = disk_fst;
            }
        }

        Ok((uuid, fstype))
    }

    /// Find all currently available devices
    pub fn probe_devices(&mut self) -> Result<(), Error> {
        let stats = self.load_dev_stats()?;
        for dname in fs::read_dir("/sys/block")? {
            self.load_blk_device(dname?.file_name().into_string().unwrap().as_str(), &stats)?;
        }

        Ok(())
    }

    /// Resolve device by UUID
    pub fn by_uuid(&self, id: &str) -> Option<&BlkDev> {
        self.devices.iter().find(|&d| d.uuid.eq(id))
    }

    /// Resolve device by /dev/<device> path
    pub fn by_path(&self, p: &str) -> Option<&BlkDev> {
        self.devices.iter().find(|&d| d.path.eq(&PathBuf::from(p)))
    }

    /// Return all known block devices
    pub fn get_devices(&self) -> &Vec<BlkDev> {
        &self.devices
    }
}
