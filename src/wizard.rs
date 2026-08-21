use crate::config::{AppConfig, HostConfig};
use crate::secrets;
use crate::ssh::SshConfig;
use std::path::PathBuf;

/// Expand a leading `~/`, which users type and `PathBuf` does not understand.
fn shellexpand_home(input: &str) -> String {
    match input.strip_prefix("~/") {
        Some(rest) => match dirs::home_dir() {
            Some(home) => home.join(rest).to_string_lossy().into_owned(),
            None => input.to_string(),
        },
        None => input.to_string(),
    }
}
use anyhow::Result;
use inquire::{Confirm, CustomType, Password, Select, Text};

/// Where the wizard should put the password it just collected.
enum PasswordDestination {
    Keyring,
    ConfigFile,
    DoNotStore,
}

fn choose_password_destination() -> Result<PasswordDestination> {
    let keyring_ready = secrets::keyring_available();

    let mut options = Vec::new();
    if keyring_ready {
        options.push("🔐 OS keyring (recommended)");
    }
    options.push("🚫 Do not store — ask me each time");
    options.push("⚠️  config.json (obfuscated only, recoverable by anyone who reads the file)");

    if !keyring_ready {
        if secrets::keyring_supported() {
            println!(
                "\nℹ️  No OS keyring is reachable right now (on Linux this needs a running \
                 Secret Service such as gnome-keyring or KeePassXC)."
            );
        } else {
            println!(
                "\nℹ️  This build has no keyring support. To enable it: \
                 cargo install mikrotui --features keyring"
            );
        }
    }

    let choice = Select::new("Where should the password be stored?", options).prompt()?;

    Ok(if choice.starts_with("🔐") {
        PasswordDestination::Keyring
    } else if choice.starts_with("🚫") {
        PasswordDestination::DoNotStore
    } else {
        PasswordDestination::ConfigFile
    })
}

pub fn run_add_host_wizard() -> Result<()> {
    println!("\n🌐 === MikroTUI - Add New Router Host ===\n");

    let name = Text::new("Router Name/Alias (e.g. Office-Router):")
        .with_default("MikroTik-Router")
        .prompt()?;

    let host = Text::new("MikroTik IP or Hostname:")
        .with_default("192.168.88.1")
        .prompt()?;

    let port = CustomType::<u16>::new("SSH Port:")
        .with_default(22)
        .prompt()?;

    let user = Text::new("SSH Username:").with_default("admin").prompt()?;

    println!(
        "\n💡 MikroTUI only ever issues read commands. Giving it a RouterOS account in the \
         `read` group, rather than a full admin, is what actually guarantees that."
    );

    let use_key = Confirm::new("Authenticate with an SSH private key instead of a password?")
        .with_default(false)
        .prompt()?;

    if use_key {
        let path = Text::new("Path to the private key (Ed25519 or ECDSA):")
            .with_help_message("RSA is not accepted; see the Security section of the README")
            .prompt()?;
        let path = PathBuf::from(shellexpand_home(&path));

        // Fail here rather than at the first connection: a missing, encrypted-and-
        // unopenable, or RSA key is far easier to fix while still in the wizard.
        let ask = |label: &str| -> Result<String> {
            Ok(Password::new(label)
                .with_display_mode(inquire::PasswordDisplayMode::Masked)
                .without_confirmation()
                .prompt()?)
        };
        crate::ssh::identity::prepare(&path, Some(&ask))?;

        let mut app_config = AppConfig::load().unwrap_or_default();
        let mut host_cfg = HostConfig::new(name.clone(), host, port, user);
        host_cfg.identity_file = Some(path.clone());

        println!("\n📋 === Configuration Summary ===");
        println!(" • Alias:      {}", name);
        println!(" • Host / IP:  {}:{}", host_cfg.host, host_cfg.port);
        println!(" • User:       {}", host_cfg.user);
        println!(" • Auth:       key {}", path.display());
        println!("=================================\n");

        if !Confirm::new("Do you want to save this configuration permanently?")
            .with_default(true)
            .prompt()?
        {
            println!("❌ Operation canceled. Configuration was not saved.");
            return Ok(());
        }

        app_config.add_host(host_cfg);
        app_config.save()?;
        println!(
            "✅ Configuration saved to: {}\n",
            AppConfig::get_config_path()?.display()
        );
        return Ok(());
    }

    let password = Password::new("SSH Password (leave empty to always be asked):")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()?;

    let destination = if password.is_empty() {
        PasswordDestination::DoNotStore
    } else {
        choose_password_destination()?
    };

    let storage_label = match destination {
        PasswordDestination::Keyring => "OS keyring",
        PasswordDestination::ConfigFile => "config.json (obfuscated — NOT encrypted)",
        PasswordDestination::DoNotStore => "not stored (asked at each connection)",
    };

    println!("\n📋 === Configuration Summary ===");
    println!(" • Alias:      {}", name);
    println!(" • Host / IP:  {}:{}", host, port);
    println!(" • User:       {}", user);
    println!(" • Password:   {}", storage_label);
    println!("=================================\n");

    let confirm = Confirm::new("Do you want to save this configuration permanently?")
        .with_default(true)
        .prompt()?;

    if !confirm {
        println!("❌ Operation canceled. Configuration was not saved.");
        return Ok(());
    }

    let mut app_config = AppConfig::load().unwrap_or_default();
    let mut host_cfg = HostConfig::new(name.clone(), host, port, user);

    match destination {
        PasswordDestination::Keyring => {
            secrets::keyring_set(&host_cfg.account_id(), &password)?;
            println!(
                "🔐 Password stored in the OS keyring as '{}'.",
                host_cfg.account_id()
            );
        }
        PasswordDestination::ConfigFile => {
            host_cfg.set_file_password(&password);
            println!(
                "⚠️  Password written to config.json with obfuscation only. Anyone who can \
                 read that file can recover it."
            );
        }
        PasswordDestination::DoNotStore => {}
    }

    app_config.add_host(host_cfg);
    app_config.save()?;

    println!(
        "✅ Configuration saved to: {}\n",
        AppConfig::get_config_path()?.display()
    );

    Ok(())
}

/// Move obfuscated config-file passwords into the OS keyring.
pub fn run_migrate_secrets() -> Result<()> {
    let mut app_config = AppConfig::load()?;

    let pending: Vec<String> = app_config
        .hosts_with_file_passwords()
        .iter()
        .map(|h| h.name.clone())
        .collect();

    if pending.is_empty() {
        println!("\n✅ No obfuscated passwords found in config.json — nothing to migrate.\n");
        return Ok(());
    }

    if !secrets::keyring_available() {
        if secrets::keyring_supported() {
            println!(
                "\n❌ No OS keyring is reachable. On Linux this needs a running Secret Service \
                 (gnome-keyring, KeePassXC, ...).\n"
            );
        } else {
            println!(
                "\n❌ This build has no keyring support. Reinstall with:\n   \
                 cargo install mikrotui --features keyring\n"
            );
        }
        return Ok(());
    }

    println!(
        "\n🔐 === Migrating {} password(s) to the OS keyring ===\n",
        pending.len()
    );

    let mut migrated = 0usize;
    for host in app_config.hosts.iter_mut() {
        let Some(password) = host.file_password() else {
            continue;
        };
        let account = host.account_id();
        match secrets::keyring_set(&account, &password) {
            Ok(()) => {
                host.clear_file_password();
                migrated += 1;
                println!(" ✅ {} -> keyring entry '{}'", host.name, account);
            }
            Err(e) => {
                println!(" ❌ {} kept in config.json: {e}", host.name);
            }
        }
    }

    app_config.save()?;
    println!(
        "\n{migrated} of {} migrated. config.json rewritten.\n",
        pending.len()
    );

    Ok(())
}

pub fn run_list_hosts() -> Result<()> {
    let app_config = AppConfig::load()?;

    if app_config.hosts.is_empty() {
        println!("\n⚠️  No hosts registered in ~/.config/mikrotui/config.json");
        println!("Use 'mikrotui host add' to register a new router.\n");
        return Ok(());
    }

    println!("\n📜 === Stored MikroTUI Routers ===");
    for (idx, h) in app_config.hosts.iter().enumerate() {
        let is_default = app_config.default_host.as_deref() == Some(&h.name);
        let secret = if let Some(key) = &h.identity_file {
            format!("🔑 key {}", key.display())
        } else if h.stored_obfuscated().is_some() {
            "⚠️  config.json (obfuscated)".to_string()
        } else if secrets::keyring_get(&h.account_id()).is_some() {
            "🔐 keyring".to_string()
        } else {
            "prompt".to_string()
        };
        println!(
            " [{}] {} {} -> ssh {}@{}:{}  | auth: {}",
            idx + 1,
            h.name,
            if is_default { "(Default)" } else { "" },
            h.user,
            h.host,
            h.port,
            secret
        );
    }
    println!("===================================");

    let obfuscated = app_config.hosts_with_file_passwords().len();
    if obfuscated > 0 {
        println!(
            "\n⚠️  {obfuscated} password(s) live in config.json with obfuscation only, which \
             anyone able to read the file can reverse.\n   Run `mikrotui host migrate` to move \
             them into the OS keyring."
        );
    }
    println!();
    Ok(())
}

pub fn handle_first_time_run() -> Result<Option<SshConfig>> {
    println!("\n⚠️  No configuration file found at:");
    println!("   {}", AppConfig::get_config_path()?.display());
    println!();

    let options = vec![
        "🧙 Add a new host interactively (Save permanently to config file)",
        "⚡ Connect temporarily to a host (Without saving permanently)",
        "🎮 Launch Demo Mode (No physical router required)",
    ];

    let ans = Select::new("What would you like to do?", options).prompt()?;

    if ans.starts_with("🧙") {
        run_add_host_wizard()?;
        if let Ok(config) = AppConfig::load() {
            if let Some(host) = config.hosts.first() {
                if secrets::keyring_get(&host.account_id()).is_none()
                    && host.file_password().is_some()
                {
                    eprintln!(
                        "⚠️  Using the password stored in config.json. It is only obfuscated, \
                         not encrypted — run 'mikrotui host migrate' to move it to the keyring."
                    );
                }
                let pass = secrets::keyring_get(&host.account_id())
                    .or_else(|| host.file_password())
                    .map(Ok)
                    .unwrap_or_else(|| secrets::prompt(&host.account_id()))?;
                return Ok(Some(SshConfig {
                    host: host.host.clone(),
                    port: host.port,
                    user: host.user.clone(),
                    pass: Some(pass),
                    key_path: None,
                    demo_mode: false,
                    ..SshConfig::default()
                }));
            }
        }
    } else if ans.starts_with("⚡") {
        let host = Text::new("Temporary IP/Host:")
            .with_default("192.168.88.1")
            .prompt()?;
        let port = CustomType::<u16>::new("SSH Port:")
            .with_default(22)
            .prompt()?;
        let user = Text::new("SSH Username:").with_default("admin").prompt()?;
        let pass = Password::new("SSH Password:")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .without_confirmation()
            .prompt()?;

        return Ok(Some(SshConfig {
            host,
            port,
            user,
            pass: if pass.is_empty() { None } else { Some(pass) },
            key_path: None,
            demo_mode: false,
            ..SshConfig::default()
        }));
    } else {
        return Ok(Some(SshConfig {
            demo_mode: true,
            ..SshConfig::default()
        }));
    }

    Ok(None)
}

/// Pick a stored host. Password resolution is the caller's job, so that CLI flags,
/// the environment and the keyring all take precedence in one place.
pub fn prompt_select_host(config: &AppConfig) -> Result<HostConfig> {
    if config.hosts.len() == 1 {
        return Ok(config.hosts[0].clone());
    }

    // Offer the configured default first so it is the pre-selected entry.
    let mut hosts: Vec<&HostConfig> = config.hosts.iter().collect();
    if let Some(default_name) = config.default_host.as_deref() {
        if let Some(pos) = hosts.iter().position(|h| h.name == default_name) {
            let d = hosts.remove(pos);
            hosts.insert(0, d);
        }
    }

    let labels: Vec<String> = hosts
        .iter()
        .map(|h| {
            let default_marker = if config.default_host.as_deref() == Some(&h.name) {
                " (default)"
            } else {
                ""
            };
            format!("{} ({}:{}){}", h.name, h.host, h.port, default_marker)
        })
        .collect();

    let choice = Select::new("Select router to connect:", labels.clone()).prompt()?;
    let idx = labels.iter().position(|l| *l == choice).unwrap_or(0);

    Ok(hosts[idx].clone())
}
