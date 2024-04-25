use nix::{mount::MsFlags, sys::statvfs, unistd};
use std::io::Error;
use sys_mount::Mount;

/// Utilities for the root filesystem operations.
///
/// This module is intended to do all the basic operations those are typically
/// done by external utils, such as mount, umount, switch root etc.

/// Returns filesystem type
fn fs_type(p: &str) -> Result<u64, Error> {
    Ok(statvfs::statvfs(p)?.filesystem_id())
}

/// Recursively removes everything from the specific filesystem
fn rmrf() {}

/// Mounts mountpoint
pub fn mount(fstype: &str, dev: &str, dst: &str) -> Result<(), Error> {
    if let Err(err) = Mount::builder().fstype(fstype).mount(dev, dst) {
        return Err(Error::new(std::io::ErrorKind::NotConnected, format!("Failed to mount {}: {}", fstype, err)));
    } else {
        log::info!("Mounted {} at {} as {}", dev, dst, fstype);
    }

    Ok(())
}

/// Un-mount a mountpoint.
pub fn umount(dst: &str) -> Result<(), Error> {
    Ok(nix::mount::umount(dst)?)
}

/// Switches root
pub fn pivot(temp: &str, fstype: &str) -> Result<(), Error> {
    unistd::chdir(temp)?;
    nix::mount::mount(Some("."), "/", Some(fstype), MsFlags::MS_MOVE, Option::<&str>::None)?;
    unistd::chroot(".")?;

    Ok(())
}
