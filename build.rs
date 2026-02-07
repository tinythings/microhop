use std::process::Command;

fn main() {
    // Capture git commit hash at build time
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

    if std::path::Path::new(".git").exists() {
        println!("cargo:rerun-if-changed=.git/HEAD");
        println!("cargo:rerun-if-changed=.git/refs");
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

    println!("cargo:rerun-if-changed=build.rs");
}
