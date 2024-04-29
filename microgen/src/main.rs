use clap::{builder::styling, Arg, Command, Error};
use colored::Colorize;
use kmoddep::modinfo::lsmod;
use nix::sys::utsname::uname;
use std::env;

static VERSION: &str = "0.0.1";
static APPNAME: &str = "microgen";

pub fn clidef(version: &'static str) -> Command {
    let styles = styling::Styles::styled()
        .header(styling::AnsiColor::Yellow.on_default() | styling::Effects::BOLD)
        .usage(styling::AnsiColor::Yellow.on_default() | styling::Effects::BOLD)
        .literal(styling::AnsiColor::BrightGreen.on_default())
        .placeholder(styling::AnsiColor::BrightRed.on_default());

    Command::new(APPNAME)
        .version(version)
        .about(format!("{} - utility for generating microhop-based initramfs", APPNAME))
        .arg(
            Arg::new("extract")
                .short('x')
                .long("extract")
                .help("Specify comma-separated list of kernel modules to be used.")
                .value_delimiter(','),
        )
        .arg(
            Arg::new("kernel")
                .short('k')
                .long("kernel")
                .help("Kernel release")
                .default_value(format!("{:?}", uname().unwrap().release())),
        )
        .arg(Arg::new("root").short('r').long("root").help("Path to the root filesystem.").default_value("/"))
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .action(clap::ArgAction::SetTrue)
                .help("List available kernel versions")
                .conflicts_with_all(["extract", "kernel", "root"]),
        )
        .disable_version_flag(true)
        .disable_colored_help(false)
        .styles(styles)
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let mut cli = clidef(VERSION);

    if args.len() == 1 {
        return {
            cli.print_help().unwrap();
            Ok(())
        };
    }

    let params = cli.to_owned().get_matches();
    let x_mods: Vec<String> = params.get_many::<String>("extract").unwrap_or_default().map(|s| s.to_string()).collect();

    let infos = kmoddep::get_kernel_infos(params.get_one::<String>("root").unwrap());
    if params.get_flag("list") {
        println!("{}", "Available kernels:".bright_yellow());
        for i in infos {
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
        for knfo in infos {
            let kn = knfo.get_kernel_path().file_name().unwrap().to_str().unwrap().to_string();
            if krel == kn {
                println!("{:?}", knfo.get_deps_for(&x_mods.iter().map(|x| x.to_string()).collect::<Vec<String>>()));
            }
        }
    }

    Ok(())
}
