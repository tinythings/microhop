//! Utilities for the root filesystem operations.
//!
//! This module is intended to do all the basic operations those are typically
//! done by external utils, such as mount, umount, switch root etc.

use nix::{mount::MsFlags, sys::statvfs, unistd};
use std::{fs, io::Error, path::Path};
use walkdir::WalkDir;

/// Returns filesystem type
fn fs_type(p: &str) -> Result<u64, Error> {
    Ok(statvfs::statvfs(p)?.filesystem_id())
}

/// Recursively removes everything from the ramfs
fn rmrf(sr: &str) -> Result<(), Error> {
    fn is_sys(e: &str, sr: &str) -> bool {
        for d in ["/proc", "/sys", "/dev", sr] {
            if e == "/"
                || e.starts_with(d)
                || e.starts_with(format!("{}{}", sr, d).as_str())
                || e.starts_with(format!("{}{}/", sr, d).as_str())
            {
                return true;
            }
        }

        false
    }

    WalkDir::new("/").into_iter().flat_map(|r| r.ok()).for_each(|e| {
        let p = e.path().as_os_str().to_str().unwrap_or_default();
        if let Ok(fst) = fs_type(p) {
            if fst == 0 && !is_sys(p, sr) && e.path().is_dir() && p != "/" {
                fs::remove_dir_all(e.path()).unwrap_or_default();
            }
        }
    });
    Ok(())
}

/// Mounts mountpoint
pub fn mount(fstype: &str, dev: &str, dst: &str) -> Result<(), Error> {
    mount_with_flags(fstype, dev, dst, MsFlags::MS_NOATIME)
}

/// Mounts mountpoint with specific flags
pub fn mount_with_flags(fstype: &str, dev: &str, dst: &str, flags: MsFlags) -> Result<(), Error> {
    if let Err(err) = nix::mount::mount(Some(dev), dst, Some(fstype), flags, Option::<&str>::None) {
        return Err(Error::new(
            std::io::ErrorKind::NotConnected,
            format!("Failed to mount {} on {} as {}: {}", fstype, dev, dst, err),
        ));
    } else {
        log::debug!("Mounted {} at {} as {} with flags {:?}", dev, dst, fstype, flags);
    }

    Ok(())
}

/// Un-mount a mountpoint.
#[allow(dead_code)]
pub fn umount(dst: &str) -> Result<(), Error> {
    Ok(nix::mount::umount(dst)?)
}

/// Check if overlayfs is supported by the kernel
pub fn is_overlayfs_supported() -> bool {
    if let Ok(filesystems) = fs::read_to_string("/proc/filesystems") {
        if filesystems.contains("overlay") {
            log::debug!("Overlayfs is supported by kernel");
            return true;
        }
    }
    log::warn!("Overlayfs is not supported by kernel");
    false
}

/// Mount overlayfs with lower and upper directories
pub fn mount_overlayfs(lowerdir: &str, upperdir: &str, workdir: &str, target: &str) -> Result<(), Error> {
    if !Path::new(upperdir).exists() {
        fs::create_dir_all(upperdir)?;
        log::debug!("Created upperdir: {}", upperdir);
    }
    if !Path::new(workdir).exists() {
        fs::create_dir_all(workdir)?;
        log::debug!("Created workdir: {}", workdir);
    }

    let options = format!("lowerdir={},upperdir={},workdir={}", lowerdir, upperdir, workdir);

    log::info!("Mounting overlayfs: {}", options);

    if let Err(err) = nix::mount::mount(Some("overlay"), target, Some("overlay"), MsFlags::empty(), Some(options.as_str())) {
        return Err(Error::new(std::io::ErrorKind::NotConnected, format!("Failed to mount overlayfs at {}: {}", target, err)));
    }

    log::info!("Overlayfs mounted successfully at {}", target);
    Ok(())
}

/// Switches root
pub fn pivot(temp: &str, fstype: &str) -> Result<(), Error> {
    rmrf(temp)?;
    log::debug!("Cleanup ramfs");

    unistd::chdir(temp)?;
    nix::mount::mount(Some(temp), "/", Some(fstype), MsFlags::MS_MOVE, Option::<&str>::None)?;
    unistd::chroot(".")?;
    log::debug!("Enter the rootfs");

    Ok(())
}
