//! Utilities for the root filesystem operations.
//!
//! This module is intended to do all the basic operations those are typically
//! done by external utils, such as mount, umount, switch root etc.

use nix::{mount::MsFlags, sys::statvfs, unistd};
use std::{fs, io::Error};
use walkdir::WalkDir;

/// Returns filesystem type
fn fs_type(p: &str) -> Result<u64, Error> {
    Ok(statvfs::statvfs(p)?.filesystem_id())
}

/// Recursively removes everything from the ramfs
fn rmrf(sr: &str) -> Result<(), Error> {
    fn is_sys(e: &str, sr: &str) -> bool {
        for d in ["/proc", "/sys", "/dev", sr] {
            if e != "/"
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
    if let Err(err) = nix::mount::mount(Some(dev), dst, Some(fstype), MsFlags::MS_NOATIME, Option::<&str>::None) {
        return Err(Error::new(
            std::io::ErrorKind::NotConnected,
            format!("Failed to mount {} on {} as {}: {}", fstype, dev, dst, err),
        ));
    } else {
        log::debug!("Mounted {} at {} as {}", dev, dst, fstype);
    }

    Ok(())
}

/// Un-mount a mountpoint.
#[allow(dead_code)]
pub fn umount(dst: &str) -> Result<(), Error> {
    Ok(nix::mount::umount(dst)?)
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
