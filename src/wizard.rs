use anyhow::Result;
use inquire::{Confirm, CustomType, Password, Select, Text};
use crate::config::{AppConfig, HostConfig};
use crate::ssh::SshConfig;

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

    let user = Text::new("SSH Username:")
        .with_default("admin")
        .prompt()?;

    let password = Password::new("SSH Password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()?;

    println!("\n📋 === Configuration Summary ===");
    println!(" • Alias:      {}", name);
    println!(" • Host / IP:  {}:{}", host, port);
    println!(" • User:       {}", user);
    println!(" • Password:   {}", if password.is_empty() { "(no password)" } else { "•••••••• (encrypted)" });
    println!("=================================\n");

    let confirm = Confirm::new("Do you want to save this configuration permanently?")
        .with_default(true)
        .prompt()?;

    if confirm {
        let mut app_config = AppConfig::load().unwrap_or_default();
        let mut host_cfg = HostConfig {
            name: name.clone(),
            host,
            port,
            user,
            enc_password: None,
        };
        host_cfg.set_password(&password);

        app_config.add_host(host_cfg);
        app_config.save()?;

        println!("✅ Configuration successfully saved to: {}\n", AppConfig::get_config_path()?.display());
    } else {
        println!("❌ Operation canceled. Configuration was not saved.");
    }

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
        println!(
            " [{}] {} {} -> ssh {}@{}:{}",
            idx + 1,
            h.name,
            if is_default { "(Default)" } else { "" },
            h.user,
            h.host,
            h.port
        );
    }
    println!("===================================\n");
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
                return Ok(Some(SshConfig {
                    host: host.host.clone(),
                    port: host.port,
                    user: host.user.clone(),
                    pass: host.get_password(),
                    key_path: None,
                    demo_mode: false,
                }));
            }
        }
    } else if ans.starts_with("⚡") {
        let host = Text::new("Temporary IP/Host:").with_default("192.168.88.1").prompt()?;
        let port = CustomType::<u16>::new("SSH Port:").with_default(22).prompt()?;
        let user = Text::new("SSH Username:").with_default("admin").prompt()?;
        let pass = Password::new("SSH Password:")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?;

        return Ok(Some(SshConfig {
            host,
            port,
            user,
            pass: if pass.is_empty() { None } else { Some(pass) },
            key_path: None,
            demo_mode: false,
        }));
    } else {
        return Ok(Some(SshConfig {
            host: "192.168.88.1".to_string(),
            port: 22,
            user: "admin".to_string(),
            pass: None,
            key_path: None,
            demo_mode: true,
        }));
    }

    Ok(None)
}

pub fn prompt_select_host(config: &AppConfig) -> Result<SshConfig> {
    if config.hosts.len() == 1 {
        let h = &config.hosts[0];
        return Ok(SshConfig {
            host: h.host.clone(),
            port: h.port,
            user: h.user.clone(),
            pass: h.get_password(),
            key_path: None,
            demo_mode: false,
        });
    }

    let host_names: Vec<String> = config.hosts.iter().map(|h| format!("{} ({}:{})", h.name, h.host, h.port)).collect();
    let choice = Select::new("Select router to connect:", host_names).prompt()?;

    let idx = config.hosts.iter().position(|h| choice.starts_with(&h.name)).unwrap_or(0);
    let h = &config.hosts[idx];

    Ok(SshConfig {
        host: h.host.clone(),
        port: h.port,
        user: h.user.clone(),
        pass: h.get_password(),
        key_path: None,
        demo_mode: false,
    })
}
