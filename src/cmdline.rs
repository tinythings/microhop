use std::collections::HashMap;
use std::fs;
use std::io::Error;

pub struct CmdLine {
    params: HashMap<String, String>,
}

impl CmdLine {
    pub fn new() -> Result<Self, Error> {
        Self::new_with_mask(&[])
    }

    pub fn new_with_mask(mask: &[String]) -> Result<Self, Error> {
        let cmdline = fs::read_to_string("/proc/cmdline")?;
        let mut params = HashMap::new();

        for param in cmdline.split_whitespace() {
            if let Some((key, value)) = param.split_once('=') {
                let is_masked = mask.iter().any(|m| m == key);
                if is_masked {
                    log::debug!("Masking cmdline parameter: {}={}", key, value);
                } else {
                    params.insert(key.to_string(), value.to_string());
                }
            } else {
                let is_masked = mask.iter().any(|m| m == param);
                if is_masked {
                    log::debug!("Masking cmdline parameter: {}", param);
                } else {
                    params.insert(param.to_string(), String::new());
                }
            }
        }

        Ok(CmdLine { params })
    }

    pub fn get_root_device(&self) -> Option<&str> {
        self.params.get("root").map(|s| s.as_str())
    }

    pub fn get_root_fstype(&self) -> Option<&str> {
        self.params.get("rootfstype").map(|s| s.as_str())
    }

    pub fn get_root_options(&self) -> Option<&str> {
        self.params.get("rootflags").map(|s| s.as_str())
    }
}

impl Default for CmdLine {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| CmdLine { params: HashMap::new() })
    }
}
