use cpio::{newc::trailer, NewcBuilder};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Cursor, Error, Write},
    path::PathBuf,
    vec,
};
use walkdir::WalkDir;
use zstd::stream::write::Encoder;

/// Packs initramfs to CPIO archive
struct InitRamfsPacker {
    path: String,
    files: Vec<PathBuf>,
}

impl InitRamfsPacker {
    fn new(p: &str) -> Self {
        {
            InitRamfsPacker { path: p.to_string(), files: vec![] }
        }
    }

    /// Find all the required files
    fn get_content(&mut self) {
        self.files = WalkDir::new(&self.path)
            .follow_links(true)
            .into_iter()
            .flatten()
            .map(|e| e.path().to_owned())
            .collect::<Vec<PathBuf>>();
    }

    /// Build file metadata and information for the CPIO archive
    fn file_loader(inode: u32, p: &str) -> io::Result<(NewcBuilder, Vec<u8>)> {
        let pth = PathBuf::from(p);
        let mut data: Vec<u8> = vec![];

        let f_meta = fs::metadata(&pth).unwrap();
        let arc_meta;
        if pth.is_symlink() {
            data.extend(fs::read_link(pth).unwrap().to_str().unwrap().as_bytes());
            arc_meta = NewcBuilder::new(p)
                .ino(inode)
                .uid(0)
                .gid(f_meta.gid())
                .mode(f_meta.permissions().mode())
                .set_mode_file_type(cpio::newc::ModeFileType::Symlink);
        } else if pth.is_file() {
            data.extend(fs::read(&pth)?);
            arc_meta = NewcBuilder::new(p)
                .ino(inode)
                .uid(0)
                .gid(f_meta.gid())
                .mode(f_meta.permissions().mode())
                .set_mode_file_type(cpio::newc::ModeFileType::Regular);
        } else {
            arc_meta = NewcBuilder::new(p)
                .ino(inode)
                .uid(0)
                .gid(f_meta.gid())
                .mode(f_meta.permissions().mode())
                .set_mode_file_type(cpio::newc::ModeFileType::Directory);
        }

        Ok((arc_meta, data))
    }

    fn pack(&mut self, output: &str) -> Result<(), Error> {
        self.get_content();

        let all_data: Vec<u8> = vec![];
        let cur = Cursor::new(all_data);
        let mut out = BufWriter::new(cur);

        let mut inode = 1;
        for fp in &self.files {
            let (bdr, data) = InitRamfsPacker::file_loader(inode, fp.to_str().unwrap()).unwrap();
            let mut w = bdr.write(&mut out, data.len() as u32);
            w.write_all(&data)?;
            w.finish().unwrap();
            inode += 1;
        }

        let out = trailer(out).unwrap();
        let mut encoder = Encoder::new(File::create(format!("../{}", output)).unwrap(), 10).unwrap();
        encoder.write_all(&out.into_inner().unwrap().into_inner())?;
        encoder.finish()?;

        Ok(())
    }
}

/// Pack a content of a path into a CPIO archive
pub fn pack(p: &str) -> Result<(), Error> {
    InitRamfsPacker::new(".").pack(p)
}
