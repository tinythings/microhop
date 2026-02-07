use profile::cfg::MhConfig;
use std::io::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use syslib::blk::BlkInfo;
use uuid::Uuid;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct SystemDir<T: AsRef<str>> {
    pub fstype: T,
    pub dev: T,
    pub dst: T,
    pub mode: Option<T>,
}

impl<T: AsRef<str>> SystemDir<T> {
    const fn new(fstype: T, dev: T, dst: T) -> Self {
        Self { fstype, dev, dst, mode: None }
    }

    pub fn new_with_mode(fstype: T, dev: T, dst: T, mode: T) -> Self {
        Self { fstype, dev, dst, mode: Some(mode) }
    }
}

// Mount required system dirs
pub const SYS_MPT: &[SystemDir<&'static str>] = &[
    // Has to go always first
    SystemDir::new("proc", "none", "/proc"),
    SystemDir::new("sysfs", "none", "/sys"),
    SystemDir::new("devtmpfs", "devtmpfs", "/dev"),
    SystemDir::new("tmpfs", "tmpfs", "/run"),
];

// Initial greetings
pub fn greet(cfg: &MhConfig) -> Result<(), Error> {
    // Say hello
    log::info!("Welcome to the Microhop {}!", VERSION);

    // Debug itself
    log::debug!("Init program: {}", cfg.get_init_path());

    // Log kernel cmdline
    if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
        log::debug!("Kernel cmdline: {}", cmdline.trim());
    } else {
        log::warn!("Could not read /proc/cmdline");
    }

    let mask = cfg.get_mask_cmdline();
    if !mask.is_empty() {
        log::debug!("Masking kernel cmdline parameters: {:?}", mask);
    } else {
        log::debug!("No cmdline parameters to mask");
    }

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

/// Run embedded /bin/e2fsck directly instead of using in-process fsck
fn run_external_e2fsck(device: &str) -> Result<i32, String> {
    let e2fsck_path = "/bin/e2fsck";
    if !Path::new(e2fsck_path).exists() {
        return Err(format!("{} not found in initramfs", e2fsck_path));
    }

    log::info!("Starting filesystem check on {}", device);
    log::info!("This may take several minutes for corrupted filesystems...");

    let mut cmd = Command::new(e2fsck_path);
    // Use -y (answer yes to all) for severe corruption
    // -f forces check even if filesystem seems clean
    // -v for verbose output
    cmd.arg("-y");
    cmd.arg("-f");
    cmd.arg(device);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    match cmd.status() {
        Ok(st) => match st.code() {
            Some(c) => {
                log::info!("e2fsck finished with exit code {}", c);
                Ok(c)
            }
            None => Err("e2fsck terminated by signal".to_string()),
        },
        Err(e) => Err(format!("failed to execute /bin/e2fsck: {}", e)),
    }
}

/// Mount configured filesystems in a batch
/// Returns Ok(true) if all mounts succeeded, Ok(false) if some failed
pub fn mount_fs<T: AsRef<str>>(filesystems: &[SystemDir<T>]) -> Result<bool, Error> {
    use nix::mount::MsFlags;
    use std::fs::DirBuilder;
    use std::os::unix::fs::DirBuilderExt;
    let mut all_success = true;

    for t in filesystems {
        let mut flags = MsFlags::MS_NOATIME;
        if let Some(mode) = &t.mode {
            if mode.as_ref() == "ro" {
                flags |= MsFlags::MS_RDONLY;
            }
        }

        // Ensure the mount point directory exists before mounting
        let dst_path = Path::new(t.dst.as_ref());
        if !dst_path.exists() {
            log::debug!("Creating mount point directory: {}", t.dst.as_ref());
            if let Err(e) = DirBuilder::new().recursive(true).mode(0o755).create(dst_path) {
                log::error!("Failed to create mount point directory {}: {}", t.dst.as_ref(), e);
                all_success = false;
                continue;
            }
        }

        let mount_result = syslib::fs::mount_with_flags(t.fstype.as_ref(), t.dev.as_ref(), t.dst.as_ref(), flags);

        if let Err(err) = mount_result {
            log::error!("Failed to mount {} ({}): {}", t.dst.as_ref(), t.dev.as_ref(), err);
            log::warn!("Assuming filesystem corruption, attempting repair...");

            // Attempt filesystem check - only for ext filesystems currently
            let fstype = t.fstype.as_ref();
            if fstype.starts_with("ext") {
                log::info!("Running e2fsck on {}", t.dev.as_ref());
                match run_external_e2fsck(t.dev.as_ref()) {
                    Ok(code) => {
                        log::info!("e2fsck completed with exit code {}", code);
                        // Exit codes: 0=no errors, 1=errors corrected, 2=system should reboot
                        if code == 0 || code == 1 {
                            log::info!("Filesystem check successful, retrying mount...");
                            match syslib::fs::mount_with_flags(t.fstype.as_ref(), t.dev.as_ref(), t.dst.as_ref(), flags) {
                                Ok(_) => {
                                    log::info!("Successfully mounted {} after fsck", t.dst.as_ref());
                                }
                                Err(err2) => {
                                    log::error!("Mount still failed after fsck: {}", err2);
                                    all_success = false;
                                }
                            }
                        } else {
                            log::error!("e2fsck reported errors (exit code {}), mount may be unsafe", code);
                            all_success = false;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to run e2fsck: {}", e);
                        log::error!("Cannot verify filesystem integrity");
                        all_success = false;
                    }
                }
            } else {
                log::error!("No fsck available for filesystem type: {}", fstype);
                log::error!("Manual intervention may be required");
                all_success = false;
            }
        } else {
            log::debug!("Mounted {} at {} with flags {:?}", t.dev.as_ref(), t.dst.as_ref(), flags);
        }
    }

    Ok(all_success)
}

/// Parse device specification and resolve to device path
/// Supports: uuid=<UUID>, label=<LABEL>, /dev/path, plain UUID, plain label
pub fn resolve_device_path<'a>(device_spec: &'a str, blkid: &'a BlkInfo) -> Option<&'a str> {
    if device_spec.starts_with("uuid=") || device_spec.to_lowercase().starts_with("uuid=") {
        let uuid = &device_spec[5..];

        if let Some(blkdev) = blkid.by_uuid(uuid) {
            return blkdev.get_path().to_str();
        }
    } else if device_spec.starts_with("label=") || device_spec.to_lowercase().starts_with("label=") {
        let label = &device_spec[6..];

        if let Some(blkdev) = blkid.by_label(label) {
            return blkdev.get_path().to_str();
        }
    } else if device_spec.starts_with("/dev/") {
        // Use device path directly without adding any suffix
        return Some(device_spec);
    } else if Uuid::parse_str(device_spec).is_ok() {
        // Plain UUID (for backward compatibility)
        if let Some(blkdev) = blkid.by_uuid(device_spec) {
            return blkdev.get_path().to_str();
        }
    } else {
        // Assume plain label (for backward compatibility)
        if let Some(blkdev) = blkid.by_label(device_spec) {
            return blkdev.get_path().to_str();
        }
    }

    None
}

/// List all available block devices with their UUIDs and labels
fn list_available_devices(blkid: &BlkInfo) {
    let devices = blkid.get_devices();

    if devices.is_empty() {
        log::error!("No block devices found on the system!");
        return;
    }

    log::error!("Available block devices:");

    for d in devices {
        let path = d.get_path().to_str().unwrap_or("<invalid>");
        let uuid = d.get_uuid();
        let label = d.get_label();
        let fstype = d.get_fstype();

        if !fstype.is_empty() {
            log::error!("Device: {}", path);

            if !uuid.is_empty() {
                log::error!("  UUID:  {}", uuid);
                log::error!("  Use:   uuid={}", uuid);
            } else {
                log::error!("  UUID:  <none>");
            }

            if !label.is_empty() {
                log::error!("  LABEL: {}", label);
                log::error!("  Use:   label={}", label);
            } else {
                log::error!("  LABEL: <none>");
            }

            log::error!("  Type:  {}", fstype);
            log::error!("  Use:   {}", path);
        }
    }
}

/// Get block devices
pub fn get_blk_devices(cfg: &MhConfig) -> Result<(String, Vec<SystemDir<String>>), Error> {
    let mut root_fstype = String::new();
    let mut blkid = BlkInfo::new();

    blkid.probe_devices()?;

    for d in blkid.get_devices() {
        if !d.get_fstype().is_empty() {
            log::info!("{} partition at {:?} ({}) \"{}\"", d.get_fstype(), d.get_path(), d.get_uuid(), d.get_label());
        }
    }

    let mut blk_mpt: Vec<SystemDir<String>> = Vec::new();

    let mask = cfg.get_mask_cmdline();
    let cmdline = crate::cmdline::CmdLine::new_with_mask(&mask).unwrap_or_default();

    if let Some(root_device) = cmdline.get_root_device() {
        log::info!("Using root device from kernel cmdline: {}", root_device);

        let root_fs = cmdline.get_root_fstype().unwrap_or("ext4");
        let _root_opts = cmdline.get_root_options().unwrap_or("rw");

        root_fstype = root_fs.to_string();

        let devpath = resolve_device_path(root_device, &blkid);

        if let Some(devpath) = devpath {
            log::info!("Resolved root device to: {}", devpath);
            let dir = SystemDir::new(root_fs.to_string(), devpath.to_string(), cfg.get_sysroot_path());
            blk_mpt.push(dir);
        } else {
            log::error!("Could not resolve root device from cmdline: {}", root_device);
            log::error!("");
            list_available_devices(&blkid);
            return Err(Error::new(std::io::ErrorKind::NotFound, "Root device not found"));
        }
    } else {
        log::info!("No root device in kernel cmdline, using configuration file");
        for dev in cfg.get_disks()? {
            let mpt = dev.get_mountpoint().trim_end_matches('/').to_string();
            if mpt.is_empty() && root_fstype.is_empty() {
                root_fstype = dev.get_fstype().into();
            }

            let device_spec = dev.get_device();
            let devpath = resolve_device_path(device_spec, &blkid);

            if let Some(devpath) = devpath {
                let dir = SystemDir::new_with_mode(
                    dev.get_fstype().into(),
                    devpath.into(),
                    format!("{}{}", &cfg.get_sysroot_path(), mpt),
                    dev.get_mode().into(),
                );
                blk_mpt.push(dir);
            } else {
                log::error!("Could not resolve device from config: {}", device_spec);
                log::error!("");
                list_available_devices(&blkid);
                return Err(Error::new(std::io::ErrorKind::NotFound, format!("Device not found: {}", device_spec)));
            }
        }
    }

    // Sort mountpoints, so the "/" goes always first
    blk_mpt.sort_by(|ela, elb| ela.dev.cmp(&elb.dev));

    Ok((root_fstype, blk_mpt))
}

/// Verify that a symlink target exists and is executable
fn verify_symlink_target(symlink_path: &str, target_path: &Path, root_mpt: &str) -> Result<(), Error> {
    if target_path.is_absolute() {
        // For absolute symlinks, check if target exists relative to the sysroot
        let target_in_sysroot = format!("{}{}", root_mpt, target_path.display());
        let target_sysroot_path = Path::new(&target_in_sysroot);

        if target_sysroot_path.exists() {
            log::info!("Init symlink target verified at {}", target_in_sysroot);
            if let Ok(target_meta) = std::fs::metadata(&target_in_sysroot) {
                let perms = target_meta.permissions();
                let mode = perms.mode();
                if mode & 0o111 == 0 {
                    log::error!("Init symlink target exists but is not executable: {}", target_in_sysroot);
                    log::error!("Permissions: {:o}", mode);
                    return Err(Error::new(std::io::ErrorKind::PermissionDenied, "Init symlink target is not executable"));
                }
            }
            log::info!("Init binary verified at {} before pivot (absolute symlink)", symlink_path);
        } else {
            log::error!("Init absolute symlink target not found: {}", target_in_sysroot);
            log::error!("Symlink: {} -> {}", symlink_path, target_path.display());
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("Init symlink target not found: {}", target_in_sysroot),
            ));
        }
    } else {
        // Relative symlink - check normally
        let symlink_path_obj = Path::new(symlink_path);
        if !symlink_path_obj.exists() {
            log::warn!("Init symlink target does not exist yet, but will check after pivot");
        } else if let Ok(target_meta) = std::fs::metadata(symlink_path) {
            let perms = target_meta.permissions();
            let mode = perms.mode();
            if mode & 0o111 == 0 {
                log::error!("Init binary exists but is not executable: {}", symlink_path);
                log::error!("Permissions: {:o}", mode);
                return Err(Error::new(std::io::ErrorKind::PermissionDenied, "Init binary is not executable"));
            }
            log::info!("Init binary verified at {} before pivot", symlink_path);
        }
    }
    Ok(())
}

/// Verify init binary exists and handle symlinks appropriately
pub fn verify_init_binary(init_path: &str, root_mpt: &str) -> Result<(), Error> {
    let init_path_obj = Path::new(init_path);

    log::debug!("Checking for init binary at: {}", init_path);

    let init_exists = init_path_obj.exists() || init_path_obj.symlink_metadata().is_ok();

    if !init_exists {
        log::error!("Init binary not found at: {}", init_path);
        log::error!("The root filesystem mounted but does not contain the init program");

        if let Ok(entries) = std::fs::read_dir(root_mpt) {
            log::error!("Contents of {}:", root_mpt);
            for entry in entries.take(10).flatten() {
                log::error!("  - {}", entry.path().display());
            }
        }

        return Err(Error::new(std::io::ErrorKind::NotFound, format!("Init binary not found after mount: {}", init_path)));
    }

    if let Ok(metadata) = init_path_obj.symlink_metadata() {
        if metadata.is_symlink() {
            log::debug!("Init is a symlink at {}", init_path);
            if let Ok(target) = std::fs::read_link(init_path_obj) {
                log::debug!("Init symlink points to: {}", target.display());
                verify_symlink_target(init_path, &target, root_mpt)?;
            }
        } else {
            let perms = metadata.permissions();
            let mode = perms.mode();
            if mode & 0o111 == 0 {
                log::error!("Init binary exists but is not executable: {}", init_path);
                log::error!("Permissions: {:o}", mode);
                return Err(Error::new(std::io::ErrorKind::PermissionDenied, "Init binary is not executable"));
            }
            log::info!("Init binary verified at {} before pivot", init_path);
        }
    }

    Ok(())
}
