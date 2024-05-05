mod clidef;
mod rdgen;

use clap::Error;
use colored::Colorize;
use kmoddep::{kerman::KernelInfo, modinfo::lsmod};
use rdgen::IrfsGen;
use std::{env, path::PathBuf};

static VERSION: &str = "0.0.1";
static APPNAME: &str = "microgen";

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let mut cli = clidef::clidef(VERSION, APPNAME);

    if args.len() == 1 {
        return {
            cli.print_help().unwrap();
            Ok(())
        };
    }

    let params = cli.to_owned().get_matches();
    let x_mods: Vec<String> = params.get_many::<String>("extract").unwrap_or_default().map(|s| s.to_string()).collect();
    let k_info = kmoddep::get_kernel_infos(Some(params.get_one::<String>("root").unwrap()));
    let profile = params.get_one::<String>("config");
    if let Err(k_info) = k_info {
        println!("Error: {}", k_info);
        return Ok(());
    }

    if params.get_flag("list") {
        println!("{}", "Available kernels:".bright_yellow());
        for i in k_info.unwrap() {
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
    } else if !x_mods.is_empty() {
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
            println!("Generating");
            IrfsGen::generate(&kfo, cfg, PathBuf::from("./build"))?;
        }
    }

    Ok(())
}
