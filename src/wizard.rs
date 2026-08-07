use anyhow::Result;
use inquire::{Confirm, CustomType, Password, Select, Text};
use crate::config::{AppConfig, HostConfig};
use crate::ssh::SshConfig;

pub fn run_add_host_wizard() -> Result<()> {
    println!("\n🌐 === MikroTUI - Adicionar Novo Roteador ===\n");

    let name = Text::new("Nome/Apelido para o Roteador (ex: Roteador-Escritorio):")
        .with_default("Roteador-MikroTik")
        .prompt()?;

    let host = Text::new("IP ou Hostname do MikroTik:")
        .with_default("192.168.88.1")
        .prompt()?;

    let port = CustomType::<u16>::new("Porta SSH:")
        .with_default(22)
        .prompt()?;

    let user = Text::new("Usuário SSH:")
        .with_default("admin")
        .prompt()?;

    let password = Password::new("Senha SSH:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()?;

    println!("\n📋 === Resumo da Configuração ===");
    println!(" • Apelido:    {}", name);
    println!(" • Host / IP:  {}:{}", host, port);
    println!(" • Usuário:    {}", user);
    println!(" • Senha:      {}", if password.is_empty() { "(sem senha)" } else { "•••••••• (criptografada)" });
    println!("=================================\n");

    let confirm = Confirm::new("Deseja salvar esta configuração no arquivo permanente?")
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

        println!("✅ Configuração salva com sucesso em: {}\n", AppConfig::get_config_path()?.display());
    } else {
        println!("❌ Operação cancelada. A configuração não foi salva.");
    }

    Ok(())
}

pub fn run_list_hosts() -> Result<()> {
    let app_config = AppConfig::load()?;

    if app_config.hosts.is_empty() {
        println!("\n⚠️  Nenhum host cadastrado em ~/.config/mikrotui/config.json");
        println!("Use 'mikrotui host add' para cadastrar um novo roteador.\n");
        return Ok(());
    }

    println!("\n📜 === Roteadores Cadastrados no MikroTUI ===");
    for (idx, h) in app_config.hosts.iter().enumerate() {
        let is_default = app_config.default_host.as_deref() == Some(&h.name);
        println!(
            " [{}] {} {} -> ssh {}@{}:{}",
            idx + 1,
            h.name,
            if is_default { "(Padrão)" } else { "" },
            h.user,
            h.host,
            h.port
        );
    }
    println!("=============================================\n");
    Ok(())
}

pub fn handle_first_time_run() -> Result<Option<SshConfig>> {
    println!("\n⚠️  Nenhum arquivo de configuração encontrado em:");
    println!("   {}", AppConfig::get_config_path()?.display());
    println!();

    let options = vec![
        "🧙 Criar um novo host de forma assistida (Salvar permanentemente em arquivo)",
        "⚡ Testar temporariamente um host (Sem salvar permanentemente)",
        "🎮 Entrar em Modo Demonstração (Demo)",
    ];

    let ans = Select::new("O que deseja fazer?", options).prompt()?;

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
        let host = Text::new("IP/Host temporário:").with_default("192.168.88.1").prompt()?;
        let port = CustomType::<u16>::new("Porta SSH:").with_default(22).prompt()?;
        let user = Text::new("Usuário SSH:").with_default("admin").prompt()?;
        let pass = Password::new("Senha SSH:")
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
    let choice = Select::new("Selecione o roteador para conectar:", host_names).prompt()?;

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
