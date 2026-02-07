use std::fs::File;
use std::io::Read;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(microhop_binary_path)");
    println!("cargo:rustc-check-cfg=cfg(e2fsck_binary_path)");

    let git_commit = std::env::var("GIT_COMMIT")
        .ok()
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .and_then(|output| if output.status.success() { String::from_utf8(output.stdout).ok() } else { None })
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit);

    // Only set git rerun conditions if we're in a git repo
    if std::path::Path::new("../.git").exists() {
        println!("cargo:rerun-if-changed=../.git/HEAD");
        println!("cargo:rerun-if-changed=../.git/refs");
    }

    // Capture enabled features at build time
    let mut features = Vec::new();

    if std::env::var_os("CARGO_FEATURE_FSCK").is_some() {
        features.push("fsck");
    }
    if std::env::var_os("CARGO_FEATURE_NIXOS").is_some() {
        features.push("nixos");
    }

    let features_string = features.join(",");
    println!("cargo:rustc-env=ENABLED_FEATURES={}", features_string);

    if let Ok(binary_path) = std::env::var("MICROHOP_BINARY_PATH") {
        let path = std::path::Path::new(&binary_path);
        if !path.exists() {
            panic!("MICROHOP_BINARY_PATH points to non-existent file: {}", binary_path);
        }

        if !path.is_file() {
            panic!("MICROHOP_BINARY_PATH is not a regular file: {}", binary_path);
        }

        match File::open(path) {
            Ok(mut file) => {
                let mut magic = [0u8; 4];
                match file.read_exact(&mut magic) {
                    Ok(_) => {
                        if magic != [0x7F, 0x45, 0x4C, 0x46] {
                            panic!(
                                "MICROHOP_BINARY_PATH is not an ELF executable: {}\n\
                                 Expected ELF magic header [0x7F, 'E', 'L', 'F'], but found [{:#04x}, {:#04x}, {:#04x}, {:#04x}]",
                                binary_path, magic[0], magic[1], magic[2], magic[3]
                            );
                        }
                    }
                    Err(e) => {
                        panic!("Failed to read magic header from MICROHOP_BINARY_PATH ({}): {}", binary_path, e);
                    }
                }
            }
            Err(e) => {
                panic!("Failed to open MICROHOP_BINARY_PATH ({}): {}", binary_path, e);
            }
        }
    }

    if std::env::var_os("CARGO_FEATURE_FSCK").is_some() {
        if let Ok(binary_path) = std::env::var("E2FSCK_BINARY_PATH") {
            let path = std::path::Path::new(&binary_path);
            if !path.exists() {
                panic!("E2FSCK_BINARY_PATH points to non-existent file: {}", binary_path);
            }

            if !path.is_file() {
                panic!("E2FSCK_BINARY_PATH is not a regular file: {}", binary_path);
            }

            match File::open(path) {
                Ok(mut file) => {
                    let mut magic = [0u8; 4];
                    match file.read_exact(&mut magic) {
                        Ok(_) => {
                            if magic != [0x7F, 0x45, 0x4C, 0x46] {
                                panic!(
                                    "E2FSCK_BINARY_PATH is not an ELF executable: {}\n\
                                     Expected ELF magic header [0x7F, 'E', 'L', 'F'], but found [{:#04x}, {:#04x}, {:#04x}, {:#04x}]",
                                    binary_path, magic[0], magic[1], magic[2], magic[3]
                                );
                            }
                        }
                        Err(e) => {
                            panic!("Failed to read magic header from E2FSCK_BINARY_PATH ({}): {}", binary_path, e);
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to open E2FSCK_BINARY_PATH ({}): {}", binary_path, e);
                }
            }

            println!("cargo:rustc-cfg=e2fsck_binary_path");
        } else {
            println!("cargo:warning=Feature 'fsck' enabled but E2FSCK_BINARY_PATH not set; build will try local 'e2fsck'");
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}
