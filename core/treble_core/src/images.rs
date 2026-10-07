//! Image format detection: boot (ANDROID! magic) vs system (sparse/ext4).
//!
//! A system image shows folders when opened — Magisk can never patch it,
//! it needs boot/recovery. A GSI leaves recovery untouched stock.

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// What kind of Android image a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// Android boot/recovery image (`ANDROID!` magic).
    Boot,
    /// Sparse (`3A FF 26 ED`) or ext4 (superblock `53 EF` at offset 1080).
    System,
    /// Anything else (or unreadable).
    Unknown,
}

/// Boot image version byte (byte 8 after `ANDROID!`), or `None`.
pub fn boot_magic_ver(path: &Path) -> Option<u8> {
    let mut f = File::open(path).ok()?;
    let mut buf = [0u8; 12];
    let n = f.read(&mut buf).ok()?;
    if n < 8 || &buf[..8] != b"ANDROID!" {
        return None;
    }
    Some(buf[8])
}

/// Classify a file as boot / system / unknown.
pub fn image_kind(path: &Path) -> ImageKind {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return ImageKind::Unknown,
    };
    let mut buf = [0u8; 1082];
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(r) => n += r,
            Err(_) => return ImageKind::Unknown,
        }
    }
    if n >= 8 && &buf[..8] == b"ANDROID!" {
        return ImageKind::Boot;
    }
    if n >= 4 && buf[0] == 0x3A && buf[1] == 0xFF && buf[2] == 0x26 && buf[3] == 0xED {
        return ImageKind::System;
    }
    if n >= 1082 && buf[1080] == 0x53 && buf[1081] == 0xEF {
        return ImageKind::System;
    }
    ImageKind::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("ttcore-test-{name}"));
        let mut f = File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        // pad like real images (header-only files must not confuse detection)
        f.write_all(&[0u8; 100]).unwrap();
        p
    }

    #[test]
    fn boot_detected_with_version() {
        let mut img = b"ANDROID!".to_vec();
        img.push(3);
        let p = tmp("boot.img", &img);
        assert_eq!(image_kind(&p), ImageKind::Boot);
        assert_eq!(boot_magic_ver(&p), Some(3));
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn sparse_is_system() {
        let p = tmp("sparse.img", &[0x3A, 0xFF, 0x26, 0xED]);
        assert_eq!(image_kind(&p), ImageKind::System);
        assert_eq!(boot_magic_ver(&p), None);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn ext4_is_system() {
        let mut img = vec![0u8; 1082];
        img[1080] = 0x53;
        img[1081] = 0xEF;
        let p = tmp("ext4.img", &img);
        assert_eq!(image_kind(&p), ImageKind::System);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn text_is_unknown() {
        let p = tmp("x.txt", b"no android here");
        assert_eq!(image_kind(&p), ImageKind::Unknown);
        assert_eq!(boot_magic_ver(&p), None);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn missing_is_unknown() {
        let mut p = std::env::temp_dir();
        p.push("ttcore-test-does-not-exist.img");
        assert_eq!(image_kind(&p), ImageKind::Unknown);
    }
}
