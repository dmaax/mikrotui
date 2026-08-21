//! Read-only enforcement for RouterOS commands.
//!
//! The previous implementation used a denylist over whitespace-separated tokens, which
//! was trivially defeated: RouterOS v7 accepts the slash form `/ip/address/set`, whose
//! single token matches none of the denied words, and destructive commands such as
//! `/system reboot` or `/system reset-configuration` were never listed at all.
//!
//! This module inverts the logic: a command is refused unless every statement in it
//! names a known read-only action, and no statement names a known mutating action.

use std::fmt;

/// Actions that only read state from the router.
const READ_ONLY_ACTIONS: &[&str] = &[
    "print",
    "get",
    "find",
    "export",
    "monitor",
    "monitor-traffic",
    "ping",
    "traceroute",
    "resolve",
];

/// Actions that change state. Kept alongside the allowlist as defence in depth: a
/// statement naming any of these is refused even if it also names a read-only action
/// (e.g. `/ip firewall filter remove [find comment="x"]`).
const MUTATING_ACTIONS: &[&str] = &[
    "add",
    "set",
    "unset",
    "remove",
    "move",
    "enable",
    "disable",
    "edit",
    "reset",
    "reset-configuration",
    "reset-counters",
    "reboot",
    "shutdown",
    "upgrade",
    "downgrade",
    "install",
    "import",
    "restore",
    "backup",
    "run",
    "execute",
    "kill",
    "disconnect",
    "clear",
    "flush",
    "scan",
    "sniff",
    "torch",
    "setup",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardError {
    /// The statement names an action that writes to the router.
    Mutating { statement: String, action: String },
    /// The statement does not name any recognised read-only action.
    NotRecognised { statement: String },
    /// Scripting constructs (`:execute`, `:local`, ...) can hide arbitrary commands.
    Scripting { statement: String },
    Empty,
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuardError::Mutating { statement, action } => write!(
                f,
                "read-only mode: refusing '{statement}' because it performs the write action '{action}'"
            ),
            GuardError::NotRecognised { statement } => write!(
                f,
                "read-only mode: refusing '{statement}' because it names no known read-only action \
                 (allowed: {})",
                READ_ONLY_ACTIONS.join(", ")
            ),
            GuardError::Scripting { statement } => write!(
                f,
                "read-only mode: refusing '{statement}' because RouterOS script commands can wrap \
                 arbitrary write operations"
            ),
            GuardError::Empty => write!(f, "read-only mode: empty command"),
        }
    }
}

impl std::error::Error for GuardError {}

/// Split a command line into individual RouterOS statements.
///
/// RouterOS separates statements with `;` and newlines, so a guard that only inspects
/// the first statement can be bypassed with `/ip address print; /system reboot`.
fn split_statements(cmd: &str) -> Vec<&str> {
    cmd.split(['\n', '\r', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract the command *path* words of a statement.
///
/// Everything from the first `key=value` token onwards is argument data, not command
/// path, so it is excluded — this keeps values such as `address=10.0.0.1/24` from being
/// split on `/` and mistaken for path segments. The remaining tokens are split on `/`
/// so that `/ip/address/print` and `/ip address print` yield the same words.
fn path_words(statement: &str) -> Vec<String> {
    statement
        .split_whitespace()
        .take_while(|tok| !tok.contains('='))
        .flat_map(|tok| tok.split('/'))
        .map(|w| w.trim_matches(|c: char| c == '[' || c == ']').to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Check a single statement.
fn check_statement(statement: &str) -> Result<(), GuardError> {
    // `:` introduces RouterOS scripting (`:execute`, `:local`, `:do`), which can carry a
    // write command inside a string argument the path analysis below never sees.
    if statement.starts_with(':') || statement.contains(" :") {
        return Err(GuardError::Scripting {
            statement: statement.to_string(),
        });
    }

    let words = path_words(statement);

    if let Some(action) = words.iter().find(|w| MUTATING_ACTIONS.contains(&w.as_str())) {
        return Err(GuardError::Mutating {
            statement: statement.to_string(),
            action: action.clone(),
        });
    }

    if !words.iter().any(|w| READ_ONLY_ACTIONS.contains(&w.as_str())) {
        return Err(GuardError::NotRecognised {
            statement: statement.to_string(),
        });
    }

    Ok(())
}

/// Return `Ok(())` only if every statement in `cmd` is a read-only RouterOS command.
pub fn ensure_read_only(cmd: &str) -> Result<(), GuardError> {
    let statements = split_statements(cmd);
    if statements.is_empty() {
        return Err(GuardError::Empty);
    }
    for statement in statements {
        check_statement(statement)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(cmd: &str) -> bool {
        ensure_read_only(cmd).is_ok()
    }

    #[test]
    fn accepts_the_commands_the_app_actually_issues() {
        for cmd in [
            "/system resource print",
            "/system/resource/print",
            "/interface print terse without-paging",
            "/ip address print detail without-paging",
            "/ip route print terse without-paging",
            "/ip dhcp-server lease print terse without-paging",
            "/ip firewall filter print terse without-paging",
            "/ip neighbor print terse without-paging",
            "/log print follow=no",
            "/ping 8.8.8.8 count=5",
        ] {
            assert!(allowed(cmd), "should have accepted: {cmd}");
        }
    }

    #[test]
    fn rejects_space_form_writes() {
        assert!(!allowed("/ip address set 0 disabled=yes"));
        assert!(!allowed("/ip firewall filter add chain=input action=drop"));
        assert!(!allowed("/interface disable ether1"));
    }

    /// The bypass that defeated the previous denylist: RouterOS v7 slash syntax packs
    /// the action into the same token as the path.
    #[test]
    fn rejects_slash_form_writes() {
        assert!(!allowed("/ip/address/set numbers=0 disabled=yes"));
        assert!(!allowed("/ip/firewall/filter/add chain=input action=drop"));
        assert!(!allowed("/user/remove admin"));
    }

    /// Destructive commands that were never on the old denylist at all.
    #[test]
    fn rejects_destructive_system_commands() {
        assert!(!allowed("/system reboot"));
        assert!(!allowed("/system shutdown"));
        assert!(!allowed("/system reset-configuration no-defaults=yes"));
        assert!(!allowed("/system/reboot"));
        assert!(!allowed("/system package update install"));
    }

    #[test]
    fn rejects_write_hidden_after_a_read() {
        assert!(!allowed("/ip address print; /system reboot"));
        assert!(!allowed("/ip address print\n/system reboot"));
        assert!(!allowed("/ip firewall filter print; /ip/firewall/filter/remove 0"));
    }

    #[test]
    fn rejects_scripting_wrappers() {
        assert!(!allowed(":execute script=\"/system reboot\""));
        assert!(!allowed("/ip address print; :execute script=\"/system reboot\""));
        assert!(!allowed(":put [/system reboot]"));
    }

    #[test]
    fn rejects_unknown_commands_by_default() {
        assert!(!allowed("/file"));
        assert!(!allowed("/system script"));
        assert!(!allowed("whoami"));
        assert!(matches!(ensure_read_only("   "), Err(GuardError::Empty)));
    }

    /// A `/` inside an argument value must not be read as a command path separator.
    #[test]
    fn argument_values_are_not_parsed_as_paths() {
        assert!(allowed("/ip address print where address=10.0.0.1/24"));
        assert!(allowed("/ip route print where dst-address=0.0.0.0/0"));
    }

    #[test]
    fn error_message_names_the_offending_action() {
        let err = ensure_read_only("/ip/address/set numbers=0").unwrap_err();
        assert!(matches!(err, GuardError::Mutating { ref action, .. } if action == "set"));
        assert!(err.to_string().contains("set"));
    }
}
