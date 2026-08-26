//! Test-only helper that materializes self-signed TLS certificates on the fly.
//!
//! The transport round-trip tests need a CA certificate plus a server
//! certificate signed by that CA. Generating them per test keeps the suite
//! hermetic: no fixture files, no shared `/tmp` state between parallel tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

/// Owns a unique temporary directory holding generated PEM files and removes
/// it when dropped.
pub(crate) struct GeneratedTlsCerts {
    dir: PathBuf,
}

impl GeneratedTlsCerts {
    /// Path to the server certificate PEM signed by [`Self::ca_path`].
    pub(crate) fn cert_path(&self) -> String {
        self.path_for("server-cert.pem")
    }

    /// Path to the server private key PEM matching [`Self::cert_path`].
    pub(crate) fn key_path(&self) -> String {
        self.path_for("server-key.pem")
    }

    /// Path to the self-signed CA certificate PEM trusted by test clients.
    pub(crate) fn ca_path(&self) -> String {
        self.path_for("ca-cert.pem")
    }

    fn path_for(&self, file_name: &str) -> String {
        self.dir.join(file_name).display().to_string()
    }
}

impl Drop for GeneratedTlsCerts {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Generates a fresh CA plus a `localhost` server certificate into a process-
/// unique temp directory so concurrent tests never share TLS material.
pub(crate) fn generate() -> GeneratedTlsCerts {
    let id = UNIQUE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "xidl-jsonrpc-test-certs-{}-{id}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create test cert directory");

    let ca_key = rcgen::KeyPair::generate().expect("generate ca key pair");
    let mut ca_params =
        rcgen::CertificateParams::new(Vec::new()).expect("build ca certificate params");
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca = rcgen::CertifiedIssuer::self_signed(ca_params, ca_key).expect("self-sign ca");

    let server_key = rcgen::KeyPair::generate().expect("generate server key pair");
    let mut server_params = rcgen::CertificateParams::new(vec!["localhost".to_string()])
        .expect("build server certificate params");
    server_params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "xidl-jsonrpc-test");
    let server_cert = server_params
        .signed_by(&server_key, &ca)
        .expect("sign server certificate with ca");

    std::fs::write(dir.join("server-cert.pem"), server_cert.pem())
        .expect("write server certificate");
    std::fs::write(dir.join("server-key.pem"), server_key.serialize_pem())
        .expect("write server key");
    std::fs::write(dir.join("ca-cert.pem"), ca.pem()).expect("write ca certificate");

    GeneratedTlsCerts { dir }
}
