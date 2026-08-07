use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub enc_password: Option<String>,
}

impl HostConfig {
    pub fn get_password(&self) -> Option<String> {
        self.enc_password.as_ref().map(|enc| decrypt_password(enc))
    }

    pub fn set_password(&mut self, raw_pass: &str) {
        if raw_pass.is_empty() {
            self.enc_password = None;
        } else {
            self.enc_password = Some(encrypt_password(raw_pass));
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub default_host: Option<String>,
    pub hosts: Vec<HostConfig>,
}

impl AppConfig {
    pub fn get_config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir().ok_or_else(|| anyhow!("Diretório de configuração do usuário não encontrado"))?;
        path.push("mikrotui");
        fs::create_dir_all(&path)?;
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
        let path = Self::get_config_path()?;
        let json_data = serde_json::to_string_pretty(self)?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        file.write_all(json_data.as_bytes())?;

        // Restringe permissões no Unix/Linux para 0o600 (rw------- octal)
        #[cfg(unix)]
        {
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
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
}

/// Chave de criptografia baseada em impressao digital do usuario/maquina
fn get_machine_key() -> Vec<u8> {
    let username = std::env::var("USER").unwrap_or_else(|_| "mikrotui_user".to_string());
    let salt = "mikrotui_secret_salt_ros_v7";
    let combined = format!("{}:{}", username, salt);
    combined.as_bytes().to_vec()
}

/// Ofuscação/Criptografia XOR rotativa de credenciais de senha no arquivo de config
pub fn encrypt_password(raw: &str) -> String {
    let key = get_machine_key();
    let encrypted_bytes: Vec<u8> = raw
        .bytes()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();
    let hex_encoded: String = encrypted_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("enc:v1:{}", hex_encoded)
}

pub fn decrypt_password(encrypted: &str) -> String {
    if !encrypted.starts_with("enc:v1:") {
        return encrypted.to_string();
    }
    let hex_str = &encrypted[7..];
    let key = get_machine_key();

    let bytes: Vec<u8> = (0..hex_str.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).ok())
        .collect();

    let decrypted_bytes: Vec<u8> = bytes
        .into_iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();

    String::from_utf8(decrypted_bytes).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_encryption_decryption() {
        let original = "MinhaSenhaMikroTik123!";
        let enc = encrypt_password(original);
        assert!(enc.starts_with("enc:v1:"));
        let dec = decrypt_password(&enc);
        assert_eq!(original, dec);
    }
}
