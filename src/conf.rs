use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind},
    path::{Path, PathBuf},
};

/// Path to the config. It should be always only there.
static CFG_PATH: &str = "/etc/microhop.conf";

/// Disk description
pub struct MhConfDisk {
    device: String,
    fstype: String,
    path: String,
    mode: String,
}

impl MhConfDisk {
    /// Return path to the disk storage device
    pub fn get_device(&self) -> &str {
        &self.device
    }

    /// Return filesystem type
    pub fn get_fstype(&self) -> &str {
        &self.fstype
    }

    /// Return path to the mountpoint
    pub fn as_pathbuf(&self) -> PathBuf {
        PathBuf::from(&self.get_mountpoint())
    }

    /// Get raw mountpoint
    pub fn get_mountpoint(&self) -> &str {
        &self.path
    }

    /// Get mounting mode
    pub fn get_mode(&self) -> &str {
        &self.mode
    }
}

/// Main configuration struct
#[derive(Debug, Serialize, Deserialize)]
pub struct MhConfig {
    modules: Vec<String>,
    disks: IndexMap<String, String>,
    init: Option<String>,
    sysroot: Option<String>,
    log: Option<String>,
}

impl MhConfig {
    /// Return list of modules
    pub fn get_modules(&self) -> &[String] {
        &self.modules
    }

    /// Parse disk options, return default
    fn get_disk_opts(&self, opts: &str) -> Result<(String, String, String), Error> {
        let t = opts.split(',').collect::<Vec<&str>>();
        match t.len() {
            n if n > 1 && n < 3 => Ok((t[0].to_string(), t[1].to_string(), "rw".to_string())),
            3 => Ok((t[0].to_string(), t[1].to_string(), t[2].to_string())),
            _ => Err(Error::new(ErrorKind::InvalidData, format!("Disk options are incorrect: {}", opts))),
        }
    }

    /// Return disk device description
    pub fn get_disks(&self) -> Result<Vec<MhConfDisk>, Error> {
        let mut d: Vec<MhConfDisk> = Vec::default();
        for (dev, opt) in &self.disks {
            let (fstype, path, mode) = self.get_disk_opts(opt)?;
            d.push(MhConfDisk {
                device: dev.to_string(),
                fstype: fstype.to_string(),
                path: path.to_string(),
                mode: mode.to_string(),
            });
        }

        Ok(d)
    }

    /// Return path to the init app
    pub fn get_init_path(&self) -> String {
        if let Some(init) = &self.init {
            return init.to_owned();
        }

        "/sysroot".to_string()
    }

    /// Get log level
    pub fn get_log_level(&self) -> log::LevelFilter {
        if let Some(l) = &self.log {
            return match l.as_str() {
                "debug" => log::LevelFilter::Debug,
                "quiet" => log::LevelFilter::Off,
                _ => log::LevelFilter::Info,
            };
        }

        log::LevelFilter::Info
    }

    /// Get a sysroot temp path
    pub fn get_sysroot_path(&self) -> String {
        self.sysroot.to_owned().unwrap_or("/sysroot".to_string())
    }
}

/// Get the configuration
pub fn get_mh_config() -> Result<MhConfig, Error> {
    let p = Path::new(CFG_PATH);
    if !p.exists() {
        return Err(Error::new(ErrorKind::NotFound, format!("Configuration file {} is missing", CFG_PATH)));
    }

    match serde_yaml::from_reader(BufReader::new(File::open(p)?)) {
        Ok(cfg) => Ok(cfg),
        Err(err) => Err(Error::new(std::io::ErrorKind::InvalidData, err)),
    }
}
