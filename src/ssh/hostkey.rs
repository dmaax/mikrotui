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
    /// The host is in `known_hosts`, but only under other key algorithms.
    ///
    /// `check_known_hosts_path` reports this as "not known", which let a man-in-the-middle
    /// downgrade an already-trusted host back to first contact by offering a key type the
    /// file had never seen — and `--accept-new-hostkey` would then record it. Treated as a
    /// mismatch, and never resolved automatically.
    UnexpectedAlgorithm {
        host: String,
        port: u16,
        key_type: String,
        fingerprint: String,
        known_types: Vec<String>,
    },
}

impl HostKeyIssue {
    pub fn fingerprint(&self) -> &str {
        match self {
            HostKeyIssue::Unknown { fingerprint, .. }
            | HostKeyIssue::Changed { fingerprint, .. }
            | HostKeyIssue::UnexpectedAlgorithm { fingerprint, .. } => fingerprint,
        }
    }

    pub fn key_type(&self) -> &str {
        match self {
            HostKeyIssue::Unknown { key_type, .. }
            | HostKeyIssue::Changed { key_type, .. }
            | HostKeyIssue::UnexpectedAlgorithm { key_type, .. } => key_type,
        }
    }

    /// Whether accepting the key is a legitimate resolution. Only genuine first contact
    /// is: a host already recorded must never be re-learned on the strength of a key the
    /// file has not seen.
    pub fn is_first_contact(&self) -> bool {
        matches!(self, HostKeyIssue::Unknown { .. })
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
            HostKeyIssue::UnexpectedAlgorithm {
                host,
                port,
                known_types,
                ..
            } => write!(
                f,
                "{host}:{port} is already in known_hosts, but only as {}. It has now offered a \
                 {} key ({}), which is not recorded. Either the router gained a new host key, or \
                 the connection is being intercepted by something presenting a different key \
                 type. MikroTUI will not treat a host it already knows as a first contact. \
                 Confirm the fingerprint on the router with '/ip ssh print', then add or replace \
                 the entry in known_hosts yourself.",
                known_types.join(", "),
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
    let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
    let key_type = key.algorithm().to_string();

    let record_issue = |issue: HostKeyIssue| {
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(issue);
        }
    };

    // The trust decision is taken here rather than through `check_known_hosts_path`,
    // which collapses "no entry for this host" and "entries exist, but none uses this
    // key algorithm" into the same `Ok(false)`. That let a man-in-the-middle downgrade an
    // already-trusted host back to first contact by offering an unrecorded key type,
    // which `--accept-new-hostkey` would then learn. OpenSSH avoids the situation by
    // negotiating only the algorithms it has on file; we refuse the mismatch instead.
    let recorded = if known_hosts.exists() {
        match russh::keys::known_hosts::known_host_keys_path(host, port, known_hosts) {
            Ok(entries) => entries,
            // An unreadable or malformed file is not evidence of trust.
            Err(_) => {
                record_issue(HostKeyIssue::Unknown {
                    host: host.to_string(),
                    port,
                    key_type,
                    fingerprint,
                });
                return false;
            }
        }
    } else {
        Vec::new()
    };

    if recorded.iter().any(|(_, recorded)| recorded == key) {
        return true;
    }

    if let Some((line, _)) = recorded
        .iter()
        .find(|(_, recorded)| recorded.algorithm() == key.algorithm())
    {
        record_issue(HostKeyIssue::Changed {
            host: host.to_string(),
            port,
            key_type,
            fingerprint,
            line: *line,
        });
        return false;
    }

    if !recorded.is_empty() {
        let mut known_types: Vec<String> = recorded
            .iter()
            .map(|(_, k)| k.algorithm().to_string())
            .collect();
        known_types.sort();
        known_types.dedup();

        record_issue(HostKeyIssue::UnexpectedAlgorithm {
            host: host.to_string(),
            port,
            key_type,
            fingerprint,
            known_types,
        });
        return false;
    }

    // Genuinely never seen before.
    match policy {
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

#[cfg(test)]
mod tests {
    use super::*;
    use russh::keys::ssh_key::PublicKey;
    use std::sync::Mutex as StdMutex;

    // Two distinct real keys of different algorithms.
    const ED25519: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIF0bxzZBu2Q5+5lsGFR3gE75wZ8xJXKLBTPRJVfvKQ8H";
    const ED25519_OTHER: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFFyhbDOL2slDZ4TqqebVv9zW6Oak24U9gAmPP+15d8q";
    const RSA: &str = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQD08zjEcxuEqfniidJ4Axnd7vrCNVccRRjFGtzC4p37F4JVok8sY7JvKq11L3vsF4pisRWbVMoThlOGsPERFyfiODFkdll2pZkEtwf9uUjgWm+PtuSbhFBjlUvb9I9rQAuxMjewIjbisGvuLBxhyyAAtdyB2O9VyRxGKT8IX+RQKC1QTYeBOX/fmEr/cr0w+0hjq+goI0pcY522Pghh5C3Tn3GF90h17KR8uooFG5/M2BXTq+l+wcAKdG8o1AZzFjTJ1CAqMuvJjz5koNNmncqm/BFgczdE+M10Y3sJqCYLyJ2hs0FeWxNzATLadSfoVmy5++L1KjEDlWlKBoP1J5Ah";

    fn key(openssh: &str) -> PublicKey {
        PublicKey::from_openssh(openssh).expect("test key must parse")
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mikrotui-hk-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("known_hosts")
    }

    fn run(path: &PathBuf, k: &PublicKey, policy: HostKeyPolicy) -> (bool, Option<HostKeyIssue>) {
        let slot: IssueSlot = Arc::new(StdMutex::new(None));
        let ok = verify("10.9.9.9", 22, k, policy, path, &slot);
        let issue = slot.lock().unwrap().clone();
        (ok, issue)
    }

    #[test]
    fn an_unseen_host_is_first_contact() {
        let ed = key(ED25519);
        let path = scratch("unseen");
        let _ = std::fs::remove_file(&path);

        let (ok, issue) = run(&path, &ed, HostKeyPolicy::Strict);
        assert!(!ok);
        assert!(matches!(issue, Some(HostKeyIssue::Unknown { .. })));
        assert!(issue.unwrap().is_first_contact());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// The downgrade this variant exists to stop: a host already in known_hosts must not
    /// be treated as first contact just because the key algorithm is one the file has not
    /// seen — `--accept-new-hostkey` would otherwise record an impostor's key.
    #[test]
    fn a_known_host_offering_another_algorithm_is_not_first_contact() {
        let (ed, rsa) = (key(ED25519), key(RSA));
        let path = scratch("algo");
        std::fs::write(&path, format!("10.9.9.9 {RSA}\n")).unwrap();

        for policy in [HostKeyPolicy::Strict, HostKeyPolicy::AcceptNew] {
            let (ok, issue) = run(&path, &ed, policy);
            assert!(!ok, "must refuse under {policy:?}");
            let issue = issue.expect("an issue should be recorded");
            assert!(
                matches!(issue, HostKeyIssue::UnexpectedAlgorithm { .. }),
                "expected UnexpectedAlgorithm under {policy:?}, got {issue:?}"
            );
            assert!(
                !issue.is_first_contact(),
                "must never be offered for acceptance"
            );
        }

        // And nothing was appended.
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after.lines().count(), 1, "known_hosts must be untouched");
        assert!(!after.contains("ed25519"));

        // The recorded key itself still verifies.
        assert!(run(&path, &rsa, HostKeyPolicy::Strict).0);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_matching_key_is_trusted_and_a_differing_one_of_the_same_type_is_a_change() {
        let (ed, rsa) = (key(ED25519), key(RSA));
        let path = scratch("changed");
        std::fs::write(&path, format!("10.9.9.9 {ED25519}\n")).unwrap();

        assert!(run(&path, &ed, HostKeyPolicy::Strict).0);

        // Same algorithm, different key: a genuine change.
        let (ok, issue) = run(&path, &key(ED25519_OTHER), HostKeyPolicy::AcceptNew);
        assert!(!ok, "a changed key must never be accepted");
        assert!(
            matches!(issue, Some(HostKeyIssue::Changed { .. })),
            "expected Changed, got {issue:?}"
        );

        // Different algorithm: the other variant, so the two are not conflated.
        let (ok, issue) = run(&path, &rsa, HostKeyPolicy::AcceptNew);
        assert!(!ok);
        assert!(matches!(
            issue,
            Some(HostKeyIssue::UnexpectedAlgorithm { .. })
        ));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
