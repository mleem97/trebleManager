//! URL validation for firmware/ROM downloads.
//!
//! Allows http(s), max 2048 chars, archive types; strips a trailing
//! `/download` first (SourceForge direct links). Mirrors `Test-FirmwareUrl`.

const MAX_LEN: usize = 2048;
const ALLOWED: &[&str] = &[
    ".zip", ".7z", ".tar", ".gz", ".tgz", ".xz", ".app", ".rar", ".img",
];

/// Why a URL was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    Empty,
    TooLong,
    NotHttp,
    Invalid,
    NoHost,
    Credentials,
    BadType,
}

/// Checked download URL (host part for display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedUrl {
    pub host: String,
}

/// Validate a download URL.
pub fn check_url(url: &str) -> Result<CheckedUrl, UrlError> {
    let u = url.trim();
    if u.is_empty() {
        return Err(UrlError::Empty);
    }
    if u.len() > MAX_LEN {
        return Err(UrlError::TooLong);
    }
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err(UrlError::NotHttp);
    }
    let after_scheme = u.splitn(2, "://").nth(1).unwrap_or("");
    if after_scheme.is_empty() {
        return Err(UrlError::Invalid);
    }
    let host = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        return Err(UrlError::NoHost);
    }
    if let Some(at) = host.find('@') {
        if host[..at].contains(':') {
            return Err(UrlError::Credentials);
        }
    }
    let mut low = after_scheme.to_lowercase();
    if let Some(q) = low.find('?') {
        low.truncate(q);
    }
    if let Some(stripped) = low.strip_suffix("/download") {
        low = stripped.to_string();
    }
    if !ALLOWED.iter().any(|e| low.ends_with(e)) {
        return Err(UrlError::BadType);
    }
    Ok(CheckedUrl {
        host: host.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_zip_ok() {
        assert!(check_url("https://androidhost.ru/x/VTR-L29-9.1.0.297.zip").is_ok());
    }

    #[test]
    fn sourceforge_download_ok() {
        let r = check_url("https://sourceforge.net/projects/andyyan-gsi/files/lineage-20-td/lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz/download");
        assert_eq!(
            r.unwrap().host,
            "sourceforge.net".to_string()
        );
    }

    #[test]
    fn ftp_rejected() {
        assert_eq!(check_url("ftp://example.com/fw.zip"), Err(UrlError::NotHttp));
    }

    #[test]
    fn exe_rejected() {
        assert_eq!(check_url("https://example.com/fw.exe"), Err(UrlError::BadType));
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(check_url(""), Err(UrlError::Empty));
    }

    #[test]
    fn credentials_rejected() {
        assert_eq!(
            check_url("https://user:pass@example.com/fw.zip"),
            Err(UrlError::Credentials)
        );
    }
}
