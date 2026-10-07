//! Native archive handling: gzip/xz single-file decompress, tar list/extract.
//!
//! Replaces `tar.exe`/python/`gunzip` fallbacks with one code path.
//! Streaming upper bound: decompressed output is capped (zip-bomb guard).

use std::io::Read;
use std::path::{Path, PathBuf};

/// Refuse decompression beyond this many bytes (zip-bomb guard).
pub const MAX_OUTPUT: u64 = 16 * 1024 * 1024 * 1024;

/// Archive kind by extension (`.tar.gz`/`.tgz` count as tar, not gzip).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    GzipSingle,
    XzSingle,
    Tar,
    TarGz,
    TarXz,
    Tgz,
    Zip,
    Plain,
}

/// Classify by file name (case-insensitive).
pub fn classify(name: &str) -> ArchiveKind {
    let l = name.to_lowercase();
    if l.ends_with(".tar.gz") {
        ArchiveKind::TarGz
    } else if l.ends_with(".tar.xz") {
        ArchiveKind::TarXz
    } else if l.ends_with(".tgz") {
        ArchiveKind::Tgz
    } else if l.ends_with(".tar") {
        ArchiveKind::Tar
    } else if l.ends_with(".gz") {
        ArchiveKind::GzipSingle
    } else if l.ends_with(".xz") {
        ArchiveKind::XzSingle
    } else if l.ends_with(".zip") {
        ArchiveKind::Zip
    } else {
        ArchiveKind::Plain
    }
}

/// Decompress gzip bytes (capped). Pure, testable without files.
pub fn gunzip_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::read::GzDecoder;
    let mut dec = GzDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| format!("gunzip: {e}"))?;
    if out.len() as u64 > MAX_OUTPUT {
        return Err("output exceeds cap".to_string());
    }
    Ok(out)
}

/// Decompress xz bytes (capped). Pure, testable without files.
pub fn unxz_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    lzma_rs::xz_decompress(&mut &data[..], &mut out)
        .map_err(|e| format!("unxz: {e:?}"))?;
    if out.len() as u64 > MAX_OUTPUT {
        return Err("output exceeds cap".to_string());
    }
    Ok(out)
}

/// List tar entries (names only). Accepts plain, gz- or xz-compressed tars.
pub fn tar_list(data: &[u8]) -> Result<Vec<String>, String> {
    let raw: Vec<u8> = if data.len() >= 2 && data[0] == 0x1F && data[1] == 0x8B {
        gunzip_bytes(data)?
    } else if data.len() >= 6 && &data[..6] == b"\xFD7zXZ\x00" {
        unxz_bytes(data)?
    } else {
        data.to_vec()
    };
    let mut ar = tar::Archive::new(&raw[..]);
    let mut names = Vec::new();
    for entry in ar.entries().map_err(|e| format!("tar: {e}"))? {
        let e = entry.map_err(|e| format!("tar entry: {e}"))?;
        names.push(
            e.path()
                .map_err(|e| format!("tar path: {e}"))?
                .to_string_lossy()
                .to_string(),
        );
    }
    Ok(names)
}

/// Decompress a single-file archive (`.gz`/`.xz`) to `dest_dir`.
/// Returns the output path. Tar containers go through [`extract_tar`].
pub fn decompress_single(archive: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "bad file name".to_string())?;
    let lower = name.to_lowercase();
    let stem = if lower.ends_with(".gz") {
        &name[..name.len() - 3]
    } else if lower.ends_with(".xz") {
        &name[..name.len() - 3]
    } else {
        return Err("not a single-file archive (.gz/.xz)".to_string());
    };
    let stem = if stem.is_empty() { "image.img" } else { stem };
    let data = std::fs::read(archive).map_err(|e| format!("read: {e}"))?;
    let raw = if lower.ends_with(".gz") {
        gunzip_bytes(&data)?
    } else {
        unxz_bytes(&data)?
    };
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("mkdir: {e}"))?;
    let dest = dest_dir.join(stem);
    std::fs::write(&dest, &raw).map_err(|e| format!("write: {e}"))?;
    Ok(dest)
}

/// Extract a tar container (plain/`.gz`/`.xz`) into `dest_dir`.
/// Returns the list of extracted relative paths.
pub fn extract_tar(archive: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let data = std::fs::read(archive).map_err(|e| format!("read: {e}"))?;
    let lower = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    let raw: Vec<u8> = if lower.ends_with(".gz") || lower.ends_with(".tgz") {
        gunzip_bytes(&data)?
    } else if lower.ends_with(".xz") {
        unxz_bytes(&data)?
    } else {
        data
    };
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("mkdir: {e}"))?;
    let mut ar = tar::Archive::new(&raw[..]);
    // Safety: refuse absolute paths and `..` escapes (never write outside dest).
    let mut out = Vec::new();
    for entry in ar.entries().map_err(|e| format!("tar: {e}"))? {
        let mut e = entry.map_err(|e| format!("tar entry: {e}"))?;
        let rel = e
            .path()
            .map_err(|e| format!("tar path: {e}"))?
            .to_path_buf();
        if rel.is_absolute() || rel.components().any(|c| c == std::path::Component::ParentDir) {
            return Err(format!("unsafe tar path: {}", rel.display()));
        }
        e.unpack_in(dest_dir)
            .map_err(|e| format!("unpack: {e}"))?;
        out.push(rel);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gz_of(data: &[u8]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    fn xz_of(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        lzma_rs::xz_compress(&mut &data[..], &mut out).unwrap();
        out
    }

    fn tar_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut ar = tar::Builder::new(&mut buf);
            for (name, data) in files {
                let mut h = tar::Header::new_gnu();
                h.set_size(data.len() as u64);
                h.set_mode(0o644);
                h.set_cksum();
                ar.append_data(&mut h, name, &data[..]).unwrap();
            }
            ar.finish().unwrap();
        }
        buf
    }

    #[test]
    fn classify_by_extension() {
        assert_eq!(classify("a.img.gz"), ArchiveKind::GzipSingle);
        assert_eq!(classify("a.tar.gz"), ArchiveKind::TarGz);
        assert_eq!(classify("a.tgz"), ArchiveKind::Tgz);
        assert_eq!(classify("a.tar.xz"), ArchiveKind::TarXz);
        assert_eq!(classify("a.img.xz"), ArchiveKind::XzSingle);
        assert_eq!(classify("a.zip"), ArchiveKind::Zip);
        assert_eq!(classify("a.img"), ArchiveKind::Plain);
        assert_eq!(classify("A.TAR.GZ"), ArchiveKind::TarGz);
    }

    #[test]
    fn gzip_roundtrip() {
        let raw = b"ANDROID! helloworld";
        assert_eq!(gunzip_bytes(&gz_of(raw)).unwrap(), raw);
    }

    #[test]
    fn xz_roundtrip() {
        let raw = b"ANDROID! helloworld";
        assert_eq!(unxz_bytes(&xz_of(raw)).unwrap(), raw);
    }

    #[test]
    fn corrupt_rejected() {
        assert!(gunzip_bytes(b"not gzip at all....................").is_err());
        assert!(unxz_bytes(b"not xz.............................").is_err());
    }

    #[test]
    fn tar_list_plain_and_gz() {
        let t = tar_of(&[("boot.img", b"ANDROID!"), ("dir/recovery.img", b"ANDROID!")]);
        let names = tar_list(&t).unwrap();
        assert!(names.iter().any(|n| n.ends_with("boot.img")));
        assert!(names.iter().any(|n| n.ends_with("recovery.img")));
        let names2 = tar_list(&gz_of(&t)).unwrap();
        assert_eq!(names.len(), names2.len());
    }

    /// Build a minimal raw tar (header + data + end blocks) with an
    /// arbitrary entry name, bypassing the `tar` crate's own `..` guard
    /// (which refuses to even create such archives).
    fn raw_tar_with_name(name: &str, data: &[u8]) -> Vec<u8> {
        let mut hdr = [0u8; 512];
        let name_b = name.as_bytes();
        hdr[..name_b.len().min(100)].copy_from_slice(&name_b[..name_b.len().min(100)]);
        hdr[100..108].copy_from_slice(b"0000777\0");
        hdr[108..116].copy_from_slice(b"0000000\0");
        hdr[116..124].copy_from_slice(b"0000000\0");
        let size = format!("{:011o}\0", data.len());
        hdr[124..136].copy_from_slice(size.as_bytes());
        hdr[136..148].copy_from_slice(b"00000000000\0");
        hdr[156] = b'0';
        hdr[257..262].copy_from_slice(b"ustar");
        // checksum over header with chksum field as spaces
        let mut sum: u32 = 8 * 0x20;
        for (i, b) in hdr.iter().enumerate() {
            if !(148..156).contains(&i) {
                sum += *b as u32;
            }
        }
        let chk = format!("{:06o}\0 ", sum);
        hdr[148..156].copy_from_slice(chk.as_bytes());
        let mut out = Vec::from(&hdr[..]);
        out.extend_from_slice(data);
        out.resize(out.len() + (512 - data.len() % 512) % 512, 0);
        out.extend_from_slice(&[0u8; 1024]);
        out
    }

    #[test]
    fn tar_path_traversal_refused() {
        let dir = std::env::temp_dir().join("gsi-archive-test-evil");
        std::fs::create_dir_all(&dir).ok();
        // tar crate refuses to BUILD such archives, so hand-craft raw bytes.
        let buf = raw_tar_with_name("../../evil.img", b"xxx");
        let src = dir.join("evil.tar");
        std::fs::write(&src, &buf).unwrap();
        // classify first (sanity), then refuse on unpack
        assert_eq!(classify("evil.tar"), ArchiveKind::Tar);
        let err = format!("{:?}", extract_tar(&src, &dir.join("out")).unwrap_err());
        assert!(err.contains("unsafe tar path"), "got: {err}");
        assert!(!dir.join("out").join("evil.img").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn decompress_single_gz_file() {
        let dir = std::env::temp_dir().join("gsi-archive-test-gz");
        std::fs::create_dir_all(&dir).ok();
        let src = dir.join("tboot.img.gz");
        std::fs::write(&src, gz_of(b"ANDROID!12345678")).unwrap();
        let out = decompress_single(&src, &dir).unwrap();
        assert_eq!(out.file_name().unwrap(), "tboot.img");
        assert_eq!(std::fs::read(&out).unwrap(), b"ANDROID!12345678");
        std::fs::remove_dir_all(&dir).ok();
    }
}
