//! Native archive handling: gzip/xz single-file decompress, tar/zip list/extract,
//! UPDATE.APP probe (detect only, no parser).
//!
//! Replaces `tar.exe`/`unzip`/python/`gunzip` fallbacks with one code path.
//! Streaming upper bound: decompressed output is capped (zip-bomb guard).

use std::io::Read;
use std::path::{Path, PathBuf};

/// Refuse decompression beyond this many bytes (zip-bomb guard).
pub const MAX_OUTPUT: u64 = 16 * 1024 * 1024 * 1024;

/// Refuse a single zip entry beyond this many bytes (declared or streamed).
/// Boot/recovery images are tens of MB; larger payloads travel as tar/payload.
pub const MAX_ZIP_ENTRY: u64 = 2 * 1024 * 1024 * 1024;

/// Huawei UPDATE.APP record magic (see HuaweiFirmwareExtractor `APP_MAGIC`).
pub const UPDATE_APP_MAGIC: [u8; 4] = [0x55, 0xAA, 0x5A, 0xA5];

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
    extract_tar_bytes(&raw, dest_dir)
}

/// Extract plain tar bytes into `dest_dir` with tar safety.
fn extract_tar_bytes(raw: &[u8], dest_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if raw.len() as u64 > MAX_OUTPUT {
        return Err("output exceeds cap".to_string());
    }
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

/// True for boot/recovery images and payload.bin (case-insensitive).
fn is_wanted_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    let base = lower.rsplit('/').next().unwrap_or("");
    if base == "boot.img" || base == "payload.bin" {
        return true;
    }
    lower.contains("recovery") && lower.ends_with(".img")
}

/// List ZIP entry names (no extraction).
pub fn zip_list(data: &[u8]) -> Result<Vec<String>, String> {
    let mut ar =
        zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| format!("zip: {e}"))?;
    let mut names = Vec::new();
    for idx in 0..ar.len() {
        let f = ar.by_index(idx).map_err(|e| format!("zip entry: {e}"))?;
        names.push(f.name().to_string());
    }
    Ok(names)
}

/// Extract ZIP bytes into `dest_dir`.
/// Refuses absolute and `..` paths. Caps one entry at MAX_ZIP_ENTRY
/// and total output at MAX_OUTPUT. With `only_images`, keeps
/// boot/recovery images plus payload.bin only.
/// Returns extracted relative file paths (dirs made silently).
pub fn extract_zip(
    data: &[u8],
    dest_dir: &Path,
    only_images: bool,
) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("mkdir: {e}"))?;
    let mut ar =
        zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| format!("zip: {e}"))?;
    let mut out = Vec::new();
    let mut total: u64 = 0;
    for idx in 0..ar.len() {
        let f = ar.by_index(idx).map_err(|e| format!("zip entry: {e}"))?;
        let name = f.name().to_string();
        if name.is_empty() {
            return Err("unsafe zip path: empty".to_string());
        }
        if only_images && !is_wanted_image(&name) {
            continue;
        }
        let rel = Path::new(name.as_str()).to_path_buf();
        if rel.is_absolute() || rel.components().any(|c| c == std::path::Component::ParentDir) {
            return Err(format!("unsafe zip path: {}", rel.display()));
        }
        if f.is_dir() {
            std::fs::create_dir_all(dest_dir.join(&rel)).map_err(|e| format!("mkdir: {e}"))?;
            continue;
        }
        if f.size() > MAX_ZIP_ENTRY {
            return Err(format!("zip entry exceeds cap: {name}"));
        }
        let mut buf = Vec::new();
        let mut limited = f.take(MAX_ZIP_ENTRY.saturating_add(1));
        limited
            .read_to_end(&mut buf)
            .map_err(|e| format!("zip read: {e}"))?;
        if buf.len() as u64 > MAX_ZIP_ENTRY {
            return Err(format!("zip entry exceeds cap: {name}"));
        }
        total = match total.checked_add(buf.len() as u64) {
            Some(v) => v,
            None => return Err("output exceeds cap".to_string()),
        };
        if total > MAX_OUTPUT {
            return Err("output exceeds cap".to_string());
        }
        let dest = dest_dir.join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        std::fs::write(&dest, &buf).map_err(|e| format!("write: {e}"))?;
        out.push(rel);
    }
    Ok(out)
}

/// Result of [`probe_update_app`]: container flags plus honest note.
/// UPDATE.APP is proprietary, no parser here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAppProbe {
    /// Starts with UPDATE_APP_MAGIC.
    pub is_update_app: bool,
    /// Starts with gzip magic.
    pub is_gzip: bool,
    /// Starts with ZIP magic.
    pub is_zip: bool,
    /// Input length in bytes.
    pub size: usize,
    /// First up to 32 bytes as upper hex, space separated.
    pub header_hex: String,
    /// Manual path note.
    pub note: &'static str,
}

/// Detect gzip/ZIP/UPDATE.APP container. Pure, no IO, no parsing.
/// UPDATE.APP is proprietary; use huawei_firmware_extractor.py manually.
pub fn probe_update_app(data: &[u8]) -> UpdateAppProbe {
    let is_update_app = data.starts_with(&UPDATE_APP_MAGIC);
    let is_gzip = data.starts_with(&[0x1F, 0x8B]);
    let is_zip = data.starts_with(b"PK\x03\x04")
        || data.starts_with(b"PK\x05\x06")
        || data.starts_with(b"PK\x07\x08");
    let mut header_hex = String::new();
    let mut first = true;
    for b in data.iter().take(32) {
        if !first {
            header_hex.push(' ');
        }
        first = false;
        header_hex.push_str(&format!("{b:02X}"));
    }
    UpdateAppProbe {
        is_update_app,
        is_gzip,
        is_zip,
        size: data.len(),
        header_hex,
        note: "proprietary Huawei UPDATE.APP format: no parser; extract manually with huawei_firmware_extractor.py (place into data/tools/) and keep RECOVERY_RAMDIS(K).img name",
    }
}

/// Write single-file bytes to `dest_dir` using name stem.
fn write_single(raw: &[u8], filename: &str, dest_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let base_full = filename.rsplit('/').next().unwrap_or(filename);
    let base_full = base_full.rsplit('\\').next().unwrap_or(base_full);
    let lower = base_full.to_lowercase();
    let stem_raw = if lower.ends_with(".gz") || lower.ends_with(".xz") {
        let cut = base_full.len().checked_sub(3).unwrap_or(0);
        base_full.get(..cut).unwrap_or("image.img")
    } else {
        base_full
    };
    let stem = if stem_raw.is_empty() || stem_raw == "." || stem_raw == ".." {
        "image.img"
    } else {
        stem_raw
    };
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("mkdir: {e}"))?;
    let dest = dest_dir.join(stem);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    std::fs::write(&dest, raw).map_err(|e| format!("write: {e}"))?;
    Ok(vec![PathBuf::from(stem)])
}

/// Dispatch by file name like the scripts: tar/zip/gz/xz.
/// `filename` selects the kind only; `data` holds the bytes.
/// Single gz/xz writes one file (stem or image.img).
/// Returns relative paths.
pub fn extract_auto(
    filename: &str,
    data: &[u8],
    dest_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    match classify(filename) {
        ArchiveKind::Zip => extract_zip(data, dest_dir, false),
        ArchiveKind::Tar => extract_tar_bytes(data, dest_dir),
        ArchiveKind::TarGz | ArchiveKind::Tgz => {
            let raw = gunzip_bytes(data)?;
            extract_tar_bytes(&raw, dest_dir)
        }
        ArchiveKind::TarXz => {
            let raw = unxz_bytes(data)?;
            extract_tar_bytes(&raw, dest_dir)
        }
        ArchiveKind::GzipSingle => {
            let raw = gunzip_bytes(data)?;
            write_single(&raw, filename, dest_dir)
        }
        ArchiveKind::XzSingle => {
            let raw = unxz_bytes(data)?;
            write_single(&raw, filename, dest_dir)
        }
        ArchiveKind::Plain => Err(format!("unsupported archive: {filename}")),
    }
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

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in files {
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            w.start_file(*name, opts).unwrap();
            w.write_all(*data).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn le16(v: u16) -> [u8; 2] {
        v.to_le_bytes()
    }

    fn le32(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }

    /// Minimal stored ZIP with full control over declared sizes.
    /// `data` is the real bytes on disk; sizes in headers may lie.
    fn raw_zip(name: &str, comp_size: u32, uncomp_size: u32, data: &[u8]) -> Vec<u8> {
        let nb = name.as_bytes();
        let mut out = Vec::new();
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&[0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&le32(comp_size));
        out.extend_from_slice(&le32(uncomp_size));
        out.extend_from_slice(&le16(nb.len() as u16));
        out.extend_from_slice(&[0x00, 0x00]);
        out.extend_from_slice(nb);
        out.extend_from_slice(data);
        let cd_start = out.len() as u32;
        let cdh_start = out.len();
        out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        out.extend_from_slice(&[0x14, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&le32(comp_size));
        out.extend_from_slice(&le32(uncomp_size));
        out.extend_from_slice(&le16(nb.len() as u16));
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&le32(0));
        out.extend_from_slice(nb);
        let cd_size = (out.len() - cdh_start) as u32;
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(&le16(1));
        out.extend_from_slice(&le16(1));
        out.extend_from_slice(&le32(cd_size));
        out.extend_from_slice(&le32(cd_start));
        out.extend_from_slice(&[0x00, 0x00]);
        out
    }

    #[test]
    fn zip_roundtrip() {
        let z = zip_of(&[
            ("boot.img", b"ANDROID!" as &[u8]),
            ("dir/recovery.img", b"ANDROID!" as &[u8]),
            ("readme.txt", b"hi" as &[u8]),
        ]);
        let names = zip_list(&z).unwrap();
        assert!(names.iter().any(|n| n == "boot.img"));
        assert!(names.iter().any(|n| n == "dir/recovery.img"));
        let dir = std::env::temp_dir().join("gsi-archive-test-zip");
        std::fs::create_dir_all(&dir).ok();
        let out = extract_zip(&z, &dir.join("all"), false).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(
            std::fs::read(dir.join("all").join("boot.img")).unwrap(),
            b"ANDROID!"
        );
        let imgs = extract_zip(&z, &dir.join("img"), true).unwrap();
        assert!(imgs.iter().any(|p| p.to_string_lossy().ends_with("boot.img")));
        assert!(!imgs.iter().any(|p| p.to_string_lossy().ends_with("readme.txt")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_traversal_refused() {
        let dir = std::env::temp_dir().join("gsi-archive-test-ztrav");
        std::fs::create_dir_all(&dir).ok();
        let evil = raw_zip("../../evil.img", 3, 3, b"xxx");
        let names = zip_list(&evil).unwrap();
        assert!(names.iter().any(|n| n.contains("..")));
        let err = format!("{:?}", extract_zip(&evil, &dir.join("out"), false).unwrap_err());
        assert!(err.contains("unsafe zip path"), "got: {err}");
        assert!(!dir.join("out").join("evil.img").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_oversized_refused() {
        let dir = std::env::temp_dir().join("gsi-archive-test-zbig");
        std::fs::create_dir_all(&dir).ok();
        let huge = (MAX_ZIP_ENTRY + 1) as u32;
        let big = raw_zip("big.img", 3, huge, b"xxx");
        let err = format!("{:?}", extract_zip(&big, &dir.join("out"), false).unwrap_err());
        assert!(err.contains("exceeds cap"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn probe_detects() {
        let mut app = Vec::from(UPDATE_APP_MAGIC);
        app.extend_from_slice(&[0u8; 64]);
        let p = probe_update_app(&app);
        assert!(p.is_update_app);
        assert!(!p.is_gzip);
        assert!(!p.is_zip);
        assert!(p.note.contains("huawei_firmware_extractor.py"));
        let g = gz_of(b"hello");
        let pg = probe_update_app(&g);
        assert!(pg.is_gzip);
        assert!(!pg.is_update_app);
        assert!(!pg.is_zip);
        let z = zip_of(&[("a.txt", b"hi" as &[u8])]);
        let pz = probe_update_app(&z);
        assert!(pz.is_zip);
        assert!(!pz.is_update_app);
        let empty = probe_update_app(b"");
        assert!(!empty.is_update_app && !empty.is_gzip && !empty.is_zip);
    }

    #[test]
    fn auto_dispatch() {
        let dir = std::env::temp_dir().join("gsi-archive-test-auto");
        std::fs::create_dir_all(&dir).ok();
        let z = zip_of(&[("boot.img", b"ANDROID!" as &[u8])]);
        let out = extract_auto("rom.zip", &z, &dir.join("z")).unwrap();
        assert_eq!(out.len(), 1);
        let t = tar_of(&[("boot.img", b"ANDROID!")]);
        let out2 = extract_auto("rom.tar", &t, &dir.join("t")).unwrap();
        assert_eq!(out2.len(), 1);
        let gz = gz_of(b"ANDROID!123");
        let out3 = extract_auto("boot.img.gz", &gz, &dir.join("g")).unwrap();
        assert_eq!(out3.len(), 1);
        assert!(extract_auto("rom.img", b"raw", &dir.join("p")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
