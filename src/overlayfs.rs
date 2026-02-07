use profile::cfg::{MhConfig, OverlayfsConfig};
use std::{
    fs::DirBuilder,
    io::Error,
    os::unix::fs::DirBuilderExt,
    process::{Command, Stdio},
};
use syslib::blk::BlkInfo;

/// Try to prepare and mount overlayfs. Returns Ok(Some(merged_dir)) when overlay is ready
/// and should be used as final root, Ok(None) when overlay should be skipped, or Err on fatal error.
pub fn try_setup_overlay(cfg: &MhConfig, temp_mpt: &str, overlay_cfg: &OverlayfsConfig) -> Result<Option<String>, Error> {
    use crate::microhop::resolve_device_path;

    let mut blkid = BlkInfo::new();
    blkid.probe_devices()?;

    let overlay_dev_path = resolve_device_path(&overlay_cfg.device, &blkid).ok_or_else(|| {
        Error::new(std::io::ErrorKind::NotFound, format!("Could not resolve overlayfs device: {}", overlay_cfg.device))
    })?;

    log::info!("Using overlayfs device: {} ({})", overlay_dev_path, overlay_cfg.device);

    let lower_dir = temp_mpt;
    let overlay_mount = "/overlay";
    let merged_dir = "/overlay/merged";

    DirBuilder::new().recursive(true).mode(0o755).create(overlay_mount)?;

    let mut overlay_ok = false;
    match syslib::fs::mount("ext4", overlay_dev_path, overlay_mount) {
        Ok(()) => {
            log::info!("Mounted overlayfs backing device at {}", overlay_mount);
            overlay_ok = true;
        }
        Err(err) => {
            log::error!("Failed to mount overlay backing device {}: {}", overlay_dev_path, err);

            // Helper to run embedded /bin/e2fsck if present
            fn run_external_e2fsck(device: &str, auto: bool) -> Result<i32, String> {
                let mut cmd = Command::new("/bin/e2fsck");
                if auto {
                    cmd.arg("-p");
                } else {
                    cmd.arg("-n");
                }
                cmd.arg(device);
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());

                match cmd.output() {
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr);

                        if stderr.contains("Bad magic number") {
                            log::info!("e2fsck reported bad magic; trying alternate superblocks for {}", device);
                            match crate::fsck::restore_superblock_from_backup(device) {
                                Ok(true) => {
                                    log::info!("restore_superblock_from_backup succeeded for {}", device);
                                    // Re-run e2fsck after successful restore and return its code.
                                    let mut rerun = Command::new("/bin/e2fsck");
                                    if auto {
                                        rerun.arg("-p");
                                    } else {
                                        rerun.arg("-n");
                                    }
                                    rerun.arg(device);
                                    rerun.stdout(Stdio::null());
                                    rerun.stderr(Stdio::null());
                                    match rerun.output() {
                                        Ok(rout) => {
                                            if let Some(rc) = rout.status.code() {
                                                return Ok(rc);
                                            }
                                            return Err("re-run e2fsck terminated by signal".to_string());
                                        }
                                        Err(e) => log::error!("failed to execute re-run e2fsck: {}", e),
                                    }
                                }
                                Ok(false) => log::info!("restore_superblock_from_backup found no valid backups for {}", device),
                                Err(e) => log::error!("restore_superblock_from_backup error for {}: {}", device, e),
                            }
                        }

                        if let Some(code) = out.status.code() {
                            Ok(code)
                        } else {
                            Err("e2fsck terminated by signal".to_string())
                        }
                    }
                    Err(e) => Err(format!("failed to execute /bin/e2fsck: {}", e)),
                }
            }

            match run_external_e2fsck(overlay_dev_path, true) {
                Ok(code) => {
                    log::info!("external e2fsck returned {} for {}", code, overlay_dev_path);
                    if code == 0 || code == 1 {
                        log::info!("e2fsck reported successful/fixed filesystem for {}", overlay_dev_path);
                        overlay_ok = true;
                        log::info!("Retrying mount for overlay device {} after external fsck", overlay_dev_path);
                        if let Err(err2) = syslib::fs::mount("ext4", overlay_dev_path, overlay_mount) {
                            log::error!("Retry mount failed for overlay {}: {}", overlay_dev_path, err2);
                            log::info!("Proceeding with overlay flow despite mount retry failure (upper/work will be created on ramfs if needed)");
                        }
                    } else if code < 16 {
                        log::info!(
                            "external e2fsck returned {} for {}, attempting to restore from backups",
                            code,
                            overlay_dev_path
                        );
                        match crate::fsck::restore_superblock_from_backup(overlay_dev_path) {
                            Ok(true) => {
                                log::info!("Restored primary superblock from backup for {}: retrying mount", overlay_dev_path);
                                if let Err(err2) = syslib::fs::mount("ext4", overlay_dev_path, overlay_mount) {
                                    log::error!("Retry mount failed for overlay {} after restore: {}", overlay_dev_path, err2);
                                } else {
                                    overlay_ok = true;
                                }
                            }
                            Ok(false) => log::info!("No backup superblock found for {}", overlay_dev_path),
                            Err(e) => log::error!("Failed to attempt superblock restore for {}: {}", overlay_dev_path, e),
                        }
                    } else {
                        log::error!("external e2fsck returned unexpected code {} for {}", code, overlay_dev_path);
                        return Err(Error::other(format!("e2fsck returned code {}", code)));
                    }
                }
                Err(e) => log::error!("Failed to run external e2fsck for {}: {}", overlay_dev_path, e),
            }
        }
    }

    // Diagnostic: record /proc/mounts and device existence after mount attempt
    if let Ok(mounts_after) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts_after.lines() {
            if line.contains(overlay_mount) || line.contains(overlay_dev_path) {
                log::info!("DEBUG: post-mount /proc/mounts: {}", line);
            }
        }
    } else {
        log::info!("DEBUG: could not read /proc/mounts after overlay mount attempt");
    }

    match std::fs::metadata(overlay_dev_path) {
        Ok(md) => log::info!("DEBUG: overlay device {} exists, is_file: {}", overlay_dev_path, md.is_file()),
        Err(e) => log::info!("DEBUG: overlay device {} metadata error: {}", overlay_dev_path, e),
    }

    if !overlay_ok {
        if let Ok(mounts_check) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts_check.lines() {
                if line.contains(overlay_mount) || line.contains(overlay_dev_path) {
                    log::info!("DEBUG: detected overlay mount in /proc/mounts, setting overlay_ok=true");
                    overlay_ok = true;
                    break;
                }
            }
        }
    }

    log::info!("DEBUG: overlay_ok at merge-decision: {}", overlay_ok);
    let upper_full = format!("{}/{}", overlay_mount, overlay_cfg.upper);
    let work_full = format!("{}/{}", overlay_mount, overlay_cfg.workdir);

    if overlay_ok {
        log::info!("DEBUG: preparing merged_dir {} (upper: {}, work: {})", merged_dir, upper_full, work_full);
        DirBuilder::new().recursive(true).mode(0o755).create(merged_dir)?;

        match syslib::fs::mount_overlayfs(lower_dir, &upper_full, &work_full, merged_dir) {
            Ok(()) => {
                log::info!("DEBUG: overlayfs mounted at {}", merged_dir);
                return Ok(Some(merged_dir.to_string()));
            }
            Err(e) => {
                log::error!("DEBUG: mount_overlayfs failed: {}", e);
                return Ok(None);
            }
        }
    }

    Ok(None)
}
