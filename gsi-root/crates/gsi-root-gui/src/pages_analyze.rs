//! Live-device analysis render (read-only, never executing).
//!
//! Mirrors `Screen-Analyze` / `screen_analyze` with
//! `Invoke-TTAndroidAnalysis` / `android_analysis`: OS class plus detail,
//! partition (`by-name`) summary, boot-relevant notes, and the saved
//! installed-system label. All inputs are explicit transcript strings; this
//! module never touches a device, the network, or the filesystem. The caller
//! gathers transcripts live when a device answers (bounded adb queries) and
//! passes empty strings otherwise; empty input renders an honest absence
//! note, never a guess.
//!
//! Analysis itself runs through `gsi-workflow::android_analysis` over the
//! injected transcripts. Slint side lives in `ui/app.slint` (`analyze`
//! page) with the same visual pattern as the Detect page (title, note,
//! Refresh button, read-only log view).

/// Render live-device analysis over injected transcripts.
///
/// `adb_devices` is the `adb devices` output, `getprop` the
/// `adb shell getprop` output, `byname` the `/dev/block/by-name` listing.
/// `installed_label` is the saved system display name (may be empty).
/// Empty transcripts render the honest `no device` form; otherwise the OS
/// class, profile, `recovery_ramdisk` presence, counts, and warnings are
/// shown. Nothing is executed here.
pub fn analyze_device_text(
    adb_devices: &str,
    getprop: &str,
    byname: &str,
    installed_label: &str,
) -> String {
    let f = gsi_workflow::android_analysis(adb_devices, getprop, byname);
    let mut out = String::new();
    out.push_str("Analyze (read-only, nothing executed)\n");
    if f.device_count == 0 {
        out.push_str("no device connected.\n");
    } else {
        out.push_str(&format!("devices: {}\n", f.device_count));
        if !f.first_serial.trim().is_empty() {
            out.push_str(&format!("serial: {}\n", f.first_serial.trim()));
        }
    }
    if f.model.trim().is_empty() {
        out.push_str("model: unknown\n");
    } else {
        out.push_str(&format!("model: {}\n", f.model.trim()));
    }
    if !f.pname.trim().is_empty() {
        out.push_str(&format!("product: {}\n", f.pname.trim()));
    }
    if !f.display.trim().is_empty() {
        out.push_str(&format!("display: {}\n", f.display.trim()));
    }
    if f.release.trim().is_empty() {
        out.push_str("android: unknown\n");
    } else {
        out.push_str(&format!("android: {}\n", f.release.trim()));
    }
    if f.emui.trim().is_empty() {
        out.push_str("emui: unknown\n");
    } else {
        out.push_str(&format!("emui: {}\n", f.emui.trim()));
    }
    if f.os_kind.trim().is_empty() {
        out.push_str("OS class: unknown\n");
    } else {
        out.push_str(&format!("OS class: {}\n", f.os_kind.trim()));
    }
    if !f.os_detail.trim().is_empty() {
        out.push_str(&format!("detail: {}\n", f.os_detail.trim()));
    }
    if f.profile_id.trim().is_empty() {
        out.push_str("profile: VTR-L29\n");
    } else {
        out.push_str(&format!("profile: {}\n", f.profile_id.trim()));
    }
    out.push_str(&format!("by-name entries: {}\n", f.byname_count));
    if f.has_recovery_ramdisk {
        out.push_str("recovery_ramdisk: DETECTED\n");
    } else {
        out.push_str(
            "recovery_ramdisk: NOT seen in by-name -> fastboot analysis + firmware path needed.\n",
        );
    }
    if f.warnings.is_empty() {
        out.push_str("notes: none\n");
    } else {
        for w in &f.warnings {
            let t = w.trim();
            if t.is_empty() {
                continue;
            }
            out.push_str(&format!("note: {t}\n"));
        }
    }
    let label = installed_label.trim();
    if label.is_empty() {
        out.push_str("Phone runs: unknown (pick a system)\n");
    } else {
        out.push_str(&format!("Phone runs: {label}\n"));
    }
    out.push_str("refused: live adb analysis beyond injected transcripts is Phase 8 work; render only\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADB_ONE: &str = "List of devices attached\n6PQ0217B08003446\tdevice\n";
    const GETPROP_STOCK: &str = "[ro.product.model]: [VTR-L29]\n[ro.product.name]: [VTR-L29]\n[ro.build.display.id]: [VTR-L29 9.1.0.297(C432E5R1P9)]\n[ro.build.version.release]: [9]\n[ro.build.version.emui]: [EmotionUI_9.1]\n";
    const GETPROP_GSI: &str = "[ro.product.model]: [trebledroid]\n[ro.product.name]: [trebledroid]\n[ro.build.display.id]: [trebledroid 13 arm64_bvN]\n[ro.build.version.release]: [13]\n";
    const BYNAME_OK: &str =
        "recovery_ramdisk -> /dev/block/mmcblk0p30\nboot -> /dev/block/mmcblk0p28\n";

    #[test]
    fn stock_device_renders_profile_and_recovery() {
        let t = analyze_device_text(ADB_ONE, GETPROP_STOCK, BYNAME_OK, "Stock EMUI");
        assert!(t.contains("Analyze (read-only"));
        assert!(t.contains("devices: 1"));
        assert!(t.contains("model: VTR-L29"));
        assert!(t.contains("OS class: Stock"));
        assert!(t.contains("profile: VTR-L29"));
        assert!(t.contains("recovery_ramdisk: DETECTED"));
        assert!(t.contains("by-name entries: 2"));
        assert!(t.contains("Phone runs: Stock EMUI"));
        assert!(t.contains("Phase 8"));
    }

    #[test]
    fn gsi_device_defaults_profile_with_warning() {
        let t = analyze_device_text(ADB_ONE, GETPROP_GSI, "boot -> /dev/block/x\n", "");
        assert!(t.contains("profile: VTR-L29"));
        assert!(t.contains("recovery_ramdisk: NOT seen"));
        assert!(t.contains("GSI"));
        assert!(t.contains("Phone runs: unknown"));
    }

    #[test]
    fn no_device_is_honest() {
        let t = analyze_device_text("", "", "", "");
        assert!(t.contains("no device connected."));
        assert!(t.contains("model: unknown"));
        assert!(t.contains("android: unknown"));
        assert!(t.contains("recovery_ramdisk: NOT seen"));
        assert!(t.contains("no adb device"));
        assert!(t.contains("Phone runs: unknown"));
    }
}
