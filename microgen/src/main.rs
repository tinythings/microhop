mod clidef;

use clap::Error;
use colored::Colorize;
use kmoddep::modinfo::lsmod;
use std::env;

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

    profile::add(1, 2);

    let params = cli.to_owned().get_matches();
    let x_mods: Vec<String> = params.get_many::<String>("extract").unwrap_or_default().map(|s| s.to_string()).collect();
    let k_info = kmoddep::get_kernel_infos(params.get_one::<String>("root").unwrap());
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
    }

    Ok(())
}
