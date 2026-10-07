//! Container and Android image format detection.
//!
//! Pure parsing only: gzip magic, Android sparse header/chunks, raw.
//! No shell-outs, no device access.

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Outer container / image layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// gzip magic `1F 8B`.
    Gzip,
    /// Android sparse magic `3A FF 26 ED`.
    AndroidSparse,
    /// Anything else (raw ext4, boot image, unknown).
    Raw,
}

/// Parsed Android sparse header (little-endian, spec-fixed layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparseHeader {
    pub block_size: u32,
    pub total_blocks: u32,
    pub total_chunks: u32,
}

/// Detect the container of a file by magic bytes.
pub fn detect_container(path: &Path) -> Container {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Container::Raw,
    };
    let mut magic = [0u8; 4];
    let mut n = 0;
    while n < 4 {
        match f.read(&mut magic[n..]) {
            Ok(0) => break,
            Ok(r) => n += r,
            Err(_) => return Container::Raw,
        }
    }
    if n >= 2 && magic[0] == 0x1F && magic[1] == 0x8B {
        return Container::Gzip;
    }
    if n >= 4 && magic == [0x3A, 0xFF, 0x26, 0xED] {
        return Container::AndroidSparse;
    }
    Container::Raw
}

/// Parse an Android sparse header (28 bytes LE). Returns `None` if invalid.
pub fn parse_sparse_header(path: &Path) -> Option<SparseHeader> {
    let mut f = File::open(path).ok()?;
    let mut h = [0u8; 28];
    let mut n = 0;
    while n < 28 {
        match f.read(&mut h[n..]) {
            Ok(0) => break,
            Ok(r) => n += r,
            Err(_) => return None,
        }
    }
    if n < 28 || h[0..4] != [0x3A, 0xFF, 0x26, 0xED] {
        return None;
    }
    let u16le = |o: usize| u16::from(h[o]) | (u16::from(h[o + 1]) << 8);
    let u32le = |o: usize| {
        u32::from(h[o]) | (u32::from(h[o + 1]) << 8) | (u32::from(h[o + 2]) << 16) | (u32::from(h[o + 3]) << 24)
    };
    if u16le(4) != 1 || u16le(8) != 28 || u16le(10) != 12 {
        return None; // major version / header sizes mismatch
    }
    let block_size = u32le(12);
    let total_blocks = u32le(16);
    let total_chunks = u32le(20);
    if block_size == 0 || block_size % 4 != 0 {
        return None;
    }
    Some(SparseHeader {
        block_size,
        total_blocks,
        total_chunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-image-test-{name}"));
        let mut f = File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        f.write_all(&[0u8; 64]).unwrap();
        p
    }

    #[test]
    fn gzip_detected() {
        let p = tmp("a.gz", &[0x1F, 0x8B, 0x08, 0x00]);
        assert_eq!(detect_container(&p), Container::Gzip);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn sparse_detected_and_parsed() {
        // magic + major=1 + minor=0 + file_hdr=28 + chunk_hdr=12 + blk + blocks + chunks + crc
        let mut h = vec![0x3A, 0xFF, 0x26, 0xED, 0x01, 0x00, 0x00, 0x00];
        h.extend_from_slice(&28u16.to_le_bytes());
        h.extend_from_slice(&12u16.to_le_bytes());
        h.extend_from_slice(&4096u32.to_le_bytes());
        h.extend_from_slice(&100u32.to_le_bytes());
        h.extend_from_slice(&5u32.to_le_bytes());
        h.extend_from_slice(&[0u8; 4]);
        let p = tmp("s.img", &h);
        assert_eq!(detect_container(&p), Container::AndroidSparse);
        assert_eq!(
            parse_sparse_header(&p),
            Some(SparseHeader {
                block_size: 4096,
                total_blocks: 100,
                total_chunks: 5
            })
        );
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn raw_for_plain_and_missing() {
        let p = tmp("r.img", b"plain data here");
        assert_eq!(detect_container(&p), Container::Raw);
        assert_eq!(parse_sparse_header(&p), None);
        std::fs::remove_file(p).ok();
        let mut m = std::env::temp_dir();
        m.push("gsi-image-test-missing.img");
        assert_eq!(detect_container(&m), Container::Raw);
    }

    #[test]
    fn bad_sparse_rejected() {
        let p = tmp("b.img", &[0x3A, 0xFF, 0x26, 0xED, 0x02, 0x00, 0x00, 0x00]);
        assert_eq!(parse_sparse_header(&p), None);
        std::fs::remove_file(p).ok();
    }
}
