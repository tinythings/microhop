use kmoddep::kerman::KernelInfo;
use profile::cfg::MhConfig;
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Error, ErrorKind::InvalidData, Write},
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
};

const MICROHOP: &[u8] = include_bytes!("microhop");

const BLINKENLICHTEN: &str = "# Achtung Alles Lookenskepers!
#
# Das konfiguration ist nicht fuer gefingerpoken und
# mittengrabben. Ist easy das machine schnappen der springenwerk,
# blowenfusen und poppencorken mit spitzensparken. Das rubbernecken
# sichtseeren keepen das cotten-pickenen hands in das pockets
# muss.
#
# Relaxen und watchen das blinkenlichten.";

pub struct IrfsGen {
    /// Target kernel
    kinfo: KernelInfo,

    /// Profile (config)
    cfg: MhConfig,

    /// Destination where initramfs is going to be generated
    dst: PathBuf,

    /// Module dependencies
    _kmod_d: Vec<String>,

    /// Main modules
    _kmod_m: Vec<String>,
}

impl IrfsGen {
    pub fn generate(kinfo: &KernelInfo, cfg: MhConfig, dst: PathBuf) -> Result<(), Error> {
        if !dst.exists() {
            fs::create_dir_all(&dst)?;
        } else {
            return Err(Error::new(InvalidData, format!("Given destination path {:?} already exists", dst)));
        }

        let mut irfsg = IrfsGen { kinfo: kinfo.to_owned(), cfg, dst, _kmod_d: vec![], _kmod_m: vec![] };

        let kroot = irfsg.create_ramfs_dirs()?;
        irfsg.setup_microhop()?;
        irfsg.copy_kernel_modules(kroot.as_str())?;
        irfsg.write_boot_config()?;

        Ok(())
    }

    /// Copy microhop binary
    fn setup_microhop(&self) -> Result<(), Error> {
        let mhp = self.dst.join("bin/microhop");
        fs::write(&mhp, MICROHOP)?;
        let mut flags = fs::metadata(&mhp)?.permissions();
        flags.set_mode(0o755);
        fs::set_permissions(mhp, flags)?;

        // Symlink to /init
        let here = env::current_dir()?;
        env::set_current_dir(self.dst.as_path())?;
        symlink(Path::new("bin/microhop"), Path::new("init"))?;
        env::set_current_dir(here)?;

        Ok(())
    }

    /// Create directories for the ramfs.
    fn create_ramfs_dirs(&self) -> Result<String, Error> {
        let kroot = format!("lib/modules/{}", self.kinfo.get_kernel_path().as_path().file_name().unwrap().to_str().unwrap());
        for d in ["bin", "etc", "proc", "dev", "sys", self.cfg.get_sysroot_path().trim_start_matches("/"), kroot.as_str()] {
            fs::create_dir_all(self.dst.join(d.trim_start_matches('/')))?;
        }
        Ok(kroot)
    }

    /// This will find what modules are needed in the source kernel and will copy to the target only those
    fn copy_kernel_modules(&mut self, kroot: &str) -> Result<(), Error> {
        let dtree = self.kinfo.get_deps_for(self.cfg.get_modules());
        for (kmod, kmod_deps) in dtree {
            self._copy_kmod(&kmod, kroot)?;
            self._kmod_m.push(kmod);
            if !kmod_deps.is_empty() {
                for kd in kmod_deps {
                    self._copy_kmod(kd.as_str(), kroot)?;
                    self._kmod_d.push(kd);
                }
            }
        }
        Ok(())
    }

    /// Copy one kernel module
    fn _copy_kmod(&self, kmod: &str, kroot: &str) -> Result<(), Error> {
        let msrc = self.kinfo.get_kernel_path().join(kmod);
        let mdst = self.dst.join(kroot).join(kmod);

        fs::create_dir_all(mdst.as_path().parent().unwrap())?;
        fs::copy(&msrc, &mdst)?;

        println!("Copy from {:?}  ->  {:?}", msrc, mdst);

        Ok(())
    }

    /// Write boot config
    fn write_boot_config(&self) -> Result<(), Error> {
        let f = File::create(self.dst.join("etc/microhop.conf"))?;
        let mut fp = BufWriter::new(f);

        // Blinkenlichten :)
        writeln!(fp, "{}\n", BLINKENLICHTEN)?;

        // Write modules in the following order:
        //   1. First dependencies
        //   2. Main modules
        writeln!(
            fp,
            "modules:\n{}\n",
            self._kmod_d
                .iter()
                .chain(self._kmod_m.iter())
                .map(|i| format!(
                    "  - {}",
                    Path::new(i).file_stem().unwrap().to_str().unwrap().to_string().split_once('.').unwrap().0.to_string()
                ))
                .collect::<Vec<String>>()
                .join("\n")
        )?;

        // Write disks configuration
        writeln!(fp, "disks:")?;
        for d in self.cfg.get_disks()? {
            writeln!(fp, "  {}: {},{},{}", d.get_device(), d.get_fstype(), d.get_mountpoint(), d.get_mode())?;
        }
        writeln!(fp)?;

        // Transfer other options
        writeln!(fp, "init: {}", self.cfg.get_init_path())?;
        writeln!(fp, "sysroot: {}", self.cfg.get_sysroot_path())?;

        if let Some(l) = self.cfg.get_log_level_as_str() {
            writeln!(fp, "log: {}", l)?;
        }

        fp.flush()?;
        Ok(())
    }

        Ok(())
    }
}
