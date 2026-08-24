//! Where MikroTUI gets a router password from, and where it may keep one.
//!
//! MikroTUI does not persist passwords by default. Earlier versions XOR-ed the password
//! against a key derived from `$USER` and a salt hardcoded in the source, then called the
//! result encryption; anyone holding the config file could recover the password. There is
//! no way around that for unattended local storage — a key the program can recompute
//! without user input, an attacker holding the file can recompute too.
//!
//! So the options are, in resolution order:
//!
//! 1. `--password-stdin` — read from a pipe, e.g. `pass show router | mikrotui …`
//! 2. `$MIKROTUI_PASSWORD`
//! 3. the OS keyring, when built with `--features keyring`
//! 4. the legacy obfuscated config field, which now warns on every use
//! 5. an interactive prompt
//!
//! `--password` on the command line is still accepted but warns: it is visible in
//! `ps aux` and lands in shell history.

use anyhow::{anyhow, Result};
use std::io::Read;

/// Identifies a credential independently of the alias the user gave the host.
pub fn account_id(user: &str, host: &str, port: u16) -> String {
    format!("{user}@{host}:{port}")
}

#[cfg_attr(not(feature = "keyring"), allow(dead_code))]
const KEYRING_SERVICE: &str = "mikrotui";

/// Read a password from stdin, stripping exactly one trailing newline.
pub fn read_from_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| anyhow!("could not read password from stdin: {e}"))?;
    let buf = buf.strip_suffix('\n').unwrap_or(&buf);
    let buf = buf.strip_suffix('\r').unwrap_or(buf);
    Ok(buf.to_string())
}

/// Read a password from the environment, if set and non-empty.
pub fn read_from_env() -> Option<String> {
    std::env::var("MIKROTUI_PASSWORD")
        .ok()
        .filter(|v| !v.is_empty())
}

/// Prompt interactively. Only valid before the TUI takes over the terminal.
pub fn prompt(account: &str) -> Result<String> {
    let pass = inquire::Password::new(&format!("SSH password for {account}:"))
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()?;
    Ok(pass)
}

// ---------------------------------------------------------------------------
// OS keyring (optional, `--features keyring`)
// ---------------------------------------------------------------------------

/// Whether this build can talk to an OS keyring at all.
pub const fn keyring_supported() -> bool {
    cfg!(feature = "keyring") && cfg!(any(target_os = "windows", target_os = "macos", unix))
}

#[cfg(feature = "keyring")]
mod backend {
    use anyhow::{anyhow, Result};
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Run keyring work on a thread of its own.
    ///
    /// The Secret Service backend bridges its async D-Bus client into the synchronous
    /// keyring API with `block_on`, and tokio panics outright when that happens on one of
    /// its worker threads: "Cannot start a runtime from within a runtime". Every entry
    /// point below is reachable from inside `#[tokio::main]` — the `host` subcommands,
    /// host selection at startup, and switching host from the TUI, which would have taken
    /// the whole session down mid-use.
    ///
    /// A thread per call is affordable because these are rare and already wait on D-Bus.
    /// Doing it here rather than at the call sites keeps the guarantee in one place: a new
    /// caller cannot forget it.
    fn off_runtime<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, on_panic: T) -> T {
        std::thread::spawn(work).join().unwrap_or(on_panic)
    }

    /// Register the platform credential store exactly once.
    ///
    /// On Linux/BSD this is the pure-Rust zbus Secret Service client, chosen over the
    /// `dbus-secret-service` binding so that `cargo install mikrotui` does not require
    /// `libdbus-1-dev` to be present.
    fn init() {
        INIT.call_once(|| {
            #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
            {
                if let Ok(store) = zbus_secret_service_keyring_store::Store::new() {
                    keyring_core::set_default_store(store);
                }
            }
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                if let Ok(store) = apple_native_keyring_store::keychain::Store::new() {
                    keyring_core::set_default_store(store);
                }
            }
            #[cfg(target_os = "windows")]
            {
                if let Ok(store) = windows_native_keyring_store::Store::new() {
                    keyring_core::set_default_store(store);
                }
            }
        });
    }

    fn entry(account: &str) -> Result<keyring_core::Entry> {
        init();
        if keyring_core::get_default_store().is_none() {
            return Err(anyhow!(
                "no OS keyring is reachable (on Linux this needs a running Secret Service, \
                 e.g. gnome-keyring or KeePassXC)"
            ));
        }
        keyring_core::Entry::new(super::KEYRING_SERVICE, account)
            .map_err(|e| anyhow!("keyring unavailable: {e}"))
    }

    pub fn get(account: &str) -> Option<String> {
        let account = account.to_string();
        off_runtime(
            move || match entry(&account) {
                Ok(e) => match e.get_password() {
                    Ok(p) if !p.is_empty() => Some(p),
                    _ => None,
                },
                Err(_) => None,
            },
            None,
        )
    }

    pub fn set(account: &str, password: &str) -> Result<()> {
        let (account, password) = (account.to_string(), password.to_string());
        off_runtime(
            move || {
                entry(&account)?
                    .set_password(&password)
                    .map_err(|e| anyhow!("could not write to the OS keyring: {e}"))
            },
            Err(anyhow!("the keyring thread panicked")),
        )
    }

    pub fn available() -> bool {
        off_runtime(
            || {
                init();
                keyring_core::get_default_store().is_some()
            },
            false,
        )
    }
}

#[cfg(not(feature = "keyring"))]
mod backend {
    use anyhow::{anyhow, Result};

    const MSG: &str = "this build has no keyring support; reinstall with \
                       `cargo install mikrotui --features keyring`";

    pub fn get(_account: &str) -> Option<String> {
        None
    }
    pub fn set(_account: &str, _password: &str) -> Result<()> {
        Err(anyhow!(MSG))
    }
    pub fn available() -> bool {
        false
    }
}

/// Fetch a stored password from the OS keyring, if any.
pub fn keyring_get(account: &str) -> Option<String> {
    backend::get(account)
}

/// Store a password in the OS keyring.
pub fn keyring_set(account: &str, password: &str) -> Result<()> {
    backend::set(account, password)
}

/// Whether a keyring is compiled in *and* reachable right now.
pub fn keyring_available() -> bool {
    backend::available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_is_stable_and_alias_independent() {
        assert_eq!(
            account_id("admin", "192.168.88.1", 22),
            "admin@192.168.88.1:22"
        );
        assert_eq!(
            account_id("admin", "192.168.88.1", 2222),
            "admin@192.168.88.1:2222"
        );
    }

    /// Every keyring entry point is reachable from inside `#[tokio::main]` — the `host`
    /// subcommands, host selection at startup, and switching host from the TUI. The
    /// Secret Service backend bridges async D-Bus into the sync keyring API with
    /// `block_on`, which panics on a tokio worker thread, so these must not run there.
    #[tokio::test]
    async fn keyring_calls_are_safe_inside_a_tokio_runtime() {
        let account = account_id("admin", "10.255.255.1", 22);

        // A panic here fails the test; the values themselves depend on the machine.
        let _ = keyring_available();
        let _ = keyring_get(&account);
        let _ = keyring_set(&account, "probe");
    }

    #[test]
    fn keyring_is_opt_in() {
        assert_eq!(keyring_supported(), cfg!(feature = "keyring"));
        if !cfg!(feature = "keyring") {
            assert!(!keyring_available());
            assert!(keyring_get("admin@10.0.0.1:22").is_none());
            assert!(keyring_set("admin@10.0.0.1:22", "x").is_err());
        }
    }
}
