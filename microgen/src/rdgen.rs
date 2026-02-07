use kmoddep::kerman::KernelInfo;
use profile::cfg::MhConfig;
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Error, ErrorKind::InvalidData, Write},
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
};

use crate::rdpack;

// Git commit hash embedded at build time
const GIT_COMMIT_HASH: &str = env!("GIT_COMMIT_HASH");

// Embed the microhop binary via compile-time env var
const MICROHOP: &[u8] = include_bytes!(env!("MICROHOP_BINARY_PATH", "microhop"));

// Embed e2fsck when fsck feature is enabled
#[cfg(feature = "fsck")]
const E2FSCK: &[u8] = include_bytes!(env!("E2FSCK_BINARY_PATH", "e2fsck"));

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
    /// Target kernel (optional: monolithic kernel has none)
    kinfo: Option<KernelInfo>,

    /// Profile (config)
    cfg: MhConfig,

    /// Destination where initramfs is going to be generated
    dst: PathBuf,

    /// Output filename path
    dst_fn: PathBuf,

    /// Module dependencies
    _kmod_d: Vec<String>,

    /// Main modules
    _kmod_m: Vec<String>,

    /// Firmware list file path
    firmware_list_path: Option<PathBuf>,

    /// Root filesystem path
    firmware_path: PathBuf,
}

impl IrfsGen {
    pub fn generate(
        kinfo: Option<&KernelInfo>, cfg: MhConfig, dst: PathBuf, fname: PathBuf, firmware_list_path: Option<PathBuf>,
        firmware_path: PathBuf,
    ) -> Result<(), Error> {
        if dst.exists() {
            return Err(Error::new(InvalidData, format!("Given destination path {:?} already exists", dst)));
        }

        fs::create_dir_all(&dst)?;

        let mut irfsg = IrfsGen {
            kinfo: kinfo.cloned(),
            cfg,
            dst,
            dst_fn: fname,
            _kmod_d: vec![],
            _kmod_m: vec![],
            firmware_list_path,
            firmware_path,
        };

        let kroot = irfsg.create_ramfs_dirs()?;
        irfsg.setup_microhop()?;

        if irfsg.kinfo.is_some() {
            irfsg.copy_kernel_modules(kroot.as_str())?;
        } else {
            println!("ℹ No kernel modules found, skipping module copy");
        }

        if irfsg.firmware_list_path.is_some() {
            irfsg.copy_firmware_files()?;
        }

        irfsg.write_boot_config()?;
        irfsg.pack()?;

        Ok(())
    }

    /// Copy microhop binary
    fn setup_microhop(&self) -> Result<(), Error> {
        let mhp = self.dst.join("bin/microhop");
        fs::write(&mhp, MICROHOP)?;
        let mut flags = fs::metadata(&mhp)?.permissions();
        flags.set_mode(0o755);
        fs::set_permissions(mhp, flags)?;

        #[cfg(feature = "fsck")]
        {
            let e2p = self.dst.join("bin/e2fsck");
            fs::write(&e2p, E2FSCK)?;
            let mut eflags = fs::metadata(&e2p)?.permissions();
            eflags.set_mode(0o755);
            fs::set_permissions(e2p, eflags)?;
        }

        // Symlink to /init
        let here = env::current_dir()?;
        env::set_current_dir(self.dst.as_path())?;
        symlink(Path::new("bin/microhop"), Path::new("init"))?;
        env::set_current_dir(here)?;

        Ok(())
    }

    /// Create directories for the ramfs.
    fn create_ramfs_dirs(&self) -> Result<String, Error> {
        let mut dirs: Vec<String> = vec![
            "bin".to_string(),
            "etc".to_string(),
            "proc".to_string(),
            "dev".to_string(),
            "sys".to_string(),
            self.cfg.get_sysroot_path().trim_start_matches('/').to_string(),
        ];

        let kroot = if let Some(kinfo) = &self.kinfo {
            let kr = format!("lib/modules/{}", kinfo.get_kernel_path().file_name().unwrap().to_str().unwrap());
            dirs.push(kr.clone());
            kr
        } else {
            String::new()
        };

        for d in dirs {
            fs::create_dir_all(self.dst.join(d))?;
        }

        Ok(kroot)
    }

    /// This will find what modules are needed in the source kernel and will copy to the target only those
    fn copy_kernel_modules(&mut self, kroot: &str) -> Result<(), Error> {
        let kinfo = match &self.kinfo {
            Some(k) => k,
            None => return Ok(()),
        };

        // First get only main modules, and then get dependencies for them
        for (kmod, kmod_deps) in kinfo.get_deps_for(
            &self
                .cfg
                .get_modules()
                .iter()
                .filter(|e| !kinfo.is_dep(e))
                .collect::<HashSet<_>>()
                .into_iter()
                .cloned()
                .collect::<Vec<String>>(),
        ) {
            self._copy_kmod(&kmod, kroot)?;
            self._kmod_m.push(kmod);
            if !kmod_deps.is_empty() {
                for kd in kmod_deps {
                    self._copy_kmod(kd.as_str(), kroot)?;
                    if !self._kmod_m.contains(&kd) {
                        self._kmod_d.push(kd);
                    }
                }
            }
        }
        Ok(())
    }

    /// Copy one kernel module
    fn _copy_kmod(&self, kmod: &str, kroot: &str) -> Result<(), Error> {
        let kinfo = match &self.kinfo {
            Some(k) => k,
            None => return Ok(()),
        };

        let msrc = kinfo.get_kernel_path().join(kmod);
        let mdst = self.dst.join(kroot).join(kmod);

        fs::create_dir_all(mdst.as_path().parent().unwrap())?;
        fs::copy(msrc, mdst)?;

        Ok(())
    }

    /// Copy firmware files based on firmware list file
    fn copy_firmware_files(&self) -> Result<(), Error> {
        let firmware_list_path = match &self.firmware_list_path {
            Some(p) => p,
            None => return Ok(()),
        };

        let file = File::open(firmware_list_path).map_err(|e| {
            Error::new(std::io::ErrorKind::NotFound, format!("Failed to open firmware list file {:?}: {}", firmware_list_path, e))
        })?;
        let reader = BufReader::new(file);
        let firmware_base = self.cfg.get_firmware_base();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse SOURCE_PATH:DESTINATION_PATH
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(Error::new(
                    InvalidData,
                    format!("Invalid firmware entry format at line {}: '{}' (expected SOURCE:DEST)", line_num + 1, line),
                ));
            }

            let source_path = self.firmware_path.join(parts[0].trim());
            let dest_rel_path = parts[1].trim();
            let dest_path = self.dst.join(firmware_base.trim_start_matches('/')).join(dest_rel_path);

            if !source_path.exists() {
                return Err(Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Firmware file not found at line {}: {:?}", line_num + 1, source_path),
                ));
            }

            let real_source = if source_path.is_symlink() {
                let target = fs::read_link(&source_path).map_err(|e| {
                    Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Failed to read symlink at line {}: {:?} - {}", line_num + 1, source_path, e),
                    )
                })?;

                let resolved = if target.is_absolute() { target } else { source_path.parent().unwrap().join(target) };

                if !resolved.exists() {
                    return Err(Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Broken symlink at line {}: {:?} -> {:?}", line_num + 1, source_path, resolved),
                    ));
                }
                resolved
            } else {
                source_path.clone()
            };

            if dest_path.exists() {
                return Err(Error::new(
                    InvalidData,
                    format!("Destination firmware file already exists at line {}: {:?}", line_num + 1, dest_path),
                ));
            }

            let metadata = fs::metadata(&real_source).map_err(|e| {
                Error::other(format!("Failed to get metadata at line {}: {:?} - {}", line_num + 1, real_source, e))
            })?;

            let size = metadata.len();

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(&real_source, &dest_path).map_err(|e| {
                Error::other(format!(
                    "Failed to copy firmware file at line {}: {:?} -> {:?} - {}",
                    line_num + 1,
                    real_source,
                    dest_path,
                    e
                ))
            })?;

            // Set read-only permissions
            let mut perms = fs::metadata(&dest_path)?.permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&dest_path, perms)?;

            println!("Firmware: {} ({} bytes)", dest_rel_path, size);
        }

        Ok(())
    }

    /// Write boot config
    fn write_boot_config(&self) -> Result<(), Error> {
        let f = File::create(self.dst.join("etc/microhop.conf"))?;
        let mut fp = BufWriter::new(f);

        // Write generation metadata as YAML fields
        let now = chrono::Local::now();
        writeln!(fp, "# Generated by microgen")?;
        writeln!(fp, "metadata:")?;
        writeln!(fp, "  git_commit: {}", GIT_COMMIT_HASH)?;
        writeln!(fp, "  generated_at: {}", now.format("%Y-%m-%d %H:%M:%S %Z"))?;
        writeln!(fp)?;

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
                .map(|i| format!("  - {}", Path::new(i).file_stem().unwrap().to_str().unwrap().split('.').next().unwrap()))
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

        // Preserve cmdline mask entries
        let mask = self.cfg.get_mask_cmdline();
        if !mask.is_empty() {
            writeln!(fp, "\nmask_cmdline:")?;
            for m in mask {
                writeln!(fp, "  - {}", m)?;
            }
        }

        // Preserve overlayfs configuration if present
        if let Some(overlay) = self.cfg.get_overlayfs() {
            writeln!(fp, "\noverlayfs:")?;
            writeln!(fp, "  device: {}", overlay.device)?;
            writeln!(fp, "  upper: {}", overlay.upper)?;
            writeln!(fp, "  workdir: {}", overlay.workdir)?;
        }

        if let Some(firmware) = self.cfg.get_firmware() {
            writeln!(fp, "\nfirmware:")?;
            writeln!(fp, "  base: {}", firmware.base)?;
        }

        // Preserve NixOS configuration if present
        #[cfg(feature = "nixos")]
        if let Some(nixos) = self.cfg.get_nixos() {
            writeln!(fp, "\nnixos:")?;
            writeln!(fp, "  nix_path: {}", nixos.nix_path)?;
        }

        fp.flush()?;
        Ok(())
    }

    /// Pack to the CPIO
    fn pack(&self) -> Result<(), Error> {
        let here = env::current_dir()?;
        env::set_current_dir(self.dst.as_path())?;

        let out = self.dst_fn.as_os_str().to_str().unwrap();
        println!("Writing the initramfs to {:?}", out);
        rdpack::pack(out)?;

        env::set_current_dir(here)?;
        fs::remove_dir_all(&self.dst)?;

        println!("Done");
        Ok(())
    }
}
