mod cmdline;
mod fsck;
mod kmodprobe;
mod logger;
mod microhop;
mod overlayfs;

use crate::microhop::{get_blk_devices, greet, mount_fs, verify_init_binary, SYS_MPT};
use nix::{mount::MsFlags, unistd};
use std::{ffi::CString, fs::DirBuilder, io::Error, os::unix::fs::DirBuilderExt, path::Path};

static LOGGER: logger::STDOUTLogger = logger::STDOUTLogger;

// Git commit hash embedded at build time
const GIT_COMMIT_HASH: &str = match option_env!("GIT_COMMIT_HASH") {
    Some(hash) => hash,
    None => "unknown",
};

// Enabled features embedded at build time
const ENABLED_FEATURES: &str = match option_env!("ENABLED_FEATURES") {
    Some(features) => features,
    None => "fsck",
};

fn main() -> Result<(), Error> {
    // Set logger
    let cfg = match profile::cfg::get_mh_config(None) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("ERROR: Failed to load configuration file /etc/microhop.conf");
            match err.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!("  Reason: Configuration file not found");
                }
                std::io::ErrorKind::InvalidData => {
                    eprintln!("  Reason: Configuration file is invalid or contains syntax errors");
                    eprintln!("  Details: {}", err);
                }
                _ => {
                    eprintln!("  Details: {}", err);
                }
            }
            return Err(err);
        }
    };

    // Set up logger, defaulting to Info level if there's an issue
    if let Err(err) = log::set_logger(&LOGGER) {
        eprintln!("WARNING: Failed to set up logger: {}", err);
    }

    // Set log level from config, with validation
    let log_level = cfg.get_log_level();
    if let Some(level_str) = cfg.get_log_level_as_str() {
        match level_str.as_str() {
            "debug" | "info" | "quiet" => {
                log::set_max_level(log_level);
            }
            _ => {
                eprintln!("WARNING: Invalid log level '{}' in configuration. Using 'info' as default.", level_str);
                eprintln!("  Valid options are: debug, info, quiet");
                log::set_max_level(log::LevelFilter::Info);
            }
        }
    } else {
        log::set_max_level(log_level);
    }

    greet(&cfg)?;

    log::info!("Microhop build info:");
    log::info!("  Git commit: {}", GIT_COMMIT_HASH);
    if ENABLED_FEATURES.is_empty() {
        log::info!("  Features: none (default build)");
    } else {
        log::info!("  Features: {}", ENABLED_FEATURES);
    }

    if let Some(metadata) = cfg.get_metadata() {
        log::debug!("Configuration metadata:");
        log::debug!("  Git commit: {}", metadata.git_commit);
        log::debug!("  Generated at: {}", metadata.generated_at);
    }

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
        DirBuilder::new().recursive(true).mode(0o755).create(temp_mpt.as_str())?;
        log::debug!("Init sysroot path: {}", temp_mpt);
    }

    mount_fs(SYS_MPT)?;

    let (root_fstype, blk_mpt) = get_blk_devices(&cfg)?;
    if root_fstype.is_empty() {
        log::error!("Type of the root filesystem was not detected. Please double-check the configuration!");
        return Err(Error::new(std::io::ErrorKind::InvalidData, "Root filesystem type not detected"));
    }

    // Mount block devices - this will attempt fsck on any mount failures
    let mount_success = mount_fs(&blk_mpt)?;

    if !mount_success {
        log::error!("Failed to mount one or more filesystems!");
        log::error!("This usually means:");
        log::error!("  1. The filesystem is severely corrupted (fsck failed)");
        log::error!("  2. The device specification is incorrect");
        log::error!("  3. The filesystem type is not supported by the kernel");
        return Err(Error::other("Root filesystem mount failed"));
    }

    // Verify that the root filesystem was successfully mounted and contains init
    let root_mountpoint: Option<&str> = blk_mpt
        .iter()
        .find(|m| m.dst.trim_end_matches('/') == cfg.get_sysroot_path().trim_end_matches('/'))
        .map(|m| m.dst.as_str());

    if let Some(root_mpt) = root_mountpoint {
        let init_in_rootfs = format!("{}{}", root_mpt, cfg.get_init_path());
        // Use the extracted function to verify init binary before pivot
        verify_init_binary(&init_in_rootfs, root_mpt)?;
    } else {
        log::warn!("Could not determine root mountpoint for init verification");
    }

    let mut use_overlayfs = cfg.use_overlayfs();
    let mut final_root = cfg.get_sysroot_path();

    if let Some(overlay_cfg) = cfg.get_overlayfs() {
        log::info!("Overlayfs enabled in configuration");

        if !syslib::fs::is_overlayfs_supported() {
            log::error!("Overlayfs requested but not supported by kernel!");
            log::error!("Make sure CONFIG_OVERLAY_FS is enabled in kernel config");
            return Err(Error::new(std::io::ErrorKind::Unsupported, "Overlayfs not supported"));
        }

        match overlayfs::try_setup_overlay(&cfg, temp_mpt, overlay_cfg) {
            Ok(Some(mounted_root)) => {
                final_root = mounted_root;
            }
            Ok(None) => {
                use_overlayfs = false;
                final_root = cfg.get_sysroot_path();
            }
            Err(e) => return Err(e),
        }
    }

    // Remount sysfs, switch root
    log::debug!("switching root");
    // Debug: log what we intend to pivot to and current mounts for diagnosis
    log::info!("DEBUG: final_root='{}', use_overlayfs={}", final_root, use_overlayfs);
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            if line.contains(&final_root) || line.contains("/overlay") || line.starts_with("/") {
                log::info!("DEBUG: mount: {}", line);
            }
        }
    } else {
        log::info!("DEBUG: could not read /proc/mounts");
    }
    for t in SYS_MPT {
        let tgt = format!("{}{}", final_root, t.dst);
        log::info!("Moving {} -> {}", t.dst, tgt);

        if let Some(parent) = Path::new(&tgt).parent() {
            if !parent.exists() {
                log::debug!("Creating parent directory: {}", parent.display());
                if let Err(e) = std::fs::create_dir_all(parent) {
                    log::error!("Failed to create parent directory {}: {}", parent.display(), e);
                    return Err(e);
                }
            }
        }

        if !Path::new(&tgt).exists() {
            log::debug!("Creating target directory: {}", tgt);
            if let Err(e) = std::fs::create_dir_all(&tgt) {
                log::error!("Failed to create target directory {}: {}", tgt, e);
                return Err(e);
            }
        }

        match nix::mount::mount(Some(t.dst), tgt.as_str(), Some(t.fstype), MsFlags::MS_MOVE, Option::<&str>::None) {
            Ok(()) => log::debug!("Successfully moved {} to {}", t.dst, tgt),
            Err(e) => {
                log::error!("Failed to move {} to {}: {}", t.dst, tgt, e);
                log::error!("Source exists: {}", Path::new(t.dst).exists());
                log::error!("Target exists: {}", Path::new(&tgt).exists());
                return Err(Error::other(format!("Mount move failed: {}", e)));
            }
        }
    }

    // Pivot the system
    syslib::fs::pivot(&final_root, if use_overlayfs { "overlay" } else { root_fstype.as_str() })?;

    // Post-pivot verification: ensure the root filesystem is the expected one.
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        // find root entry
        for line in mounts.lines() {
            if line.split_whitespace().nth(1).map(|s| s == "/").unwrap_or(false) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let fstype_now = parts[2];
                    let expected = if use_overlayfs { "overlay" } else { root_fstype.as_str() };
                    if fstype_now != expected {
                        log::warn!(
                            "Post-pivot root fstype '{}' != expected '{}', attempting a second MS_MOVE of {} -> /",
                            fstype_now,
                            expected,
                            final_root
                        );
                        // Try to move the final_root mount onto /
                        match nix::mount::mount(
                            Some(final_root.as_str()),
                            "/",
                            Some(expected),
                            nix::mount::MsFlags::MS_MOVE,
                            Option::<&str>::None,
                        ) {
                            Ok(()) => log::info!("Second MS_MOVE succeeded: {} is now root", final_root),
                            Err(e) => log::error!("Second MS_MOVE failed: {}", e),
                        }
                    }
                }
                break;
            }
        }
    } else {
        log::warn!("Could not read /proc/mounts for post-pivot verification");
    }

    // Start external init
    log::info!("Launching init at {}", cfg.get_init_path());

    let init_path = cfg.get_init_path();

    // Verify init binary exists and is executable
    let init_path_obj = Path::new(&init_path);
    if !init_path_obj.exists() {
        log::error!("Init binary not found at: {}", init_path);
        log::error!("The root filesystem may be corrupted or not properly mounted");
        log::error!("Try running fsck on the root partition manually");
        return Err(Error::new(std::io::ErrorKind::NotFound, format!("Init binary not found: {}", init_path)));
    }

    if let Ok(metadata) = std::fs::metadata(&init_path) {
        if !metadata.is_file() {
            log::error!("Init path exists but is not a file: {}", init_path);
            return Err(Error::new(std::io::ErrorKind::InvalidInput, "Init is not a file"));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = metadata.permissions();
            let mode = perms.mode();
            if mode & 0o111 == 0 {
                log::error!("Init binary is not executable: {}", init_path);
                log::error!("Permissions: {:o}", mode);
                return Err(Error::new(std::io::ErrorKind::PermissionDenied, "Init is not executable"));
            }
        }
    }

    log::info!("Init binary verified, executing...");

    let init_cstring = match CString::new(init_path.clone()) {
        Ok(s) => s,
        Err(err) => {
            log::error!("Failed to create init path argument: contains null bytes");
            log::error!("Init path: {}", init_path);
            log::error!("Details: {}", err);
            return Err(Error::new(std::io::ErrorKind::InvalidInput, format!("Init path '{}' contains null bytes", init_path)));
        }
    };

    let argv: Vec<CString> = vec![init_cstring.clone()];

    #[allow(irrefutable_let_patterns)]
    if let Err(err) = unistd::execv(&init_cstring, &argv) {
        log::error!("Failed to execute init process: {:?}", err);
        log::error!("Init path was: {}", init_path);
        log::error!("This is a critical error - the system cannot boot");
    }

    Ok(())
}
