use colored::{self, Colorize};
use log::{Level, Metadata, Record};

pub(crate) struct STDOUTLogger;

impl log::Log for STDOUTLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, msg: &Record) {
        if self.enabled(msg.metadata()) {
            let m = format!("{}", msg.args());
            let l_msg: String = match msg.level() {
                log::Level::Info => format!("{}", m.bright_green()),
                log::Level::Warn => format!("{}", m.yellow()),
                log::Level::Error => format!("{}", m.bright_red()),
                log::Level::Debug => format!("{}", m.cyan()),
                log::Level::Trace => format!("{}", m.cyan()),
            };

            let tsb = nix::time::clock_gettime(nix::time::ClockId::CLOCK_BOOTTIME);
            let dsb = std::time::Duration::from(tsb.unwrap());
            println!("[{:12.6}][{:>6}] {}", dsb.as_secs_f32(), format!("T{}", dsb.as_secs()), l_msg);
        }
    }

    fn flush(&self) {}
}
