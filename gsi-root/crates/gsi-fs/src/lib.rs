//! Read-only ext4 inspection: superblock, extents, directories, symlinks,
//! regular files, xattrs (SELinux contexts).
//!
//! Scope: read + inspect only (writer comes next, separate review).
//! Verified against a real 2.8 GB LineageOS-GSI image cross-checked
//! with `debugfs` (see `gsi-root/docs/VERIFICATION-lineage20-20251021.md`).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// ext4 superblock essentials.
#[derive(Debug, Clone)]
pub struct Superblock {
    pub block_size: u64,
    pub blocks_per_group: u64,
    pub inodes_per_group: u64,
    pub inode_size: u16,
    pub desc_size: u16,
    pub groups: u64,
    pub is_64bit: bool,
}

/// Opened ext4 image (random access over the file, nothing loaded fully).
pub struct Ext4 {
    f: File,
    pub sb: Superblock,
}

fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from(b[o]) | (u16::from(b[o + 1]) << 8)
}
fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from(b[o])
        | (u32::from(b[o + 1]) << 8)
        | (u32::from(b[o + 2]) << 16)
        | (u32::from(b[o + 3]) << 24)
}

impl Ext4 {
    /// Open + validate superblock magic.
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut f = File::open(path).map_err(|e| format!("open: {e}"))?;
        let mut sb = [0u8; 1024];
        f.seek(SeekFrom::Start(1024))
            .map_err(|e| format!("seek: {e}"))?;
        f.read_exact(&mut sb).map_err(|e| format!("read: {e}"))?;
        if u16le(&sb, 56) != 0xEF53 {
            return Err("not ext4 (bad magic)".to_string());
        }
        let log_bs = u32le(&sb, 24);
        if log_bs > 16 {
            return Err("implausible block size".to_string());
        }
        let block_size = 1024u64 << log_bs;
        let blocks_per_group = u32le(&sb, 32) as u64;
        let inodes_per_group = u32le(&sb, 40) as u64;
        let inode_size = u16le(&sb, 88);
        if !(128..=1024).contains(&inode_size) || inode_size % 8 != 0 {
            return Err("implausible inode size".to_string());
        }
        let feat_incompat = u32le(&sb, 96);
        let is_64bit = feat_incompat & 0x80 != 0;
        let desc_size = if is_64bit {
            let d = u16le(&sb, 254);
            if d == 0 { 64 } else { d }
        } else {
            32
        };
        let total_blocks =
            u32le(&sb, 4) as u64 | ((u32le(&sb, 328) as u64) << 32);
        let groups = total_blocks.div_ceil(blocks_per_group.max(1));
        Ok(Self {
            f,
            sb: Superblock {
                block_size,
                blocks_per_group,
                inodes_per_group,
                inode_size,
                desc_size,
                groups,
                is_64bit,
            },
        })
    }

    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<(), String> {
        self.f
            .seek(SeekFrom::Start(off))
            .map_err(|e| format!("seek: {e}"))?;
        self.f
            .read_exact(buf)
            .map_err(|e| format!("read: {e}"))?;
        Ok(())
    }

    fn read_block(&mut self, blk: u64) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; self.sb.block_size as usize];
        self.read_at(blk * self.sb.block_size, &mut buf)?;
        Ok(buf)
    }

    /// Block number of an inode's table entry.
    fn inode_pos(&mut self, ino: u32) -> Result<(u64, usize), String> {
        if ino == 0 {
            return Err("inode 0".to_string());
        }
        let idx = (ino - 1) as u64;
        let grp = idx / self.sb.inodes_per_group;
        if grp >= self.sb.groups {
            return Err("inode group out of range".to_string());
        }
        let gdt_blk = if self.sb.block_size == 1024 { 2 } else { 1 };
        let mut desc = vec![0u8; self.sb.desc_size as usize];
        let off = (gdt_blk * self.sb.block_size) + grp * self.sb.desc_size as u64;
        self.read_at(off, &mut desc)?;
        let mut table = u32le(&desc, 8) as u64;
        if self.sb.is_64bit && desc.len() >= 36 {
            table |= (u32le(&desc, 32) as u64) << 32;
        }
        let inner = (idx % self.sb.inodes_per_group) as usize;
        Ok((
            table * self.sb.block_size + inner as u64 * self.sb.inode_size as u64,
            self.sb.inode_size as usize,
        ))
    }

    /// Raw inode bytes.
    pub fn read_inode(&mut self, ino: u32) -> Result<Vec<u8>, String> {
        let (off, len) = self.inode_pos(ino)?;
        let mut buf = vec![0u8; len];
        self.read_at(off, &mut buf)?;
        Ok(buf)
    }

    /// Data blocks of a file via the extent tree (depth 0..n supported).
    pub fn file_blocks(&mut self, ino: u32) -> Result<Vec<u64>, String> {
        let raw = self.read_inode(ino)?;
        // i_flags is at byte 32 (measured on a real image; 28 is i_blocks_lo).
        let flags = u32le(&raw, 32);
        if flags & 0x80000 == 0 {
            return Err("no extents flag".to_string());
        }
        let mut out = Vec::new();
        // extent header lives at i_block[0] = offset 40, 12 bytes.
        self.collect_extents(&raw[40..], &mut out)?;
        Ok(out)
    }

    fn collect_extents(&mut self, node: &[u8], out: &mut Vec<u64>) -> Result<(), String> {
        if node.len() < 12 {
            return Err("short extent node".to_string());
        }
        if u16le(node, 0) != 0xF30A {
            return Err("bad extent magic".to_string());
        }
        let entries = u16le(node, 2) as usize;
        let depth = u16le(node, 6);
        if depth == 0 {
            for i in 0..entries {
                let o = 12 + i * 12;
                if o + 12 > node.len() {
                    return Err("short extent entry".to_string());
                }
                // ee_block(4) ee_len(2) ee_start_hi(2) ee_start_lo(4)
                let len = u16le(node, o + 4) as u64;
                let start = u32le(node, o + 8) as u64
                    | ((u16le(node, o + 6) as u64) << 32);
                // len 0x8000+ means uninitialized; cap sane length anyway.
                let len = (len & 0x7FFF).max(1).min(1 << 20);
                for b in 0..len {
                    out.push(start + b);
                }
            }
            return Ok(());
        }
        for i in 0..entries {
            let o = 12 + i * 12;
            if o + 12 > node.len() {
                return Err("short extent index".to_string());
            }
            let child = u32le(node, o + 8) as u64 | ((u32le(node, o + 4) as u64) << 32);
            let blk = self.read_block(child)?;
            // Recurse into a copy (borrow-safe): parse from owned block.
            let owned = blk;
            self.collect_extents(&owned, out)?;
        }
        Ok(())
    }

    /// Read a whole regular file (follows extents).
    pub fn read_file(&mut self, ino: u32) -> Result<Vec<u8>, String> {
        let raw = self.read_inode(ino)?;
        let mode = u16le(&raw, 0);
        if mode & 0xF000 != 0x8000 {
            return Err("not a regular file".to_string());
        }
        let size = u32le(&raw, 4) as u64 | ((u32le(&raw, 108) as u64) << 32);
        let mut data = Vec::new();
        for b in self.file_blocks(ino)? {
            if data.len() as u64 >= size {
                break;
            }
            data.extend_from_slice(&self.read_block(b)?);
        }
        data.truncate(size as usize);
        Ok(data)
    }

    /// Read a symlink (fast inline or block-backed).
    pub fn read_symlink(&mut self, ino: u32) -> Result<String, String> {
        let raw = self.read_inode(ino)?;
        let mode = u16le(&raw, 0);
        if mode & 0xF000 != 0xA000 {
            return Err("not a symlink".to_string());
        }
        let size = u32le(&raw, 4) as usize;
        if size < 60 {
            let end = raw[40..].iter().position(|&c| c == 0).unwrap_or(size.min(60));
            return String::from_utf8(raw[40..40 + end].to_vec())
                .map_err(|e| format!("utf8: {e}"));
        }
        let mut data = Vec::new();
        for b in self.file_blocks(ino)? {
            data.extend_from_slice(&self.read_block(b)?);
            if data.len() >= size {
                break;
            }
        }
        data.truncate(size);
        String::from_utf8(data).map_err(|e| format!("utf8: {e}"))
    }

    /// Directory entries: (inode, name, file_type).
    pub fn read_dir(&mut self, ino: u32) -> Result<Vec<(u32, String, u8)>, String> {
        let raw = self.read_inode(ino)?;
        let mode = u16le(&raw, 0);
        if mode & 0xF000 != 0x4000 {
            return Err("not a directory".to_string());
        }
        let mut out = Vec::new();
        for b in self.file_blocks(ino)? {
            let blk = self.read_block(b)?;
            let mut o = 0usize;
            while o + 8 <= blk.len() {
                let e_ino = u32le(&blk, o);
                let rec_len = u16le(&blk, o + 4) as usize;
                let name_len = blk[o + 6] as usize;
                let ftype = blk[o + 7];
                if rec_len < 8 || o + rec_len > blk.len() {
                    break;
                }
                if e_ino != 0 && o + 8 + name_len <= blk.len() {
                    let name =
                        String::from_utf8_lossy(&blk[o + 8..o + 8 + name_len]).to_string();
                    if name != "." && name != ".." {
                        out.push((e_ino, name, ftype));
                    }
                }
                o += rec_len;
            }
        }
        Ok(out)
    }

    /// Resolve an absolute path to an inode number.
    pub fn lookup(&mut self, path: &str) -> Result<u32, String> {
        let mut ino = 2u32;
        for comp in path.split('/').filter(|c| !c.is_empty()) {
            let mut found = None;
            for (e_ino, name, _) in self.read_dir(ino)? {
                if name == comp {
                    found = Some(e_ino);
                    break;
                }
            }
            ino = found.ok_or_else(|| format!("not found: {comp}"))?;
        }
        Ok(ino)
    }

    /// SELinux context (`security.selinux` xattr) of an inode, if present.
    /// SELinux context (`security.selinux` xattr) of an inode, if present.
    ///
    /// Layout (measured on a real image, debugfs cross-checked): entry fields
    /// are name_len(1) name_index(1) pad(1) value_offs(2) value_inum(4)
    /// value_size(4) + name, 4-aligned. The inline area may start with the
    /// u32 magic `0xEA020000` (4 bytes); external xattr blocks start with a
    /// 16-byte header (magic+refcount+blocks+checksum). `e_value_offs` counts
    /// from the ENTRY start (measured); block-absolute is tried as fallback.
    /// Name index 6 = `security.`.
    pub fn selinux_context(&mut self, ino: u32) -> Result<Option<String>, String> {
        let raw = self.read_inode(ino)?;
        let mut areas: Vec<(Vec<u8>, bool)> = Vec::new();
        // Inline area (only if the inode is big enough to hold it).
        if raw.len() >= 132 {
            let extra = u16le(&raw, 128) as usize;
            if 128 + extra <= raw.len() {
                areas.push((raw[128 + extra..].to_vec(), false));
            }
        }
        // External xattr block via i_file_acl_lo (offset 104) — independent
        // of inode size.
        if raw.len() >= 108 {
            let acl = u32le(&raw, 104);
            if acl != 0 {
                let blk = self.read_block(acl as u64).unwrap_or_default();
                if !blk.is_empty() {
                    areas.push((blk, true));
                }
            }
        }
        for (area, is_block) in &areas {
            let mut q = 0usize;
            if area.len() >= 4 && u32le(area, 0) == 0xEA020000 {
                q = if *is_block { 16 } else { 4 };
            }
            let mut entries: Vec<(usize, String, usize, usize)> = Vec::new();
            let mut pp = q;
            // Entry: name_len(1) name_index(1) value_offs(2) value_inum(4)
            // value_size(4) + name, 4-aligned. No pad byte after index.
            while pp + 16 <= area.len() {
                let nl = area[pp] as usize;
                if nl == 0 {
                    break;
                }
                let vo = u16le(area, pp + 2) as usize;
                let vs = u32le(area, pp + 8) as usize;
                if pp + 16 + nl > area.len() {
                    break;
                }
                let prefix = match area[pp + 1] {
                    1 => "user.",
                    4 => "trusted.",
                    6 => "security.",
                    7 => "system.",
                    _ => "",
                };
                if prefix.is_empty() {
                    break;
                }
                let nm = format!(
                    "{}{}",
                    prefix,
                    String::from_utf8_lossy(&area[pp + 16..pp + 16 + nl])
                );
                entries.push((pp, nm, vo, vs));
                pp += ((16 + nl + 3) / 4) * 4;
            }
            for (eo, nm, vo, vs) in &entries {
                if nm != "security.selinux" || *vs == 0 || *vs > 4096 {
                    continue;
                }
                // Measured ground truth: inline areas count e_value_offs
                // from the ENTRY start; external xattr blocks count it from
                // the BLOCK start. Try the matching convention first.
                let order: [usize; 2] = if *is_block {
                    [*vo, *eo + *vo]
                } else {
                    [*eo + *vo, *vo]
                };
                for base in order {
                    if base + vs <= area.len() {
                        let raw_v = &area[base..base + vs];
                        if let Ok(s) = std::str::from_utf8(raw_v) {
                            let s = s.trim_end_matches('\0');
                            if s.contains("_u:") || s.contains(":s0") {
                                return Ok(Some(s.to_string()));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn put16(v: &mut [u8], o: usize, x: u16) {
        v[o] = (x & 0xFF) as u8;
        v[o + 1] = (x >> 8) as u8;
    }
    fn put32(v: &mut [u8], o: usize, x: u32) {
        for i in 0..4 {
            v[o + i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
    }

    /// Minimal synthetic ext4 (1K blocks): superblock, 1 group, root dir with
    /// one file (extent) + one fast symlink, file carries inline SELinux xattr.
    fn fixture() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let bs = 1024usize;
        // blocks: 0 boot, 1 super, 2 gdt, 3 bbitmap, 4 ibitmap, 5..6 itable (16 inodes x 128B), 7 rootdir, 8 file, 9 xattr
        let nblocks = 10usize;
        let mut img = vec![0u8; nblocks * bs];
        // superblock at 1024
        put32(&mut img, 1024 + 0, 16); // inodes_count
        put32(&mut img, 1024 + 4, nblocks as u32); // blocks_count_lo
        put32(&mut img, 1024 + 20, 1); // first_data_block
        put32(&mut img, 1024 + 24, 0); // log_block_size -> 1K
        put32(&mut img, 1024 + 32, 8192); // blocks_per_group
        put32(&mut img, 1024 + 40, 16); // inodes_per_group
        put16(&mut img, 1024 + 56, 0xEF53); // magic
        put16(&mut img, 1024 + 88, 128); // inode_size
        // group desc at block 2
        put32(&mut img, 2 * bs + 8, 5); // inode table block
        let mut inode = |ino: usize, mode: u16, size: u32, flags: u32, blocks: &[(u64, u64)]| {
            let off = 5 * bs + (ino - 1) * 128;
            put16(&mut img, off, mode);
            put32(&mut img, off + 4, size);
            put32(&mut img, off + 32, flags);
            // extent header at i_block
            put16(&mut img, off + 40, 0xF30A);
            put16(&mut img, off + 42, blocks.len() as u16);
            put16(&mut img, off + 44, 4);
            for (i, (start, len)) in blocks.iter().enumerate() {
                let o = off + 52 + i * 12;
                put32(&mut img, o, 0); // ee_block (logical, 0-based here)
                put16(&mut img, o + 4, *len as u16);
                put16(&mut img, o + 6, (*start >> 32) as u16);
                put32(&mut img, o + 8, (*start & 0xFFFF_FFFF) as u32);
            }
        };
        // root ino 2: dir, extent -> block 7
        inode(2, 0x41ED, bs as u32, 0x80000, &[(7, 1)]);
        // file ino 12: regular, extent -> block 8 ("hello"), xattr inline
        inode(12, 0x81A4, 5, 0x80000, &[(8, 1)]);
        img[8 * bs..8 * bs + 5].copy_from_slice(b"hello");
        // hello.txt dirent in block 7: "." (12), ".." (12), "hello.txt" (20), "link" (16)
        let mut d = 0usize;
        let mut dent = |ino: u32, name: &[u8]| {
            put32(&mut img, 7 * bs + d, ino);
            let reclen = ((8 + name.len() + 3) / 4) * 4;
            put16(&mut img, 7 * bs + d + 4, reclen as u16);
            img[7 * bs + d + 6] = name.len() as u8;
            img[7 * bs + d + 7] = if name == b"link" { 7 } else { 1 };
            img[7 * bs + d + 8..7 * bs + d + 8 + name.len()].copy_from_slice(name);
            d += reclen;
        };
        dent(2, b".");
        dent(2, b"..");
        dent(12, b"hello.txt");
        dent(13, b"link");
        // symlink ino 13: fast inline -> "hello.txt"
        {
            let off = 5 * bs + 12 * 128;
            put16(&mut img, off, 0xA1FF);
            put32(&mut img, off + 4, 9);
            img[off + 40..off + 49].copy_from_slice(b"hello.txt");
        }
        // inline xattr on ino 12: reuse area 128.. of inode (inode_size=128: no room!)
        // -> use xattr BLOCK 9 instead: header(16) + entry + value.
        {
            // point file_acl_lo of ino 12 at block 9
            let off = 5 * bs + 11 * 128;
            put32(&mut img, off + 104, 9);
            let b = 9 * bs;
            put32(&mut img, b, 0xEA020000);
            put32(&mut img, b + 4, 1);
            // entry at 16: len=7 idx=6 voff, size=24, name (no pad byte)
            let ctx = b"u:object_r:test_file:s0\0";
            let voff = 128usize; // absolute from block start
            img[b + 16] = 7;
            img[b + 17] = 6;
            put16(&mut img, b + 18, voff as u16);
            put32(&mut img, b + 24, ctx.len() as u32);
            img[b + 32..b + 39].copy_from_slice(b"selinux");
            img[b + voff..b + voff + ctx.len()].copy_from_slice(ctx);
        }
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-fs-test-minimal-{n}.img"));
        let mut f = File::create(&p).unwrap();
        f.write_all(&img).unwrap();
        p
    }

    #[test]
    fn opens_superblock() {
        let p = fixture();
        let fs = Ext4::open(&p).unwrap();
        assert_eq!(fs.sb.block_size, 1024);
        assert_eq!(fs.sb.inode_size, 128);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn lists_root() {
        let p = fixture();
        let mut fs = Ext4::open(&p).unwrap();
        let names: Vec<String> = fs
            .read_dir(2)
            .unwrap()
            .into_iter()
            .map(|(_, n, _)| n)
            .collect();
        assert_eq!(names, vec!["hello.txt".to_string(), "link".to_string()]);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn reads_file_and_symlink() {
        let p = fixture();
        let mut fs = Ext4::open(&p).unwrap();
        assert_eq!(fs.read_file(12).unwrap(), b"hello");
        assert_eq!(fs.read_symlink(13).unwrap(), "hello.txt");
        assert_eq!(fs.lookup("hello.txt").unwrap(), 12);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn reads_xattr_block_context() {
        let p = fixture();
        let mut fs = Ext4::open(&p).unwrap();
        assert_eq!(
            fs.selinux_context(12).unwrap(),
            Some("u:object_r:test_file:s0".to_string())
        );
        assert_eq!(fs.selinux_context(2).unwrap(), None);
        std::fs::remove_file(p).ok();
    }

    /// Golden test against the real LineageOS GSI (2.8 GB).
    /// Runs only with GSI_TEST_IMAGE set (env-gated, never in default runs).
    /// Expected values measured via debugfs on 2026-10-07.
    #[test]
    fn golden_real_gsi() {
        let path = match std::env::var("GSI_TEST_IMAGE") {
            Ok(p) => p,
            Err(_) => {
                println!("skipped (set GSI_TEST_IMAGE)");
                return;
            }
        };
        let mut fs =
            Ext4::open(std::path::Path::new(&path)).expect("real image must open");
        assert_eq!(fs.sb.block_size, 4096);
        let names: Vec<String> = fs
            .read_dir(2)
            .unwrap()
            .into_iter()
            .map(|(_, n, _)| n)
            .collect();
        assert!(names.contains(&"system".to_string()));
        assert!(names.contains(&"init.environ.rc".to_string()));
        let init = fs.lookup("system/bin/init").unwrap();
        assert_eq!(
            fs.selinux_context(init).unwrap(),
            Some("u:object_r:init_exec:s0".to_string())
        );
        let bp = fs.lookup("system/build.prop").unwrap();
        let text = String::from_utf8(fs.read_file(bp).unwrap()).unwrap();
        assert!(text.contains("ro.build.version.release=13"));
        assert!(text.contains("ro.lineage.version=20.0-20251021-UNOFFICIAL-arm64_bgN"));
    }
}
