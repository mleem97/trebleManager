//! ROM labels and Android-version grouping (registry presentation logic).
//!
//! Labels carry variant/build so distinct builds never collapse into one
//! entry (e.g. `LineageOS 20` vs `LineageOS 20 UNOFFICIAL (20251021)`).

/// Display label for an installed-ROM id.
pub fn rom_label(id: &str) -> String {
    if id.is_empty() {
        return "?".to_string();
    }
    if id == "stock" {
        return "Stock EMUI".to_string();
    }
    if id == "other" {
        return "Other custom ROM".to_string();
    }
    if let Some(rest) = id.strip_prefix("rom:") {
        return rest.to_string();
    }
    id.to_string()
}

/// Suggestion from an OS class: stock EMUI is detected, a custom ROM is
/// never guessed (the user states it).
pub fn rom_suggest(os_kind: &str) -> &'static str {
    if os_kind.contains("Stock") || os_kind.contains("EMUI") {
        "stock"
    } else {
        ""
    }
}

/// Registry ROM entry label: name + version/android/build + variant + build.
/// Mirrors `Get-RomEntryLabel` / `rom_options` label building.
pub fn rom_entry_label(
    name: &str,
    version: &str,
    android: &str,
    build: &str,
    variant: &str,
) -> String {
    let mut label = name.trim().to_string();
    if label.is_empty() {
        return String::new();
    }
    let mut ver = version.trim();
    if ver.is_empty() {
        ver = android.trim();
    }
    if ver.is_empty() {
        ver = build.trim();
    }
    if !ver.is_empty() {
        label.push(' ');
        label.push_str(ver);
    }
    let variant = variant.trim();
    if !variant.is_empty() && !label.contains(variant) {
        label.push(' ');
        label.push_str(variant);
    }
    let build = build.trim();
    if !build.is_empty() && !label.contains(build) {
        label.push_str(" (");
        label.push_str(build);
        label.push(')');
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(rom_label("stock"), "Stock EMUI");
        assert_eq!(rom_label("other"), "Other custom ROM");
        assert_eq!(rom_label("rom:LineageOS 20"), "LineageOS 20");
        assert_eq!(rom_label(""), "?");
    }

    #[test]
    fn suggest_never_guesses_custom() {
        assert_eq!(rom_suggest("Stock EMUI 9.1"), "stock");
        assert_eq!(rom_suggest("TrebleDroid GSI"), "");
        assert_eq!(rom_suggest(""), "");
    }

    #[test]
    fn entry_labels_stay_distinct() {
        assert_eq!(
            rom_entry_label("LineageOS", "20", "", "", ""),
            "LineageOS 20"
        );
        assert_eq!(
            rom_entry_label("LineageOS", "20", "", "20251021", "UNOFFICIAL"),
            "LineageOS 20 UNOFFICIAL (20251021)"
        );
        assert_eq!(rom_entry_label("SuperiorOS", "", "13", "20240401", ""), "SuperiorOS 13 (20240401)");
    }
}
