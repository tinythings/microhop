use std::path::Path;
use std::process::{Command, Stdio};

/// Run an external `e2fsck` for automatic fixes.
pub fn fsck_ext2(device: &str) -> Result<i32, String> {
    let e2fsck_path = "/bin/e2fsck";
    if !Path::new(e2fsck_path).exists() {
        return Err(format!("{} not found in initramfs", e2fsck_path));
    }

    // Attempt to autofix the device
    let mut cmd = Command::new(e2fsck_path);
    cmd.arg("-p");
    cmd.arg(device);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    match cmd.status() {
        Ok(st) => match st.code() {
            Some(code) => Ok(code),
            None => Err("e2fsck terminated by signal".to_string()),
        },
        Err(e) => Err(format!("failed to execute e2fsck: {}", e)),
    }
}

/// Attempt to restore the primary superblock from common backup locations.
pub fn restore_superblock_from_backup(device: &str) -> Result<bool, String> {
    let e2fsck_path = "/bin/e2fsck";
    if !Path::new(e2fsck_path).exists() {
        return Err(format!("{} not found in initramfs", e2fsck_path));
    }
    // Use candidate backup superblocks by invoking `e2fsck -b <blk>` for each.
    let backups: [u64; 20] = [
        8193, 16384, 24576, 32768, 49152, 65536, 98304, 131072, 163840, 196608, 229376, 262144, 294912, 327680, 360448, 393216,
        425984, 458752, 491520, 524288,
    ];

    for &bnum in &backups {
        let bstr = format!("{}", bnum);
        let out = Command::new(e2fsck_path)
            .arg("-b")
            .arg(&bstr)
            .arg("-f")
            .arg("-y")
            .arg(device)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match out {
            Ok(o) => {
                log::info!("running: e2fsck -b {} -f -y {}", bstr, device);
                if let Some(code) = o.status.code() {
                    if code == 0 || code == 1 {
                        log::info!("e2fsck -b {} succeeded with code {}", bstr, code);
                        return Ok(true);
                    } else {
                        log::info!("e2fsck -b {} exited with code {}", bstr, code);
                    }
                }
            }
            Err(e) => {
                log::error!("failed to execute e2fsck -b {}: {}", bnum, e);
            }
        }
    }

    Ok(false)
}
