//! Loading a private key for public key authentication.
//!
//! RSA keys are deliberately refused. `RUSTSEC-2023-0071` (the Marvin attack) has no fixed
//! release — the advisory lists `patched = []` — and it recovers a key by timing *private
//! key* operations. The CI ignore for it is justified on the grounds that MikroTUI holds no
//! RSA private key, and accepting one here would quietly invalidate that. Ed25519 and ECDSA
//! cover every RouterOS release that supports key authentication at all.

use anyhow::{anyhow, Context, Result};
use russh::keys::{Algorithm, PrivateKey};
use std::path::Path;

/// Asks the user for a key passphrase. `None` where there is nobody to ask.
pub type PassphrasePrompt<'a> = Option<&'a dyn Fn(&str) -> Result<String>>;

/// Validate a key and return the passphrase needed to open it, if any.
///
/// Run before the TUI takes over the terminal. It settles two things while a terminal is
/// still available: whether the key is usable at all — present, readable, and of an
/// accepted type — and the passphrase, which [`load`] later needs and cannot ask for from
/// inside the TUI.
///
/// `prompt` is `None` when there is nobody to ask, in which case an encrypted key is an
/// error rather than a hang.
pub fn prepare(path: &Path, prompt: PassphrasePrompt<'_>) -> Result<Option<String>> {
    if !path.exists() {
        return Err(anyhow!("identity file {} does not exist", path.display()));
    }
    warn_if_world_readable(path);

    match russh::keys::load_secret_key(path, None) {
        Ok(key) => {
            reject_unsupported(&key, path)?;
            Ok(None)
        }
        Err(russh::keys::Error::KeyIsEncrypted) => {
            let prompt = prompt.ok_or_else(|| {
                anyhow!(
                    "{} is protected by a passphrase and there is no terminal to ask for it. \
                     Connect once outside the TUI, or use an unencrypted key.",
                    path.display()
                )
            })?;

            let passphrase = prompt(&format!("Passphrase for {}:", path.display()))?;
            let key = russh::keys::load_secret_key(path, Some(&passphrase))
                .with_context(|| format!("could not decrypt {}", path.display()))?;
            reject_unsupported(&key, path)?;
            Ok(Some(passphrase))
        }
        Err(e) => Err(anyhow!("could not read {}: {e}", path.display())),
    }
}

/// Read the key, using the passphrase [`prepare`] resolved.
pub fn load(path: &Path, passphrase: Option<&str>) -> Result<PrivateKey> {
    let key = russh::keys::load_secret_key(path, passphrase)
        .with_context(|| format!("could not read {}", path.display()))?;
    reject_unsupported(&key, path)?;
    Ok(key)
}

/// Refuse key types MikroTUI will not authenticate with.
fn reject_unsupported(key: &PrivateKey, path: &Path) -> Result<()> {
    match key.algorithm() {
        Algorithm::Ed25519 | Algorithm::Ecdsa { .. } => Ok(()),
        Algorithm::Rsa { .. } => Err(anyhow!(
            "{} is an RSA key, which MikroTUI does not accept. The `rsa` crate carries an \
             unfixed timing sidechannel (RUSTSEC-2023-0071) affecting private key operations, \
             and upstream has no patched release. Generate an Ed25519 key instead: \
             ssh-keygen -t ed25519",
            path.display()
        )),
        other => Err(anyhow!(
            "{} uses {other}, which MikroTUI does not support. Use an Ed25519 or ECDSA key.",
            path.display()
        )),
    }
}

/// Warn when a private key is readable by anyone, as `ssh` does.
///
/// Split by platform rather than `cfg`-ing the body: an empty body leaves `path` unused,
/// which `-D warnings` turns into a build failure on Windows.
#[cfg(unix)]
fn warn_if_world_readable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(meta) = std::fs::metadata(path) {
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            eprintln!(
                "⚠️  {} is accessible by other users ({mode:o}). Restrict it with: chmod 600 {}",
                path.display(),
                path.display()
            );
        }
    }
}

/// Windows permissions do not map onto the Unix mode bits this check reads.
#[cfg(not(unix))]
fn warn_if_world_readable(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mikrotui-id-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("id")
    }

    /// Generate a key with ssh-keygen, which is what users will actually feed in.
    ///
    /// Panics rather than skipping if ssh-keygen is missing: a test that quietly passes
    /// without exercising anything is worse than one that fails loudly.
    fn keygen(path: &PathBuf, kind: &str, passphrase: &str) {
        let status = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", kind, "-N", passphrase, "-f"])
            .arg(path)
            .status()
            .expect("ssh-keygen is required to run these tests");
        assert!(status.success(), "ssh-keygen failed for a {kind} key");
    }

    #[test]
    fn an_unencrypted_ed25519_key_needs_no_passphrase() {
        let path = scratch("ed");
        keygen(&path, "ed25519", "");

        assert_eq!(prepare(&path, None).unwrap(), None);
        assert!(load(&path, None).is_ok());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The reason for refusing RSA is the unfixed advisory the CI ignore depends on.
    #[test]
    fn rsa_keys_are_refused_with_the_reason() {
        let path = scratch("rsa");
        keygen(&path, "rsa", "");

        let err = prepare(&path, None)
            .expect_err("an RSA key must be refused")
            .to_string();
        assert!(err.contains("RSA"), "got: {err}");
        assert!(err.contains("RUSTSEC-2023-0071"), "got: {err}");
        assert!(load(&path, None).is_err(), "load must refuse it too");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn ecdsa_is_accepted() {
        let path = scratch("ecdsa");
        keygen(&path, "ecdsa", "");
        assert!(prepare(&path, None).is_ok());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// An encrypted key must report the passphrase back, or `connect` cannot open it.
    #[test]
    fn an_encrypted_key_returns_the_passphrase_it_needed() {
        let path = scratch("enc");
        keygen(&path, "ed25519", "hunter2");

        // Nobody to ask: an explanation, not a hang.
        let err = prepare(&path, None)
            .expect_err("must not silently succeed")
            .to_string();
        assert!(err.contains("passphrase"), "got: {err}");

        let ask = |_: &str| -> Result<String> { Ok("hunter2".to_string()) };
        let passphrase = prepare(&path, Some(&ask)).unwrap();
        assert_eq!(passphrase.as_deref(), Some("hunter2"));

        // And that passphrase actually opens the key.
        assert!(load(&path, passphrase.as_deref()).is_ok());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_missing_key_is_reported_clearly() {
        let err = prepare(&PathBuf::from("/nonexistent/key"), None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not exist"), "got: {err}");
    }
}
