//! Serial masking: device serials stay masked by default.
//!
//! There is no LogBus yet (`gsi-app` is still a shell), so the TUI owns a
//! small local mask function. Pass `--show-serials` to display full serials.

/// Mask a device serial for display.
///
/// Keeps the first two and last two characters and hides the middle, so
/// `6PQ0217B08003446` becomes `6P...46`. Short or blank input yields
/// `****` (never leaks, never panics).
pub fn mask_serial(serial: &str) -> String {
    let chars: Vec<char> = serial.trim().chars().collect();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let mut out = String::new();
    let mut i = 0;
    while i < 2 {
        out.push(chars[i]);
        i += 1;
    }
    out.push_str("...");
    let mut j = chars.len() - 2;
    while j < chars.len() {
        out.push(chars[j]);
        j += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_long_serial() {
        assert_eq!(mask_serial("6PQ0217B08003446"), "6P...46");
    }

    #[test]
    fn short_and_blank_yield_stars() {
        assert_eq!(mask_serial(""), "****");
        assert_eq!(mask_serial("ab"), "****");
        assert_eq!(mask_serial("1234"), "****");
        assert_eq!(mask_serial("12345"), "12...45");
    }

    #[test]
    fn trims_whitespace_first() {
        assert_eq!(mask_serial("  ABC123  "), "AB...23");
        assert_eq!(mask_serial("  6PQ0217B08003446\n"), "6P...46");
    }
}
