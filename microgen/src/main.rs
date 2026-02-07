mod analyser;
mod clidef;
mod kconfig_validator;
mod rdgen;
mod rdpack;

use clap::ArgMatches;
use colored::Colorize;
use kmoddep::{kerman::KernelInfo, modinfo::lsmod};
use rdgen::IrfsGen;
use std::{error::Error, io, path::PathBuf};

// Version from Cargo.toml
static VERSION: &str = env!("CARGO_PKG_VERSION");
static APPNAME: &str = "microgen";

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

fn run_info(params: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let rfs = params.get_one::<String>("list").map(|v| v.as_str());

    if params.get_flag("list-filesystems") {
        kconfig_validator::print_supported_filesystems();
        return Ok(());
    }

    if params.get_flag("list-block-devices") {
        kconfig_validator::print_supported_block_devices();
        return Ok(());
    }

    let k_info = kmoddep::get_kernel_infos(rfs)?;

    if rfs.is_some() {
        println!("{}", "Available kernels:".bright_yellow());
        for i in k_info {
            println!(
                "  {}",
                i.get_kernel_path().file_name().unwrap_or_default().to_str().unwrap_or_default().bright_yellow().bold()
            );
        }
    } else if params.get_flag("lsmod") {
        println!("\n{:<30} {:<10} {}", "Name".bright_yellow(), "Size".bright_yellow(), "Used by".bright_yellow());
        for m in lsmod() {
            println!(
                "{:<30} {:<10} {} {}",
                m.name.bright_green().bold(),
                m.mem_size.to_string().green(),
                m.instances.to_string().bright_white().bold(),
                m.dependencies.join(", ").white()
            );
        }
    }

    Ok(())
}

/// Run validation of microhop.conf
fn run_validate(params: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config_path = params.get_one::<String>("config").unwrap();

    let cfg = match profile::cfg::get_mh_config(Some(config_path)) {
        Ok(cfg) => {
            println!("Configuration file parsed successfully");
            cfg
        }
        Err(e) => {
            println!("Failed to parse configuration file:");
            println!("  {}", e.to_string().red());
            return Err(e.into());
        }
    };

    let mut has_errors = false;
    let mut has_warnings = false;

    println!("\n{}", "=== Modules ===".bright_yellow().bold());
    let modules = cfg.get_modules();
    if modules.is_empty() {
        println!("  No modules configured (assuming monolithic kernel)");
    } else {
        println!("  {} module(s) configured:", modules.len());
        for module in modules {
            println!("    - {}", module.bright_white());
        }
    }

    println!("\n{}", "=== Disks ===".bright_yellow().bold());
    match cfg.get_disks() {
        Ok(disks) => {
            if disks.is_empty() {
                println!("  No disks configured");
                println!("  Note: Disks can be specified via kernel command line (root=)");
                has_warnings = true;
            } else {
                println!(" {} disk(s) configured:", disks.len());
                for disk in &disks {
                    let device = disk.get_device();
                    let fstype = disk.get_fstype();
                    let mountpoint = disk.get_mountpoint();
                    let mode = disk.get_mode();

                    let device_status = if device.starts_with("uuid=") || device.starts_with("label=") {
                        format!("Preferred format ({})", device)
                    } else if device.starts_with("/dev/") {
                        has_warnings = true;
                        format!("Legacy format: {}. Consider using uuid= or label= instead", device)
                    } else if device.len() == 36 && device.chars().filter(|c| *c == '-').count() == 4 {
                        has_warnings = true;
                        format!("Plain UUID detected: {}. Consider prefixing with 'uuid=' for clarity", device)
                    } else {
                        has_warnings = true;
                        format!("Unknown device format: {}", device)
                    };

                    println!("    {}", device_status);
                    println!("      Filesystem: {}", fstype.bright_white());
                    println!("      Mountpoint: {}", mountpoint.bright_white());
                    println!("      Mode: {}", mode.bright_white());
                }
            }
        }
        Err(e) => {
            println!("  Invalid disk configuration:");
            println!("    {}", e.to_string().red());
            has_errors = true;
        }
    }

    println!("\n{}", "=== Init ===".bright_yellow().bold());
    let init_path = cfg.get_init_path();
    println!("  Init path: {}", init_path.bright_white());

    println!("\n{}", "=== Sysroot ===".bright_yellow().bold());
    let sysroot_path = cfg.get_sysroot_path();
    println!("  Sysroot path: {}", sysroot_path.bright_white());

    println!("\n{}", "=== Logging ===".bright_yellow().bold());
    if let Some(log_level) = cfg.get_log_level_as_str() {
        let valid_levels = ["debug", "info", "quiet"];
        if valid_levels.contains(&log_level.as_str()) {
            println!("  Log level: {}", log_level.bright_white());
        } else {
            println!("  Invalid log level: '{}'. Valid options: debug, info, quiet", log_level);
            has_warnings = true;
        }
    } else {
        println!("  Log level: {} (default)", "info".bright_white());
    }

    println!("\n{}", "=== Overlayfs ===".bright_yellow().bold());
    if let Some(overlay_cfg) = cfg.get_overlayfs() {
        println!("  Overlayfs is configured:");
        println!("    Device: {}", overlay_cfg.device.bright_white());
        println!("    Upper: {}", overlay_cfg.upper.bright_white());
        println!("    Workdir: {}", overlay_cfg.workdir.bright_white());
        println!("  Note: Requires CONFIG_OVERLAY_FS in kernel");
    } else {
        println!("  Overlayfs not configured");
    }

    println!("\n{}", "=== Firmware ===".bright_yellow().bold());
    let firmware_base = cfg.get_firmware_base();
    println!("  Firmware base path: {}", firmware_base.bright_white());

    println!("\n{}", "=== Validation Summary ===".bright_cyan().bold());
    if has_errors {
        println!("  Validation FAILED - configuration has errors");
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidData, "Configuration validation failed")));
    } else if has_warnings {
        println!("  Validation passed with warnings");
        println!("  Review warnings above for potential improvements");
    } else {
        println!("  Validation PASSED - configuration is valid");
    }
    println!();

    Ok(())
}

/// Run analysis and profile generator
fn run_analyse(_params: &ArgMatches) -> Result<(), Box<dyn Error>> {
    if !nix::unistd::Uid::effective().is_root() {
        return Err(Box::new(io::Error::new(io::ErrorKind::Unsupported, "error: superuser privileges required")));
    }

    let _cfg = analyser::SysAnalyser::new().get_config(kmoddep::get_kernel_infos(None)?)?;

    Ok(())
}

/// Create a new initramfs
fn run_new(params: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let x_mods: Vec<String> = params.get_many::<String>("extract").unwrap_or_default().map(|s| s.to_string()).collect();
    let profile = params.get_one::<String>("config");
    let kernel_config = params.get_one::<String>("kernel-config");
    let validate_only = params.get_flag("validate-only");

    // Only get kernel info if we actually need it (when extracting modules or if modules are configured)
    let need_kernel_info = !x_mods.is_empty()
        || (profile.is_some() && {
            // Check if profile has modules configured
            match profile::cfg::get_mh_config(profile.map(|x| x.as_str())) {
                Ok(cfg) => !cfg.get_modules().is_empty(),
                Err(_) => true, // If we can't read config, assume we might need kernel info
            }
        });

    let k_info = if need_kernel_info {
        match kmoddep::get_kernel_infos(Some(params.get_one::<String>("root").unwrap())) {
            Ok(info) => Some(info),
            Err(e) => {
                println!("Unable to get the information about the kernel: {}", e);
                if !x_mods.is_empty() {
                    return Ok(());
                }
                None
            }
        }
    } else {
        None
    };

    if !x_mods.is_empty() {
        let krel = params.get_one::<String>("kernel").unwrap().replace('"', "");
        let kernels = k_info.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No kernel information available"))?;
        if kernels.is_empty() {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::NotFound,
                "No kernel modules found in the specified root filesystem",
            )));
        }
        for knfo in kernels {
            let kn = knfo.get_kernel_path().file_name().unwrap().to_str().unwrap().to_string();
            if krel == kn {
                println!("{:?}", knfo.get_deps_for(&x_mods.iter().map(|x| x.to_string()).collect::<Vec<String>>()));
            }
        }
    } else if let Some(profile) = profile {
        let cfg = profile::cfg::get_mh_config(Some(profile))?;

        if let Some(kconfig_path) = kernel_config {
            println!("{}", "Validating kernel configuration...".bright_cyan().bold());

            let filesystems: Option<Vec<String>> =
                params.get_many::<String>("filesystems").map(|values| values.map(|s| s.to_string()).collect());

            // Get optional block devices list from CLI
            let block_devices: Option<Vec<String>> =
                params.get_many::<String>("block-devices").map(|values| values.map(|s| s.to_string()).collect());

            match kconfig_validator::KConfigValidator::from_file(kconfig_path) {
                Ok(validator) => {
                    match validator.validate(&cfg, filesystems.as_deref(), block_devices.as_deref()) {
                        Ok(result) => {
                            result.print();

                            if validate_only {
                                // Exit after validation
                                if result.has_errors() {
                                    return Err(Box::new(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "Kernel configuration validation failed",
                                    )));
                                }
                                return Ok(());
                            }

                            if result.has_errors() {
                                return Err(Box::new(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "Cannot generate initramfs: kernel configuration is incompatible",
                                )));
                            }

                            if result.has_warnings() {
                                println!("{}", "⚠ Continuing with warnings...".bright_yellow());
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Validation error: {}", e).bright_red());
                            return Err(Box::new(e));
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Failed to parse kernel config: {}", e).bright_red());
                    return Err(Box::new(e));
                }
            }
        } else if validate_only {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--validate-only requires --kernel-config to be specified",
            )));
        }

        // Generate initramfs if not validation-only
        if !validate_only {
            let kfo: Option<KernelInfo> = if let Some(k_info) = k_info {
                match k_info.len() {
                    0 => {
                        println!("ℹ No kernel modules found, assuming monolithic kernel");
                        None
                    }
                    1 => Some(k_info[0].to_owned()),
                    _ => {
                        return Err(Box::new(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Multiple kernels found; please select one explicitly",
                        )))
                    }
                }
            } else {
                // No kernel info available (no modules configured)
                println!("ℹ No kernel modules configured, assuming monolithic kernel");
                None
            };

            println!("Generating initramfs");

            let firmware_list_path = params.get_one::<String>("firmware-list").map(PathBuf::from);
            let firmware_path = PathBuf::from(params.get_one::<String>("root").unwrap());

            IrfsGen::generate(
                kfo.as_ref(),
                cfg,
                PathBuf::from(params.get_one::<String>("output").unwrap()),
                PathBuf::from(params.get_one::<String>("file").unwrap()),
                firmware_list_path,
                firmware_path,
            )?;
        }
    } else {
        clidef::clidef(VERSION, APPNAME, GIT_COMMIT_HASH, ENABLED_FEATURES).print_help().unwrap();
    }

    Ok(())
}

#[allow(clippy::unit_arg)]
fn main() -> Result<(), Box<dyn Error>> {
    let mut cli = clidef::clidef(VERSION, APPNAME, GIT_COMMIT_HASH, ENABLED_FEATURES);
    let params = cli.to_owned().get_matches();
    if params.get_flag("version") {
        println!("Version: {} (commit: {})", VERSION, GIT_COMMIT_HASH);
    } else {
        match match params.subcommand() {
            Some(("new", args)) => run_new(args),
            Some(("analyse", args)) => run_analyse(args),
            Some(("validate", args)) => run_validate(args),
            Some(("info", args)) => run_info(args),
            _ => Ok(cli.print_help()?),
        } {
            Ok(_) => {}
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    Ok(())
}
