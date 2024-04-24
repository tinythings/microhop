use nix::kmod::{self, ModuleInitFlags};
use std::{
    ffi::CString,
    fs::File,
    io::{BufReader, Error, Read},
    path::PathBuf,
};
use walkdir::WalkDir;

pub struct KModProbe {
    km_path: PathBuf,
}

impl KModProbe {
    pub fn new() -> Self {
        KModProbe { km_path: PathBuf::from(format!("/lib/modules/{}", uname::uname().unwrap().release)) }
    }

    /// Load a kernel module
    pub fn modprobe(&self, name: &str) {
        let mp: PathBuf = if !name.contains('/') || !name.contains('.') {
            self.find_module(name).unwrap_or_default()
        } else {
            self.km_path.join(name)
        };
        if mp.file_name().unwrap().to_string_lossy().ends_with(".zst") {
            kmod::init_module(&self.unzstd(mp).unwrap(), &CString::new("").unwrap()).unwrap();
        } else {
            kmod::finit_module(File::open(mp).unwrap(), &CString::new("").unwrap(), ModuleInitFlags::empty()).unwrap();
        }
    }

    /// Decompress a zstd binary into a blob in a memory
    fn unzstd(&self, p: PathBuf) -> Result<Vec<u8>, Error> {
        let mut dec = zstd::Decoder::new(BufReader::new(File::open(p).unwrap())).unwrap();
        let mut buff = [0u8; 0x1000];
        let mut data: Vec<u8> = Vec::new();

        loop {
            match dec.read(&mut buff) {
                Ok(0) => break,
                Ok(bts) => data.extend(&buff[..bts]),
                Err(e) => return Err(e),
            }
        }

        Ok(data)
    }

    /// Find module on the filesystem, which corresponds to the current kernel
    fn find_module(&self, name: &str) -> Option<PathBuf> {
        let mut p: Option<PathBuf> = None;
        WalkDir::new(&self.km_path).into_iter().flat_map(|r| r.ok()).for_each(|e| {
            if p.is_none() && e.path().is_file() && e.path().file_name().unwrap().to_str().unwrap().contains(name) {
                p = Some(e.path().to_path_buf());
            }
        });

        p
    }
}
