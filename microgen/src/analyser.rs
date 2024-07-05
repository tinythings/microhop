// Analyser of the current system.
// This is used to update/refresh microhop on a current system,
// regenerating it after kernel update.

use profile::cfg::MhConfig;

pub struct SysAnalyser {}

impl SysAnalyser {
    pub fn new() -> SysAnalyser {
        SysAnalyser {}
    }

    /// Look at currently running modules and find out
    /// main ones
    fn get_main_modules() {}

    /// Find current disks
    fn get_diskmap() {}

    /// Return a composed configuration
    pub fn get_config() -> MhConfig {
        todo!()
    }
}
