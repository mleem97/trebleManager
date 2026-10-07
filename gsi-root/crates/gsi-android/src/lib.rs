//! Android property file parsing (`build.prop`, `default.prop`, ...).
//!
//! Strict `key=value` lines; `#` comments and blank lines skipped.
//! Never guesses: unknown keys stay unknown.

use std::collections::BTreeMap;

/// Parsed properties (sorted for stable output).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Props {
    map: BTreeMap<String, String>,
}

impl Props {
    /// Parse property text. Later lines overwrite earlier ones (init order).
    pub fn parse(text: &str) -> Self {
        let mut map = BTreeMap::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // import statements are not properties
            if line.starts_with("import ") {
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let val = line[eq + 1..].trim();
                if !key.is_empty() {
                    map.insert(key.to_string(), val.to_string());
                }
            }
        }
        Self { map }
    }

    /// Exact key lookup.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    /// Number of parsed keys.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when no keys parsed.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# comment\nro.build.version.sdk=33\nro.product.cpu.abi=arm64-v8a\n\nimport /system/etc/prop2\nro.secure = 1\nro.secure=0\nnovalue\n";

    #[test]
    fn parses_basics() {
        let p = Props::parse(SAMPLE);
        assert_eq!(p.get("ro.build.version.sdk"), Some("33"));
        assert_eq!(p.get("ro.product.cpu.abi"), Some("arm64-v8a"));
        assert_eq!(p.get("missing"), None);
    }

    #[test]
    fn later_wins_and_import_skipped() {
        let p = Props::parse(SAMPLE);
        assert_eq!(p.get("ro.secure"), Some("0"));
        assert_eq!(p.get("import /system/etc/prop2"), None);
        assert_eq!(p.get("novalue"), None);
    }

    #[test]
    fn empty_stays_empty() {
        assert!(Props::parse("# nothing\n\n").is_empty());
    }
}
