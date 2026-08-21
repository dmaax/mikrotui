//! SSH host key verification against `known_hosts`.
//!
//! Previously `check_server_key` returned `Ok(true)` unconditionally, so any machine on
//! the path could impersonate the router and collect the admin credentials MikroTUI is
//! about to send. Authentication now only happens after the server key has been matched
//! against `known_hosts`.
//!
//! The decision is never taken inside the async handler: an unknown or changed key makes
//! the connection fail with a [`HostKeyIssue`], which the caller surfaces (and, outside
//! the TUI, may resolve by prompting the user before retrying).

use russh::keys::ssh_key::PublicKey;
use russh::keys::HashAlg;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// What MikroTUI should do when a host is not yet in `known_hosts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyPolicy {
    /// Refuse unknown hosts. Used inside the TUI, where prompting is not possible.
    Strict,
    /// Record unknown hosts automatically (trust on first use), as
    /// `ssh -o StrictHostKeyChecking=accept-new` does.
    AcceptNew,
}

/// Why host key verification failed.
#[derive(Debug, Clone)]
pub enum HostKeyIssue {
    /// No entry for this host yet.
    Unknown {
        host: String,
        port: u16,
        key_type: String,
        fingerprint: String,
    },
    /// An entry exists but the key differs — either the router was reinstalled, or the
    /// connection is being intercepted. Never resolved automatically.
    Changed {
        host: String,
        port: u16,
        key_type: String,
        fingerprint: String,
        line: usize,
    },
}

impl HostKeyIssue {
    pub fn fingerprint(&self) -> &str {
        match self {
            HostKeyIssue::Unknown { fingerprint, .. }
            | HostKeyIssue::Changed { fingerprint, .. } => fingerprint,
        }
    }

    pub fn key_type(&self) -> &str {
        match self {
            HostKeyIssue::Unknown { key_type, .. } | HostKeyIssue::Changed { key_type, .. } => {
                key_type
            }
        }
    }
}

impl std::fmt::Display for HostKeyIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostKeyIssue::Unknown { host, port, .. } => write!(
                f,
                "the host key for {host}:{port} is not in known_hosts ({} {}). \
                 Connect once outside the TUI to review and accept it, or pass --accept-new-hostkey.",
                self.key_type(),
                self.fingerprint()
            ),
            HostKeyIssue::Changed { host, port, line, .. } => write!(
                f,
                "REMOTE HOST IDENTIFICATION HAS CHANGED for {host}:{port}. The key offered now is \
                 {} {}, which does not match the entry on line {line} of known_hosts. \
                 This may be a man-in-the-middle attack. If the router was genuinely reinstalled, \
                 remove that line and reconnect.",
                self.key_type(),
                self.fingerprint()
            ),
        }
    }
}

/// Shared slot the async handler writes its verdict into.
pub type IssueSlot = Arc<Mutex<Option<HostKeyIssue>>>;

/// Location of the `known_hosts` file MikroTUI reads and writes.
///
/// Defaults to the OpenSSH file so keys accepted by `ssh` are already trusted here.
pub fn default_known_hosts_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ssh").join("known_hosts"))
}

/// Verify `key` for `host:port`, recording an issue in `slot` when it cannot be trusted.
///
/// Returns whether the connection may proceed to authentication.
pub fn verify(
    host: &str,
    port: u16,
    key: &PublicKey,
    policy: HostKeyPolicy,
    known_hosts: &PathBuf,
    slot: &IssueSlot,
) -> bool {
    // Display renders this as "SHA256:<base64>", matching OpenSSH.
    let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
    let key_type = key.algorithm().to_string();

    let record_issue = |issue: HostKeyIssue| {
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(issue);
        }
    };

    // A missing known_hosts file simply means nothing is trusted yet.
    let known = if known_hosts.exists() {
        russh::keys::check_known_hosts_path(host, port, key, known_hosts)
    } else {
        Ok(false)
    };

    match known {
        Ok(true) => true,
        Ok(false) => match policy {
            HostKeyPolicy::AcceptNew => match learn(host, port, key, known_hosts) {
                Ok(()) => true,
                Err(_) => {
                    record_issue(HostKeyIssue::Unknown {
                        host: host.to_string(),
                        port,
                        key_type,
                        fingerprint,
                    });
                    false
                }
            },
            HostKeyPolicy::Strict => {
                record_issue(HostKeyIssue::Unknown {
                    host: host.to_string(),
                    port,
                    key_type,
                    fingerprint,
                });
                false
            }
        },
        Err(russh::keys::Error::KeyChanged { line }) => {
            record_issue(HostKeyIssue::Changed {
                host: host.to_string(),
                port,
                key_type,
                fingerprint,
                line,
            });
            false
        }
        // Any other failure (unreadable or malformed file) is treated as untrusted
        // rather than silently accepted.
        Err(_) => {
            record_issue(HostKeyIssue::Unknown {
                host: host.to_string(),
                port,
                key_type,
                fingerprint,
            });
            false
        }
    }
}

/// Append `key` to the `known_hosts` file, creating it with owner-only permissions.
pub fn learn(host: &str, port: u16, key: &PublicKey, known_hosts: &PathBuf) -> anyhow::Result<()> {
    if let Some(parent) = known_hosts.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }

    if !known_hosts.exists() {
        crate::config::create_private_file(known_hosts)?;
    }

    russh::keys::known_hosts::learn_known_hosts_path(host, port, key, known_hosts)
        .map_err(|e| anyhow::anyhow!("could not write to {}: {e}", known_hosts.display()))
}
