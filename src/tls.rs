//! STARTTLS: self-signed certificate generation and the rustls acceptor.
//!
//! One provider story for the whole tree — rustls/ring, the same stack the
//! webhooks deliver with (`reqwest` with `rustls-tls`). No native-tls, no
//! openssl, and no process-global crypto provider install: the ring provider
//! is handed explicitly to every config this module builds, so an embedding
//! process keeps its own.
//!
//! `swarmail gen-cert` mints a self-signed pair for a domain (the dev and
//! testing bootstrap); an operator-supplied PEM pair is read from files.
//! Both end in the same `rustls::ServerConfig` the SMTP listener upgrades
//! sessions with.

use rustls::pki_types::CertificateDer;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A PEM certificate/key pair the SMTP listener offers `STARTTLS` with.
#[derive(Clone)]
pub struct TlsConfig {
    /// PEM-encoded leaf certificate the listener presents.
    pub cert_pem: String,
    /// PEM-encoded private key matching `cert_pem`.
    pub key_pem: String,
}

impl fmt::Debug for TlsConfig {
    /// Redacts the private key: this type is printable in test failures and
    /// CI logs, and a key must never end up in a log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsConfig")
            .field("cert_pem", &self.cert_pem.len())
            .field("key_pem", &"<redacted>")
            .finish()
    }
}

impl TlsConfig {
    /// Read an operator's PEM pair from disk.
    pub fn from_files(cert_path: &Path, key_path: &Path) -> io::Result<Self> {
        let cert_pem = std::fs::read_to_string(cert_path)
            .map_err(|e| io::Error::other(format!("tls-cert {}: {e}", cert_path.display())))?;
        let key_pem = std::fs::read_to_string(key_path)
            .map_err(|e| io::Error::other(format!("tls-key {}: {e}", key_path.display())))?;
        Ok(Self { cert_pem, key_pem })
    }

    /// Mint a self-signed pair for `domain` — the dev/testing bootstrap.
    /// The name may be a DNS name or a literal IP; rcgen maps both to SANs.
    pub fn generate_self_signed(domain: &str) -> io::Result<Self> {
        let certified = rcgen::generate_simple_self_signed(vec![domain.to_string()])
            .map_err(|e| io::Error::other(format!("self-signed certificate for {domain}: {e}")))?;
        Ok(Self {
            cert_pem: certified.cert.pem(),
            key_pem: certified.key_pair.serialize_pem(),
        })
    }

    /// Write the pair out as `cert.pem` + `key.pem` under `dir`, creating
    /// the directory if missing. Returns the two paths.
    ///
    /// The private key lands owner-only (mode 0600 on unix): a generated
    /// key must never be group/other-readable — review finding from run
    /// starttls-self-signed-cert-genera-ef7796f0. The certificate is
    /// public and keeps the process default mode.
    pub fn write_to_dir(&self, dir: &Path) -> io::Result<(PathBuf, PathBuf)> {
        std::fs::create_dir_all(dir)?;
        let cert = dir.join("cert.pem");
        let key = dir.join("key.pem");
        std::fs::write(&cert, &self.cert_pem)?;
        write_private_key(&key, &self.key_pem)?;
        Ok((cert, key))
    }

    /// The rustls server config this pair upgrades sessions with: the PEM is
    /// parsed here, so a malformed pair is refused before anything binds.
    pub fn server_config(&self) -> io::Result<rustls::ServerConfig> {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut self.cert_pem.as_bytes()).collect::<io::Result<_>>()?;
        if certs.is_empty() {
            return Err(io::Error::other("tls-cert holds no PEM certificate"));
        }
        let key = rustls_pemfile::private_key(&mut self.key_pem.as_bytes())?
            .ok_or_else(|| io::Error::other("tls-key holds no PEM private key"))?;
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(io::Error::other)
    }
}

/// Write the private key PEM so only the file's owner can read it (mode
/// 0600 on unix). The key is created with that mode itself, so it is never
/// briefly wider; a pre-existing wider key — an older run's `key.pem` — is
/// tightened *before* the new bytes land, because `open(2)` with `O_CREAT`
/// does not re-mode an existing file; the pin afterwards makes the mode
/// exact regardless of umask. The certificate, by contrast, is public and
/// written with the plain default mode.
///
/// (Review finding from run starttls-self-signed-cert-genera-ef7796f0:
/// `write_to_dir` used `std::fs::write`, leaving the key 0664.)
fn write_private_key(path: &Path, pem: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Ok(meta) = std::fs::metadata(path)
            && meta.is_file()
        {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
            .write_all(pem.as_bytes())?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        // No POSIX mode bits on this platform; the umask/ACL default is the
        // tightest portable equivalent.
        std::fs::write(path, pem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch dir per test: pid + test name.
    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "swarmail-tls-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn generated_pair_parses_back_into_a_server_config() {
        let tls = TlsConfig::generate_self_signed("localhost").unwrap();
        assert!(tls.cert_pem.contains("BEGIN CERTIFICATE"), "{tls:?}");
        assert!(tls.key_pem.contains("PRIVATE KEY"), "{tls:?}");
        // The pair must load: this is exactly what the listener hands rustls.
        tls.server_config().unwrap();
    }

    #[test]
    fn debug_output_redacts_the_private_key() {
        let tls = TlsConfig::generate_self_signed("localhost").unwrap();
        let shown = format!("{tls:?}");
        assert!(shown.contains("TlsConfig"), "{shown}");
        assert!(!shown.contains(&tls.key_pem), "the key leaked: {shown}");
        assert!(shown.contains("<redacted>"), "{shown}");
    }

    #[test]
    fn a_pair_with_no_certificate_is_refused() {
        let mut tls = TlsConfig::generate_self_signed("localhost").unwrap();
        tls.cert_pem = "not a pem at all".into();
        let err = tls.server_config().unwrap_err();
        assert!(err.to_string().contains("no PEM certificate"), "{err}");
    }

    #[test]
    fn a_pair_with_no_key_is_refused() {
        let mut tls = TlsConfig::generate_self_signed("localhost").unwrap();
        tls.key_pem = "also not a pem".into();
        let err = tls.server_config().unwrap_err();
        assert!(err.to_string().contains("no PEM private key"), "{err}");
    }

    #[test]
    fn a_key_that_does_not_match_the_cert_is_refused() {
        let mut tls = TlsConfig::generate_self_signed("localhost").unwrap();
        tls.key_pem = TlsConfig::generate_self_signed("other.example")
            .unwrap()
            .key_pem;
        assert!(
            tls.server_config().is_err(),
            "a mismatched key must not load"
        );
    }

    #[test]
    fn from_files_reads_the_pair_and_names_the_missing_side() {
        let dir = scratch("from-files");
        let (cert, key) = TlsConfig::generate_self_signed("localhost")
            .unwrap()
            .write_to_dir(&dir)
            .unwrap();
        let pair = TlsConfig::from_files(&cert, &key).unwrap();
        assert!(pair.cert_pem.contains("BEGIN CERTIFICATE"));

        let err = TlsConfig::from_files(Path::new("/nonexistent/cert.pem"), &key).unwrap_err();
        assert!(err.to_string().contains("tls-cert"), "{err}");
        let err = TlsConfig::from_files(&cert, Path::new("/nonexistent/key.pem")).unwrap_err();
        assert!(err.to_string().contains("tls-key"), "{err}");
    }

    #[test]
    fn write_to_dir_creates_the_dir_and_refuses_a_file_in_its_place() {
        let tls = TlsConfig::generate_self_signed("localhost").unwrap();
        let dir = scratch("write");
        let (cert, key) = tls.write_to_dir(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&cert).unwrap(), tls.cert_pem);
        assert_eq!(std::fs::read_to_string(&key).unwrap(), tls.key_pem);

        // A directory path that is actually a file must fail, not clobber.
        let file = scratch("not-a-dir");
        std::fs::write(&file, "occupied").unwrap();
        assert!(tls.write_to_dir(&file).is_err());
    }

    /// Unix-only per the test-portability decision: file modes do not exist
    /// on every host this crate builds for.
    #[cfg(unix)]
    #[test]
    fn the_generated_key_is_owner_only_and_the_cert_stays_readable() {
        use std::os::unix::fs::PermissionsExt;
        let tls = TlsConfig::generate_self_signed("localhost").unwrap();
        let (cert, key) = tls.write_to_dir(&scratch("key-mode")).unwrap();

        let mode = std::fs::metadata(&key).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "the private key must never be group/other-readable, got {mode:o}"
        );
        // The certificate is public: it still reads back, byte for byte.
        assert_eq!(std::fs::read_to_string(&cert).unwrap(), tls.cert_pem);
    }

    /// A key left wide by an older run must be tightened on rewrite:
    /// `open(2)` with `O_CREAT` does not re-mode an existing file.
    #[cfg(unix)]
    #[test]
    fn a_stale_wider_key_file_is_tightened_on_rewrite() {
        use std::os::unix::fs::PermissionsExt;
        let tls = TlsConfig::generate_self_signed("localhost").unwrap();
        let dir = scratch("tighten");
        std::fs::create_dir_all(&dir).unwrap();
        let stale = dir.join("key.pem");
        std::fs::write(&stale, "an older run's key, written wide").unwrap();
        std::fs::set_permissions(&stale, std::fs::Permissions::from_mode(0o664)).unwrap();

        let (_, key) = tls.write_to_dir(&dir).unwrap();
        let mode = std::fs::metadata(&key).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "a re-run must tighten the key: {mode:o}"
        );
        assert_eq!(std::fs::read_to_string(&key).unwrap(), tls.key_pem);
    }
}
