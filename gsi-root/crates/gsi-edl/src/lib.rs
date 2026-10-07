//! Qualcomm Snapdragon 626 / MSM8953 EDL + FRP/bootloader flows.
//!
//! Owner-repair only: every destructive flow takes an [`OwnershipProof`]
//! plus two explicit confirms and refuses before any I/O without them.
//! Packet builders are pure and golden-tested (simplified layouts, not a
//! full Sahara/Firehose stack). The only live transport here refuses:
//! the physical BQ device is used in later field sessions, never here.
//!
//! Per-variant honesty: full `frp`+`config` erase needs EDL 9008 deep
//! flash; without deep flash only fastboot/adb paths apply where the
//! bootloader already permits them, otherwise EDL is mandatory.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------- safety gates

/// Proof that FRP work is owner repair. Destructive flows accept only
/// [`OwnershipProof::OwnerConfirmed`] with a non-empty statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipProof {
    OwnerConfirmed { statement: String },
    NotConfirmed,
}

/// Refuse unless the caller passes owner repair proof (no I/O by caller).
pub fn require_ownership(proof: &OwnershipProof) -> Result<(), String> {
    match proof {
        OwnershipProof::OwnerConfirmed { statement } if !statement.trim().is_empty() => Ok(()),
        _ => Err(
            "edl: owner repair proof required (OwnershipProof::OwnerConfirmed with non-empty statement)"
                .to_string(),
        ),
    }
}

/// Aggregate two explicit destructive-step confirms (both must be true).
pub fn require_double_confirm(first: bool, second: bool) -> Result<(), String> {
    if first && second {
        Ok(())
    } else {
        Err("edl: double confirmation required (two explicit confirms)".to_string())
    }
}

/// One audit-log line. All inputs explicit; timestamp is passed in.
pub fn audit_line(operation: &str, timestamp: &str, device_id: &str, note: &str) -> String {
    format!(
        "[{}] {} device={} note={}",
        timestamp.trim(),
        operation.trim(),
        device_id.trim(),
        note.trim()
    )
}

// ---------------------------------------------------------- sahara packets

/// Simplified Sahara hello (client -> device), 48 bytes little-endian:
/// cmd=1, len=48, version=2, compatible=1, max-packet=1024, mode=0, zero
/// reserved. Golden-tested; not a full Sahara stack.
pub fn sahara_hello() -> Vec<u8> {
    let mut b = vec![0u8; 48];
    b[0] = 1;
    b[4] = 48;
    b[8] = 2;
    b[12] = 1;
    b[16] = 0x00;
    b[17] = 0x04;
    b
}

/// Simplified Sahara done: cmd=5, len=8.
pub fn sahara_done() -> Vec<u8> {
    vec![5, 0, 0, 0, 8, 0, 0, 0]
}

// ---------------------------------------------------------- firehose xml

fn check_label(label: &str) -> Result<String, String> {
    let t = label.trim();
    if t.is_empty() {
        return Err("firehose: empty partition label".to_string());
    }
    if t.bytes()
        .any(|c| c <= 0x20 || c == b'"' || c == b'<' || c == b'>' || c == b'&')
    {
        return Err(format!("firehose: bad partition label '{label}'"));
    }
    Ok(t.to_string())
}

/// Firehose hello XML.
pub fn firehose_hello() -> String {
    "<?xml version=\"1.0\" ?><data><hello verbose=\"0\" /></data>".to_string()
}

/// Firehose reset XML (power reset).
pub fn firehose_reset() -> String {
    "<?xml version=\"1.0\" ?><data><power value=\"reset\" /></data>".to_string()
}

/// Firehose erase XML for one partition label.
pub fn firehose_erase(label: &str) -> Result<String, String> {
    let l = check_label(label)?;
    Ok(format!(
        "<?xml version=\"1.0\" ?><data><erase physical_partition_number=\"0\" label=\"{l}\" /></data>"
    ))
}

/// Firehose read XML for a sector range (`count` must be > 0).
pub fn firehose_read(label: &str, start_sector: u64, count: u64) -> Result<String, String> {
    let l = check_label(label)?;
    if count == 0 {
        return Err("firehose: read count must be > 0".to_string());
    }
    Ok(format!(
        "<?xml version=\"1.0\" ?><data><read physical_partition_number=\"0\" label=\"{l}\" start_sector=\"{start_sector}\" num_partition_sectors=\"{count}\" /></data>"
    ))
}

/// Firehose write XML for a sector range (`count` must be > 0).
/// Payload transfer is out of scope; this builds the command only.
pub fn firehose_write(label: &str, start_sector: u64, count: u64) -> Result<String, String> {
    let l = check_label(label)?;
    if count == 0 {
        return Err("firehose: write count must be > 0".to_string());
    }
    Ok(format!(
        "<?xml version=\"1.0\" ?><data><write physical_partition_number=\"0\" label=\"{l}\" start_sector=\"{start_sector}\" num_partition_sectors=\"{count}\" /></data>"
    ))
}

/// Firehose patch XML (`size` must be > 0, `value` non-empty).
pub fn firehose_patch(
    label: &str,
    byte_offset: u64,
    size: u64,
    value: &str,
) -> Result<String, String> {
    let l = check_label(label)?;
    if size == 0 {
        return Err("firehose: patch size must be > 0".to_string());
    }
    if value.trim().is_empty() {
        return Err("firehose: patch value required".to_string());
    }
    let v = value.trim().to_string();
    Ok(format!(
        "<?xml version=\"1.0\" ?><data><patch physical_partition_number=\"0\" label=\"{l}\" byte_offset=\"{byte_offset}\" size_in_bytes=\"{size}\" value=\"{v}\" /></data>"
    ))
}

/// `setbootablestoragedrive` XML (`drive` 0..=7, eMMC range).
pub fn firehose_set_boot_drive(drive: u32) -> Result<String, String> {
    if drive > 7 {
        return Err("firehose: boot drive must be 0..=7".to_string());
    }
    Ok(format!(
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"{drive}\" /></data>"
    ))
}

// ---------------------------------------------------------- transport

/// Byte transport for Firehose packets. Real I/O lives outside this
/// crate; dry runs use [`StubTransport`].
pub trait FirehoseTransport {
    fn send(&mut self, packet: &[u8]) -> Result<Vec<u8>, String>;
}

/// Recording stub: stores sent packets, replays scripted replies.
/// Runs out of replies -> returns a canned ACK (documented, stable).
#[derive(Debug, Default)]
pub struct StubTransport {
    sent: Vec<Vec<u8>>,
    replies: Vec<Vec<u8>>,
    next: usize,
}

impl StubTransport {
    /// New stub with scripted replies (may be empty -> canned ACKs).
    pub fn new(replies: Vec<Vec<u8>>) -> Self {
        Self {
            sent: Vec::new(),
            replies,
            next: 0,
        }
    }

    /// Packets sent so far (refusal tests assert this stays empty).
    pub fn sent(&self) -> &[Vec<u8>] {
        &self.sent
    }

    /// Count of packets sent so far.
    pub fn sent_count(&self) -> usize {
        self.sent.len()
    }
}

impl FirehoseTransport for StubTransport {
    fn send(&mut self, packet: &[u8]) -> Result<Vec<u8>, String> {
        self.sent.push(packet.to_vec());
        if self.next < self.replies.len() {
            let r = self.replies[self.next].clone();
            self.next += 1;
            Ok(r)
        } else {
            Ok(b"<data><response value=\"ACK\" /></data>".to_vec())
        }
    }
}

/// Refusal transport: there is no live EDL device in this context.
/// The physical BQ unit is reserved for later field sessions.
#[derive(Debug, Default)]
pub struct RefusingTransport;

impl RefusingTransport {
    /// New refusal transport.
    pub fn new() -> Self {
        Self
    }
}

impl FirehoseTransport for RefusingTransport {
    fn send(&mut self, _packet: &[u8]) -> Result<Vec<u8>, String> {
        Err("no live EDL device in this context".to_string())
    }
}

// ---------------------------------------------------------- partition sets

/// FRP reset erase set on MSM8953/BQ: `frp` + `config`.
pub fn frp_erase_set() -> Vec<String> {
    vec!["frp".to_string(), "config".to_string()]
}

/// Partitions that must be backed up before ANY erase.
pub fn backup_required_set() -> Vec<String> {
    vec![
        "persist".to_string(),
        "modemst1".to_string(),
        "modemst2".to_string(),
    ]
}

/// Backup-first plan (precede every erase; enforced by plan builders).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPlan {
    pub device_id: String,
    pub partitions: Vec<String>,
    pub steps: Vec<String>,
    pub warnings: Vec<String>,
}

/// Backup plan for `persist`/`modemst1`/`modemst2` (read-only Firehose
/// reads; caller executes against its own transport).
pub fn backup_plan(device_id: &str) -> BackupPlan {
    let partitions = backup_required_set();
    let steps: Vec<String> = partitions
        .iter()
        .map(|p| format!("read {p} to host backup (verify bytes before any erase)"))
        .collect();
    BackupPlan {
        device_id: device_id.trim().to_string(),
        partitions,
        steps,
        warnings: vec![
            "Backup first: no erase runs before persist/modemst backups verify.".to_string(),
        ],
    }
}

// ---------------------------------------------------------- variant matrix

/// Supported device variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceVariant {
    /// BQ Aquaris X Pro, EU (reference device for field tests).
    AquarisXProEu,
    /// Other MSM8953 units (same SoC rules, generic notes).
    Msm8953Generic,
}

/// Flash depth: full EDL vs host-side paths only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashDepth {
    /// EDL 9008 full (Firehose programmer, deep flash).
    WithDeepFlash,
    /// fastboot/adb paths only (no deep flash).
    WithoutDeepFlash,
}

/// Honest capability note per variant and depth.
pub fn capability_note(variant: &DeviceVariant, depth: &FlashDepth) -> &'static str {
    match (variant, depth) {
        (DeviceVariant::AquarisXProEu, FlashDepth::WithDeepFlash) => {
            "EDL 9008 full: frp+config erase via matching Firehose programmer; owner proof, backup first, double confirm; 9008 entry is guidance text only."
        }
        (DeviceVariant::Msm8953Generic, FlashDepth::WithDeepFlash) => {
            "EDL 9008 full: frp+config erase via matching Firehose programmer; owner proof, backup first, double confirm."
        }
        (DeviceVariant::AquarisXProEu, FlashDepth::WithoutDeepFlash) => {
            "No deep flash: fastboot erase applies only where the bootloader already permits it (unlocked, erase allowed); on a locked unit EDL is mandatory, FRP reset is not possible from fastboot."
        }
        (DeviceVariant::Msm8953Generic, FlashDepth::WithoutDeepFlash) => {
            "No deep flash: fastboot/adb paths apply only where the SoC mode already permits erase; otherwise EDL is mandatory."
        }
    }
}

/// One plan step. `xml` holds the Firehose command for EDL steps and is
/// empty for host-side (fastboot/adb/guidance) steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdlStep {
    pub kind: String,
    pub xml: String,
    pub note: String,
}

/// Typed plan per variant with capability notes and audit lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdlPlan {
    pub variant: String,
    pub depth: String,
    pub steps: Vec<EdlStep>,
    pub warnings: Vec<String>,
    pub audit: Vec<String>,
}

fn variant_name(v: &DeviceVariant) -> &'static str {
    match v {
        DeviceVariant::AquarisXProEu => "aquaris-x-pro-eu",
        DeviceVariant::Msm8953Generic => "msm8953-generic",
    }
}

fn depth_name(d: &FlashDepth) -> &'static str {
    match d {
        FlashDepth::WithDeepFlash => "with-deep-flash",
        FlashDepth::WithoutDeepFlash => "without-deep-flash",
    }
}

/// Build the FRP plan for one variant/depth. Enforces owner proof,
/// double confirm, and backup-first before any erase step is planned.
pub fn plan_for_variant(
    variant: &DeviceVariant,
    depth: &FlashDepth,
    proof: &OwnershipProof,
    first_confirm: bool,
    second_confirm: bool,
    backup_done: bool,
) -> Result<EdlPlan, String> {
    require_ownership(proof)?;
    require_double_confirm(first_confirm, second_confirm)?;
    if !backup_done {
        return Err(
            "edl: backup-first required (persist/modemst1/modemst2) before any erase".to_string(),
        );
    }
    let mut warnings = vec![capability_note(variant, depth).to_string()];
    let mut steps = Vec::new();
    match depth {
        FlashDepth::WithDeepFlash => {
            for label in frp_erase_set() {
                let xml = firehose_erase(&label)?;
                steps.push(EdlStep {
                    kind: format!("erase:{label}"),
                    xml,
                    note: format!("EDL erase of {label} (frp reset set)"),
                });
            }
            steps.push(EdlStep {
                kind: "reset".to_string(),
                xml: firehose_reset(),
                note: "reboot after erase".to_string(),
            });
            warnings.push(
                "Needs matching MSM8953 Firehose programmer and 9008 mode; never bundled here."
                    .to_string(),
            );
        }
        FlashDepth::WithoutDeepFlash => {
            steps.push(EdlStep {
                kind: "fastboot".to_string(),
                xml: String::new(),
                note: "fastboot erase frp (only if bootloader already permits erase)".to_string(),
            });
            steps.push(EdlStep {
                kind: "fastboot".to_string(),
                xml: String::new(),
                note: "fastboot erase config (only if bootloader already permits erase)"
                    .to_string(),
            });
            warnings.push(
                "Without deep flash nothing is forced: locked units must go through EDL."
                    .to_string(),
            );
        }
    }
    Ok(EdlPlan {
        variant: variant_name(variant).to_string(),
        depth: depth_name(depth).to_string(),
        steps,
        warnings,
        audit: Vec::new(),
    })
}

// ---------------------------------------------------------- frp execution

/// Returned FRP report: every executed/planned operation is logged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrpReport {
    pub device_id: String,
    pub variant: String,
    pub depth: String,
    pub operations: Vec<String>,
    pub log: Vec<String>,
}

/// Run the FRP reset flow. Gates (owner proof, double confirm, backup)
/// are checked before any transport I/O; deep-flash steps are sent,
/// host-side steps are reported as planned (never executed here).
#[allow(clippy::too_many_arguments)] // explicit inputs by design: no hidden state, all caller-measured and tested
pub fn run_frp_reset(
    transport: &mut impl FirehoseTransport,
    variant: &DeviceVariant,
    depth: &FlashDepth,
    proof: &OwnershipProof,
    first_confirm: bool,
    second_confirm: bool,
    backup_done: bool,
    device_id: &str,
    timestamp: &str,
    operator_note: &str,
) -> Result<FrpReport, String> {
    let plan = plan_for_variant(
        variant,
        depth,
        proof,
        first_confirm,
        second_confirm,
        backup_done,
    )?;
    if device_id.trim().is_empty() {
        return Err("edl: device id required".to_string());
    }
    let mut report = FrpReport {
        device_id: device_id.trim().to_string(),
        variant: plan.variant.clone(),
        depth: plan.depth.clone(),
        operations: Vec::new(),
        log: Vec::new(),
    };
    for step in &plan.steps {
        if step.xml.is_empty() {
            report.operations.push(format!("planned:{}", step.kind));
            report.log.push(audit_line(
                &format!("plan:{}", step.kind),
                timestamp,
                &report.device_id,
                &step.note,
            ));
            continue;
        }
        transport
            .send(step.xml.as_bytes())
            .map_err(|e| format!("edl send ({}): {e}", step.kind))?;
        report.operations.push(step.kind.clone());
        report.log.push(audit_line(
            &step.kind,
            timestamp,
            &report.device_id,
            operator_note,
        ));
    }
    Ok(report)
}

// ---------------------------------------------------------- bootloader

/// Bootloader unlock plan (BQ path: manufacturer unlock code + fastboot).
/// The code itself is never logged; only its presence is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootloaderPlan {
    pub device_id: String,
    pub steps: Vec<String>,
    pub warnings: Vec<String>,
    pub audit: Vec<String>,
}

/// Build the BQ bootloader unlock plan. Requires owner proof, double
/// confirm, a non-empty manufacturer unlock code, and a device id.
/// This crate never executes fastboot; the plan is host-side guidance.
pub fn build_bootloader_unlock_plan(
    proof: &OwnershipProof,
    first_confirm: bool,
    second_confirm: bool,
    device_id: &str,
    unlock_code: &str,
    timestamp: &str,
    operator_note: &str,
) -> Result<BootloaderPlan, String> {
    require_ownership(proof)?;
    require_double_confirm(first_confirm, second_confirm)?;
    if device_id.trim().is_empty() {
        return Err("edl: device id required".to_string());
    }
    if unlock_code.trim().is_empty() {
        return Err("edl: BQ manufacturer unlock code required (never logged)".to_string());
    }
    let steps = vec![
        "Obtain unlock code via the BQ manufacturer flow (device-specific, owner only)."
            .to_string(),
        "adb reboot bootloader (verify fastboot device id first).".to_string(),
        "fastboot flashing unlock (enter the manufacturer code when prompted).".to_string(),
        "fastboot reboot (unlock wipes user data; restore from backup).".to_string(),
    ];
    let warnings = vec![
        "Unlock wipes user data; back up first.".to_string(),
        "Where the BQ vendor flow applies, only a code issued for this exact device works."
            .to_string(),
    ];
    let audit = vec![audit_line(
        "bootloader-unlock-plan",
        timestamp,
        device_id.trim(),
        operator_note,
    )];
    Ok(BootloaderPlan {
        device_id: device_id.trim().to_string(),
        steps,
        warnings,
        audit,
    })
}

// ---------------------------------------------------------- profile lookup

/// Candidate locations for a `data/compatibility/qualcomm` profile.
/// `GSI_EDL_PROFILE_DIR` wins when set; otherwise ancestors of the
/// crate manifest dir are searched (repo layout, no network).
pub fn qualcomm_profile_candidates(file_name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = std::env::var_os("GSI_EDL_PROFILE_DIR") {
        let p = PathBuf::from(dir).join(file_name);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    let mut cur: Option<PathBuf> = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for _ in 0..7 {
        let base = match cur {
            Some(b) => b,
            None => break,
        };
        let p = base.join("data/compatibility/qualcomm").join(file_name);
        if !out.contains(&p) {
            out.push(p);
        }
        cur = base.parent().map(Path::to_path_buf);
    }
    out
}

fn find_profile(file_name: &str) -> Result<PathBuf, String> {
    for p in qualcomm_profile_candidates(file_name) {
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(format!(
        "edl: profile '{file_name}' not found (set GSI_EDL_PROFILE_DIR)"
    ))
}

/// Load the BQ Aquaris X Pro ROM entries via the existing
/// `gsi-registry` loader (same schema family as the Huawei P10 VTR-L29
/// profile; unknown fields ignored).
pub fn load_bq_roms() -> Result<Vec<gsi_registry::RomEntry>, String> {
    let path = find_profile("bq-aquaris-x-pro.json")?;
    gsi_registry::load_roms(&path)
}

// ---------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> OwnershipProof {
        OwnershipProof::OwnerConfirmed {
            statement: "I own this BQ Aquaris X Pro (EU) and request owner repair.".to_string(),
        }
    }

    fn tmp_json(content: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("gsi-edl-test-{n}.json"));
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn sahara_hello_golden() {
        let h = sahara_hello();
        assert_eq!(h.len(), 48);
        assert_eq!(
            &h[0..20],
            &[1, 0, 0, 0, 48, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 4, 0, 0]
        );
        assert!(h[20..].iter().all(|b| *b == 0));
    }

    #[test]
    fn sahara_done_golden() {
        assert_eq!(sahara_done(), vec![5, 0, 0, 0, 8, 0, 0, 0]);
    }

    #[test]
    fn firehose_xml_golden() {
        assert_eq!(
            firehose_hello(),
            "<?xml version=\"1.0\" ?><data><hello verbose=\"0\" /></data>"
        );
        assert_eq!(
            firehose_reset(),
            "<?xml version=\"1.0\" ?><data><power value=\"reset\" /></data>"
        );
        assert_eq!(
            firehose_erase("frp").unwrap(),
            "<?xml version=\"1.0\" ?><data><erase physical_partition_number=\"0\" label=\"frp\" /></data>"
        );
        assert_eq!(
            firehose_erase("config").unwrap(),
            "<?xml version=\"1.0\" ?><data><erase physical_partition_number=\"0\" label=\"config\" /></data>"
        );
        assert_eq!(
            firehose_read("persist", 0, 4).unwrap(),
            "<?xml version=\"1.0\" ?><data><read physical_partition_number=\"0\" label=\"persist\" start_sector=\"0\" num_partition_sectors=\"4\" /></data>"
        );
        assert_eq!(
            firehose_write("frp", 1, 2).unwrap(),
            "<?xml version=\"1.0\" ?><data><write physical_partition_number=\"0\" label=\"frp\" start_sector=\"1\" num_partition_sectors=\"2\" /></data>"
        );
        assert_eq!(
            firehose_patch("config", 8, 1, "0").unwrap(),
            "<?xml version=\"1.0\" ?><data><patch physical_partition_number=\"0\" label=\"config\" byte_offset=\"8\" size_in_bytes=\"1\" value=\"0\" /></data>"
        );
        assert_eq!(
            firehose_set_boot_drive(1).unwrap(),
            "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"1\" /></data>"
        );
    }

    #[test]
    fn firehose_rejects_bad_input() {
        assert!(firehose_erase("").is_err());
        assert!(firehose_erase("a b").is_err());
        assert!(firehose_erase("a\"b").is_err());
        assert!(firehose_read("persist", 0, 0).is_err());
        assert!(firehose_write("persist", 0, 0).is_err());
        assert!(firehose_patch("config", 0, 0, "0").is_err());
        assert!(firehose_patch("config", 0, 1, " ").is_err());
        assert!(firehose_set_boot_drive(8).is_err());
    }

    #[test]
    fn stub_replays_and_records() {
        let mut s = StubTransport::new(vec![b"ok1".to_vec()]);
        let r1 = s.send(b"a").unwrap();
        let r2 = s.send(b"b").unwrap();
        assert_eq!(r1, b"ok1");
        assert_eq!(r2, b"<data><response value=\"ACK\" /></data>");
        assert_eq!(s.sent_count(), 2);
        assert_eq!(s.sent()[0], b"a");
    }

    #[test]
    fn refusing_transport_names_context() {
        let mut t = RefusingTransport::new();
        let e = t.send(b"hello").unwrap_err();
        assert_eq!(e, "no live EDL device in this context");
    }

    #[test]
    fn ownership_gate() {
        assert!(require_ownership(&owner()).is_ok());
        assert!(require_ownership(&OwnershipProof::NotConfirmed).is_err());
        assert!(require_ownership(&OwnershipProof::OwnerConfirmed {
            statement: "  ".to_string()
        })
        .is_err());
    }

    #[test]
    fn double_confirm_aggregates() {
        assert!(require_double_confirm(true, true).is_ok());
        assert!(require_double_confirm(true, false).is_err());
        assert!(require_double_confirm(false, true).is_err());
        assert!(require_double_confirm(false, false).is_err());
    }

    #[test]
    fn audit_line_golden() {
        assert_eq!(
            audit_line(
                "erase:frp",
                "2026-10-07 12:00:00",
                "BQ-XPRO-EU-1",
                "owner repair"
            ),
            "[2026-10-07 12:00:00] erase:frp device=BQ-XPRO-EU-1 note=owner repair"
        );
    }

    #[test]
    fn backup_plan_covers_sensitive_sets() {
        let b = backup_plan("BQ-XPRO-EU-1");
        assert_eq!(b.partitions, backup_required_set());
        assert!(b.partitions.contains(&"persist".to_string()));
        assert!(b.partitions.contains(&"modemst1".to_string()));
        assert!(b.partitions.contains(&"modemst2".to_string()));
        assert_eq!(b.steps.len(), 3);
        assert!(!b.warnings.is_empty());
    }

    #[test]
    fn frp_plan_enforces_backup_first() {
        let e = plan_for_variant(
            &DeviceVariant::AquarisXProEu,
            &FlashDepth::WithDeepFlash,
            &owner(),
            true,
            true,
            false,
        )
        .unwrap_err();
        assert!(e.contains("backup-first"));
    }

    #[test]
    fn frp_refuses_before_any_io_without_proof() {
        let mut s = StubTransport::new(vec![]);
        let r = run_frp_reset(
            &mut s,
            &DeviceVariant::AquarisXProEu,
            &FlashDepth::WithDeepFlash,
            &OwnershipProof::NotConfirmed,
            true,
            true,
            true,
            "BQ-XPRO-EU-1",
            "2026-10-07 12:00:00",
            "owner repair",
        );
        assert!(r.is_err());
        assert_eq!(s.sent_count(), 0);
    }

    #[test]
    fn frp_refuses_before_any_io_without_confirms_or_backup() {
        let mut s = StubTransport::new(vec![]);
        assert!(run_frp_reset(
            &mut s,
            &DeviceVariant::AquarisXProEu,
            &FlashDepth::WithDeepFlash,
            &owner(),
            true,
            false,
            true,
            "BQ-XPRO-EU-1",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .is_err());
        assert!(run_frp_reset(
            &mut s,
            &DeviceVariant::AquarisXProEu,
            &FlashDepth::WithDeepFlash,
            &owner(),
            true,
            true,
            false,
            "BQ-XPRO-EU-1",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .is_err());
        assert_eq!(s.sent_count(), 0);
    }

    #[test]
    fn frp_deep_flash_dry_run_logs_everything() {
        let mut s = StubTransport::new(vec![]);
        let r = run_frp_reset(
            &mut s,
            &DeviceVariant::AquarisXProEu,
            &FlashDepth::WithDeepFlash,
            &owner(),
            true,
            true,
            true,
            "BQ-XPRO-EU-1",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .unwrap();
        assert_eq!(r.operations, vec!["erase:frp", "erase:config", "reset"]);
        assert_eq!(s.sent_count(), 3);
        assert_eq!(r.log.len(), 3);
        assert!(r.log.iter().all(|l| l.contains("BQ-XPRO-EU-1")));
        assert!(r.log.iter().all(|l| l.contains("2026-10-07 12:00:00")));
        let sent = String::from_utf8_lossy(&s.sent()[0]).to_string();
        assert!(sent.contains("label=\"frp\""));
    }

    #[test]
    fn frp_without_deep_flash_plans_only_no_edl_io() {
        let mut s = StubTransport::new(vec![]);
        let r = run_frp_reset(
            &mut s,
            &DeviceVariant::AquarisXProEu,
            &FlashDepth::WithoutDeepFlash,
            &owner(),
            true,
            true,
            true,
            "BQ-XPRO-EU-1",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .unwrap();
        assert_eq!(s.sent_count(), 0);
        assert!(r.operations.iter().all(|o| o.starts_with("planned:")));
        assert!(!r.log.is_empty());
    }

    #[test]
    fn matrix_notes_are_honest() {
        assert!(
            capability_note(&DeviceVariant::AquarisXProEu, &FlashDepth::WithDeepFlash)
                .contains("9008")
        );
        assert!(
            capability_note(&DeviceVariant::AquarisXProEu, &FlashDepth::WithoutDeepFlash)
                .contains("EDL is mandatory")
        );
        let p = plan_for_variant(
            &DeviceVariant::Msm8953Generic,
            &FlashDepth::WithoutDeepFlash,
            &owner(),
            true,
            true,
            true,
        )
        .unwrap();
        assert!(p.warnings.iter().any(|w| w.contains("locked")));
    }

    #[test]
    fn bootloader_plan_gates_and_hides_code() {
        assert!(build_bootloader_unlock_plan(
            &OwnershipProof::NotConfirmed,
            true,
            true,
            "BQ-XPRO-EU-1",
            "CODE-1",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .is_err());
        assert!(build_bootloader_unlock_plan(
            &owner(),
            true,
            true,
            "BQ-XPRO-EU-1",
            " ",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .is_err());
        let p = build_bootloader_unlock_plan(
            &owner(),
            true,
            true,
            "BQ-XPRO-EU-1",
            "SECRET-CODE",
            "2026-10-07 12:00:00",
            "owner repair",
        )
        .unwrap();
        assert!(p
            .steps
            .iter()
            .any(|s| s.contains("fastboot flashing unlock")));
        assert!(p.steps.iter().any(|s| s.contains("BQ manufacturer flow")));
        let blob = format!("{:?}", p);
        assert!(!blob.contains("SECRET-CODE"));
        assert!(p.audit.iter().all(|a| a.contains("BQ-XPRO-EU-1")));
    }

    #[test]
    fn fixture_profile_loads_via_registry() {
        let p = tmp_json(
            r#"{"roms": [{"name": "LineageOS", "version": 20, "android": 13, "status": "working", "gsi": "arm64_bgN"}]}"#,
        );
        let entries = gsi_registry::load_roms(&p).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(gsi_registry::entry_label(&entries[0]), "LineageOS 20");
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn bq_profile_loads_via_registry() {
        let mut found: Option<PathBuf> = None;
        for c in qualcomm_profile_candidates("bq-aquaris-x-pro.json") {
            if c.is_file() {
                found = Some(c);
                break;
            }
        }
        let path = match found {
            Some(p) => p,
            None => return,
        };
        let entries = gsi_registry::load_roms(&path).unwrap();
        assert!(!entries.is_empty());
        assert!(entries
            .iter()
            .any(|e| e.status == "broken" && e.markers.iter().any(|m| m.contains("unverified"))));
        let text = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(gsi_registry::parse_tools_block(&v).platform_tools.is_some());
    }

    #[test]
    fn bq_yaml_mirrors_json() {
        let mut yml: Option<PathBuf> = None;
        let mut jsn: Option<PathBuf> = None;
        for c in qualcomm_profile_candidates("bq-aquaris-x-pro.yaml") {
            if c.is_file() {
                yml = Some(c);
                break;
            }
        }
        for c in qualcomm_profile_candidates("bq-aquaris-x-pro.json") {
            if c.is_file() {
                jsn = Some(c);
                break;
            }
        }
        let (yp, jp) = match (yml, jsn) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };
        let y = std::fs::read_to_string(&yp).unwrap();
        let j = std::fs::read_to_string(&jp).unwrap();
        for key in [
            "bq-aquaris-x-pro",
            "MSM8953",
            "roms:",
            "tools:",
            "edl:",
            "9008",
            "unverified",
        ] {
            assert!(y.contains(key), "yaml missing {key}");
        }
        assert!(j.contains("bq-aquaris-x-pro"));
    }
}
