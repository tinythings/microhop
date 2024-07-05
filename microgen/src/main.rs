mod analyser;
mod clidef;
mod rdgen;
mod rdpack;

use clap::{ArgMatches, Error};
use colored::Colorize;
use kmoddep::{kerman::KernelInfo, modinfo::lsmod};
use rdgen::IrfsGen;
use std::path::PathBuf;

static VERSION: &str = "0.0.9";
static APPNAME: &str = "microgen";

/// Run information section
fn run_info(params: &ArgMatches) -> Result<(), Error> {
    let rfs = params.get_one::<String>("list").map(|v| v.as_str());
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

/// Run analysis and profile generator
fn run_analyse(_params: &ArgMatches) -> Result<(), Error> {
    println!("Not implemented yet :-(");
    Ok(())
}

/// Create a new initramfs
fn run_new(params: &ArgMatches) -> Result<(), Error> {
    let x_mods: Vec<String> = params.get_many::<String>("extract").unwrap_or_default().map(|s| s.to_string()).collect();
    let k_info = kmoddep::get_kernel_infos(Some(params.get_one::<String>("root").unwrap()));
    let profile = params.get_one::<String>("config");
    if let Err(k_info) = k_info {
        println!("Unable to get the information about the kernel: {}", k_info);
        return Ok(());
    }

    if !x_mods.is_empty() {
        let krel = params.get_one::<String>("kernel").unwrap().replace('"', "");
        for knfo in k_info.unwrap() {
            let kn = knfo.get_kernel_path().file_name().unwrap().to_str().unwrap().to_string();
            if krel == kn {
                println!("{:?}", knfo.get_deps_for(&x_mods.iter().map(|x| x.to_string()).collect::<Vec<String>>()));
            }
        }
    } else if let Some(profile) = profile {
        let cfg = profile::cfg::get_mh_config(Some(profile))?;
        if let Ok(k_info) = k_info {
            let kfo: KernelInfo;

            // Rewrite this better
            if k_info.len() > 1 {
                panic!("Need to implement matching a proper kernel from CLI")
            } else {
                kfo = k_info[0].to_owned();
            }
            println!("Generating initramfs");
            IrfsGen::generate(
                &kfo,
                cfg,
                PathBuf::from(params.get_one::<String>("output").unwrap()),
                PathBuf::from(params.get_one::<String>("file").unwrap()),
            )?;
        }
    } else {
        clidef::clidef(VERSION, APPNAME).print_help().unwrap();
    }

    Ok(())
}

#[allow(clippy::unit_arg)]
fn main() -> Result<(), Error> {
    let mut cli = clidef::clidef(VERSION, APPNAME);
    let params = cli.to_owned().get_matches();
    if params.get_flag("version") {
        println!("Version: {}", VERSION);
    } else {
        match match params.subcommand() {
            Some(("new", args)) => run_new(args),
            Some(("analyse", args)) => run_analyse(args),
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
