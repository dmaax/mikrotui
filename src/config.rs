use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::secrets;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,

    /// Legacy field: a password XOR-ed against a key derived from `$USER` and a salt
    /// that ships in the binary. This is obfuscation, not encryption — anyone holding
    /// this file can recover the password. Kept only so existing configs keep working;
    /// `mikrotui host migrate` moves these into the OS keyring and clears them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obfuscated_password: Option<String>,

    /// Private key to authenticate with instead of a password.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<PathBuf>,

    /// The name this field had before it was renamed to say what it actually is.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "enc_password"
    )]
    legacy_enc_password: Option<String>,
}

impl HostConfig {
    pub fn new(name: String, host: String, port: u16, user: String) -> Self {
        Self {
            name,
            host,
            port,
            user,
            identity_file: None,
            obfuscated_password: None,
            legacy_enc_password: None,
        }
    }

    /// Identifier used for keyring lookups.
    pub fn account_id(&self) -> String {
        secrets::account_id(&self.user, &self.host, self.port)
    }

    /// The obfuscated password stored in this file, under either field name.
    pub fn stored_obfuscated(&self) -> Option<&String> {
        self.obfuscated_password
            .as_ref()
            .or(self.legacy_enc_password.as_ref())
    }

    /// Deobfuscate the file-stored password, if there is one.
    pub fn file_password(&self) -> Option<String> {
        self.stored_obfuscated().map(|enc| deobfuscate(enc))
    }

    /// Write an obfuscated password into the config file.
    ///
    /// Only reached through an explicit `--save-password`; the wizard warns first.
    pub fn set_file_password(&mut self, raw_pass: &str) {
        self.legacy_enc_password = None;
        self.obfuscated_password = if raw_pass.is_empty() {
            None
        } else {
            Some(obfuscate(raw_pass))
        };
    }

    pub fn clear_file_password(&mut self) {
        self.obfuscated_password = None;
        self.legacy_enc_password = None;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub default_host: Option<String>,
    pub hosts: Vec<HostConfig>,
}

impl AppConfig {
    pub fn get_config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow!("could not locate the user configuration directory"))?;
        path.push("mikrotui");
        fs::create_dir_all(&path)?;
        #[cfg(unix)]
        {
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o700));
        }
        path.push("config.json");
        Ok(path)
    }

    pub fn exists() -> bool {
        Self::get_config_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::get_config_path()?)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let json_data = serde_json::to_string_pretty(self)?;

        // Create with owner-only permissions rather than chmod-ing after the write:
        // the previous order left the file world-readable for the window between
        // writing the credentials and fixing the mode.
        let mut file = open_private(path)?;
        file.write_all(json_data.as_bytes())?;
        file.sync_all()?;

        // Also correct the mode of a file that already existed with wider permissions.
        #[cfg(unix)]
        {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    pub fn add_host(&mut self, host_config: HostConfig) {
        if self.hosts.iter().any(|h| h.name == host_config.name) {
            self.hosts.retain(|h| h.name != host_config.name);
        }
        if self.default_host.is_none() {
            self.default_host = Some(host_config.name.clone());
        }
        self.hosts.push(host_config);
    }

    /// Hosts still carrying an obfuscated password in the config file.
    pub fn hosts_with_file_passwords(&self) -> Vec<&HostConfig> {
        self.hosts
            .iter()
            .filter(|h| h.stored_obfuscated().is_some())
            .collect()
    }
}

/// Open `path` for writing, truncating, with mode 0600 applied at creation time.
fn open_private(path: &Path) -> Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    Ok(opts.open(path)?)
}

/// Create an empty file owned readable/writable only by the current user.
pub fn create_private_file(path: &Path) -> Result<()> {
    let mut opts = OpenOptions::new();
    opts.write(true).create(true);
    #[cfg(unix)]
    opts.mode(0o600);
    opts.open(path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Legacy obfuscation
// ---------------------------------------------------------------------------
//
// Retained only to read configs written by MikroTUI <= 0.2.2 and to keep the explicit
// `--save-password` escape hatch working. It is a repeating-key XOR whose key is
// `$USER` plus a constant compiled into the published binary, so it protects nothing
// against anyone who can read the file. Every code path that uses it warns.

const OBFUSCATION_PREFIX: &str = "enc:v1:";

fn obfuscation_key() -> Vec<u8> {
    let username = std::env::var("USER").unwrap_or_else(|_| "mikrotui_user".to_string());
    format!("{username}:mikrotui_secret_salt_ros_v7").into_bytes()
}

pub fn obfuscate(raw: &str) -> String {
    let key = obfuscation_key();
    let hex: String = raw
        .bytes()
        .enumerate()
        .map(|(i, b)| format!("{:02x}", b ^ key[i % key.len()]))
        .collect();
    format!("{OBFUSCATION_PREFIX}{hex}")
}

pub fn deobfuscate(stored: &str) -> String {
    let Some(hex_str) = stored.strip_prefix(OBFUSCATION_PREFIX) else {
        return stored.to_string();
    };
    let key = obfuscation_key();

    let bytes: Vec<u8> = (0..hex_str.len())
        .step_by(2)
        .filter_map(|i| hex_str.get(i..i + 2))
        .filter_map(|pair| u8::from_str_radix(pair, 16).ok())
        .collect();

    let decoded: Vec<u8> = bytes
        .into_iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();

    String::from_utf8(decoded).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obfuscation_round_trips() {
        let original = "MinhaSenhaMikroTik123!";
        let stored = obfuscate(original);
        assert!(stored.starts_with(OBFUSCATION_PREFIX));
        assert_eq!(original, deobfuscate(&stored));
    }

    /// Configs written by <= 0.2.2 used `enc_password`; they must keep working.
    #[test]
    fn reads_the_legacy_field_name() {
        let stored = obfuscate("hunter2");
        let json = format!(
            r#"{{"default_host":"r1","hosts":[{{"name":"r1","host":"10.0.0.1","port":22,"user":"admin","enc_password":"{stored}"}}]}}"#
        );
        let cfg: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.hosts[0].file_password().as_deref(), Some("hunter2"));
        assert_eq!(cfg.hosts_with_file_passwords().len(), 1);
    }

    #[test]
    fn new_hosts_carry_no_password_by_default() {
        let h = HostConfig::new("r1".into(), "10.0.0.1".into(), 22, "admin".into());
        assert!(h.file_password().is_none());
        assert!(h.stored_obfuscated().is_none());
        assert_eq!(h.account_id(), "admin@10.0.0.1:22");
    }

    #[test]
    fn clearing_removes_both_field_names() {
        let stored = obfuscate("hunter2");
        let json = format!(
            r#"{{"hosts":[{{"name":"r1","host":"10.0.0.1","port":22,"user":"admin","enc_password":"{stored}"}}]}}"#
        );
        let mut cfg: AppConfig = serde_json::from_str(&json).unwrap();
        cfg.hosts[0].clear_file_password();
        assert!(cfg.hosts[0].stored_obfuscated().is_none());
        let round_tripped = serde_json::to_string(&cfg).unwrap();
        assert!(!round_tripped.contains("enc_password"));
        assert!(!round_tripped.contains("obfuscated_password"));
    }

    /// The config is never observable with wider permissions, even under a permissive
    /// umask and even when overwriting a file that was already world-readable.
    #[cfg(unix)]
    #[test]
    fn saved_config_is_never_world_readable() {
        let dir = std::env::temp_dir().join(format!("mikrotui-save-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        let previous_umask = unsafe { libc_umask(0o000) };

        let mut cfg = AppConfig::default();
        let mut host = HostConfig::new("r1".into(), "10.0.0.1".into(), 22, "admin".into());
        host.set_file_password("hunter2");
        cfg.add_host(host);

        cfg.save_to(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "fresh config should be 0600, got {mode:o}");

        // A config left behind by an older version with loose permissions gets fixed.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        cfg.save_to(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "rewritten config should be 0600, got {mode:o}");

        unsafe { libc_umask(previous_umask) };
        fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    unsafe extern "C" {
        #[link_name = "umask"]
        fn libc_umask(mask: u32) -> u32;
    }

    #[cfg(unix)]
    #[test]
    fn private_files_are_created_owner_only() {
        let dir = std::env::temp_dir().join(format!("mikrotui-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secret");
        create_private_file(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
        fs::remove_dir_all(&dir).ok();
    }
}
