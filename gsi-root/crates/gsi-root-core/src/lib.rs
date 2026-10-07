//! GSI root pipeline orchestration: profiles, engines, plans, manifests.
//!
//! Status discipline (from the project spec): every capability is
//! `Verified`, `Documented`, `Experimental` or `Unsupported` — a theoretical
//! implementation is never presented as tested.

use gsi_image::{Container, SparseHeader};

/// Maturity of a capability or claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maturity {
    /// Proven on hardware.
    Verified,
    /// Described from authoritative docs, not yet proven here.
    Documented,
    /// Implemented to try, outcome unknown.
    Experimental,
    /// Known not to work / out of scope.
    Unsupported,
}

/// Observed GSI profile (measured, never guessed from the filename alone).
#[derive(Debug, Clone, Default)]
pub struct GsiProfile {
    /// Container of the input file.
    pub container: Option<Container>,
    /// Sparse header when the (decompressed) image is sparse.
    pub sparse: Option<SparseHeader>,
    /// Android API level when known (e.g. from filename-independent props).
    pub android_api: Option<u32>,
    /// CPU architecture when known (`arm64`, ...).
    pub arch: Option<String>,
}

/// One root engine candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootEngine {
    /// Magisk-style early init (first-stage hook). Needs POC on hardware.
    MagiskStyleEarlyInit,
    /// Second-stage init handoff.
    SecondStageInit,
    /// Legacy ramdisk path.
    LegacyRamdisk,
    /// No engine applies.
    Unsupported,
}

/// Engine support verdict with reasons (never silent).
#[derive(Debug, Clone)]
pub struct SupportResult {
    pub engine: RootEngine,
    pub supported: bool,
    pub maturity: Maturity,
    pub reasons: Vec<String>,
}

/// Decide which engine applies to a profile.
///
/// Current honesty level: without a hardware POC every early-init engine is
/// at best `Experimental`; unknown profiles are `Unsupported`.
pub fn resolve_engine(profile: &GsiProfile) -> SupportResult {
    if profile.container.is_none() {
        return SupportResult {
            engine: RootEngine::Unsupported,
            supported: false,
            maturity: Maturity::Unsupported,
            reasons: vec!["no container detected".to_string()],
        };
    }
    SupportResult {
        engine: RootEngine::MagiskStyleEarlyInit,
        supported: true,
        maturity: Maturity::Experimental,
        reasons: vec![
            "hardware POC pending: normal power-boot with active root not yet proven".to_string(),
        ],
    }
}

/// Planned patch operation (inspectable before anything is written).
#[derive(Debug, Clone)]
pub struct PatchPlan {
    pub engine: RootEngine,
    pub steps: Vec<String>,
    pub maturity: Maturity,
}

/// Build a patch plan. Refuses (`Unsupported`) when no engine applies.
pub fn plan_patch(profile: &GsiProfile) -> Result<PatchPlan, String> {
    let r = resolve_engine(profile);
    if !r.supported {
        return Err(format!("no root engine: {}", r.reasons.join("; ")));
    }
    Ok(PatchPlan {
        engine: r.engine,
        steps: vec![
            "hash input".to_string(),
            "analyze image".to_string(),
            "select profile".to_string(),
            "apply root engine (EXPERIMENTAL)".to_string(),
            "repack".to_string(),
            "verify".to_string(),
            "write manifest".to_string(),
        ],
        maturity: r.maturity,
    })
}

/// Build manifest (reproducibility record).
#[derive(Debug, Clone)]
pub struct Manifest {
    pub tool_version: String,
    pub input_sha256: String,
    pub engine: RootEngine,
    pub maturity: Maturity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_profile_is_unsupported() {
        let r = resolve_engine(&GsiProfile::default());
        assert!(!r.supported);
        assert_eq!(r.engine, RootEngine::Unsupported);
    }

    #[test]
    fn detected_container_yields_experimental_engine() {
        let p = GsiProfile {
            container: Some(Container::Gzip),
            ..Default::default()
        };
        let r = resolve_engine(&p);
        assert!(r.supported);
        assert_eq!(r.maturity, Maturity::Experimental);
        assert_eq!(r.engine, RootEngine::MagiskStyleEarlyInit);
    }

    #[test]
    fn plan_refuses_unknown() {
        assert!(plan_patch(&GsiProfile::default()).is_err());
    }

    #[test]
    fn plan_lists_inspectable_steps() {
        let p = GsiProfile {
            container: Some(Container::Raw),
            ..Default::default()
        };
        let plan = plan_patch(&p).unwrap();
        assert!(plan.steps.iter().any(|s| s.contains("verify")));
        assert!(plan.steps.iter().any(|s| s.contains("EXPERIMENTAL")));
    }
}
