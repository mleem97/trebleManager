//! Pure core logic for trebleManager.
//!
//! Mirrors the reference implementations in `scripts/Treble-Toolkit.ps1`
//! (`Test-BootImageMagic`, `Test-ImageKind`, `Get-FlashVerdict`,
//! `Test-FirmwareUrl`) and `scripts/treble-toolkit.sh`
//! (`boot_magic_ver`, `image_kind`, `flash_verdict`, `valid_url`).
//! No I/O beyond reading the inspected file; no device access.

pub mod fastboot;
pub mod firmware;
pub mod images;
pub mod roms;
