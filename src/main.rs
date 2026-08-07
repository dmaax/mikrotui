mod app;
mod config;
mod models;
mod ssh;
mod ui;
mod wizard;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use app::{App, InputMode, PingState};
use config::AppConfig;
use ssh::{RouterClient, SshConfig};

#[derive(Parser, Debug)]
#[command(name = "mikrotui", version = env!("CARGO_PKG_VERSION"), about = "WinBox TUI for MikroTik via SSH (Read-Only with Safe Mode)")]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,

    /// MikroTik IP or Hostname
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// SSH Port
    #[arg(short, long)]
    port: Option<u16>,

    /// SSH Username
    #[arg(short, long)]
    user: Option<String>,

    /// SSH Password
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Run in Demo Mode
    #[arg(short, long)]
    demo: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Manage stored router hosts
    Host {
        #[command(subcommand)]
        action: HostCommands,
    },
    /// Dump MikroTik resources as JSON or Text without opening TUI
    Dump {
        /// Resource: system, interfaces, ip-addresses, ip-routes, dhcp-leases, firewall, neighbors, logs
        #[arg(default_value = "system")]
        resource: String,

        /// Output format: json or text
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Force Demo Mode
        #[arg(short, long)]
        demo: bool,
    },
    /// Execute raw RouterOS CLI command via SSH
    Exec {
        /// RouterOS command (e.g. "/ip address print terse")
        command: String,

        /// Force Demo Mode
        #[arg(short, long)]
        demo: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum HostCommands {
    /// Add new host interactively
    Add,
    /// List stored hosts
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = CliArgs::parse();

    // 1. Subcommands handler
    if let Some(ref cmd) = cli.command {
        match cmd {
            Commands::Host { action } => match action {
                HostCommands::Add => {
                    wizard::run_add_host_wizard()?;
                    return Ok(());
                }
                HostCommands::List => {
                    wizard::run_list_hosts()?;
                    return Ok(());
                }
            },
            Commands::Dump { resource, format, demo } => {
                run_dump_command(&cli, resource, format, *demo).await?;
                return Ok(());
            }
            Commands::Exec { command, demo } => {
                run_exec_command(&cli, command, *demo).await?;
                return Ok(());
            }
        }
    }

    // 2. Normal TUI mode
    let ssh_config = determine_ssh_config(&cli)?;

    let ssh_config = match ssh_config {
        Some(cfg) => cfg,
        None => return Ok(()),
    };

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::new(ssh_config);
    let _ = app.load_all_data().await;

    // Main event loop
    let res = run_app(&mut terminal, &mut app).await;

    // Terminal cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error running MikroTUI: {:?}", err);
    }

    Ok(())
}

async fn run_dump_command(cli: &CliArgs, resource: &str, format: &str, force_demo: bool) -> Result<()> {
    let mut ssh_config = determine_ssh_config(cli)?.ok_or_else(|| anyhow!("No host configuration selected"))?;
    if force_demo {
        ssh_config.demo_mode = true;
    }

    let client = RouterClient::new(ssh_config);
    client.connect().await?;

    let is_json = format.to_lowercase() == "json";

    match resource.to_lowercase().as_str() {
        "system" | "resources" => {
            let res = client.fetch_system_resource().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!("{:#?}", res);
            }
        }
        "interfaces" | "interface" => {
            let list = client.fetch_interfaces().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        "ip-addresses" | "ip" | "addresses" => {
            let list = client.fetch_ip_addresses().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        "ip-routes" | "routes" => {
            let list = client.fetch_ip_routes().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        "dhcp-leases" | "dhcp" => {
            let list = client.fetch_dhcp_leases().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        "firewall" => {
            let list = client.fetch_firewall_rules().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        "neighbors" | "neighbor" => {
            let list = client.fetch_neighbors().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        "logs" => {
            let list = client.fetch_logs().await?;
            if is_json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("{:#?}", list);
            }
        }
        _ => {
            return Err(anyhow!("Unknown resource: '{}'. Valid options: system, interfaces, ip-addresses, ip-routes, dhcp-leases, firewall, neighbors, logs", resource));
        }
    }

    Ok(())
}

async fn run_exec_command(cli: &CliArgs, command: &str, force_demo: bool) -> Result<()> {
    let mut ssh_config = determine_ssh_config(cli)?.ok_or_else(|| anyhow!("No host configuration selected"))?;
    if force_demo {
        ssh_config.demo_mode = true;
    }

    let client = RouterClient::new(ssh_config);
    client.connect().await?;

    let output = client.exec_command(command).await?;
    println!("=== Raw Response from MikroTik CLI ===");
    println!("{}", output);
    println!("=====================================");

    Ok(())
}

fn determine_ssh_config(cli: &CliArgs) -> Result<Option<SshConfig>> {
    if cli.demo {
        return Ok(Some(SshConfig {
            host: "192.168.88.1".to_string(),
            port: 22,
            user: "admin".to_string(),
            pass: None,
            key_path: None,
            demo_mode: true,
        }));
    }

    if let Some(host) = &cli.host {
        return Ok(Some(SshConfig {
            host: host.clone(),
            port: cli.port.unwrap_or(22),
            user: cli.user.clone().unwrap_or_else(|| "admin".to_string()),
            pass: cli.password.clone(),
            key_path: None,
            demo_mode: false,
        }));
    }

    if AppConfig::exists() {
        let config = AppConfig::load()?;
        if !config.hosts.is_empty() {
            let selected = wizard::prompt_select_host(&config)?;
            return Ok(Some(selected));
        }
    }

    wizard::handle_first_time_run()
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<app::AppEvent>(32);

    loop {
        // Process background events from Tokio channel
        while let Ok(event) = rx.try_recv() {
            match event {
                app::AppEvent::DataLoaded {
                    system,
                    interfaces,
                    ip_addresses,
                    ip_routes,
                    dhcp_leases,
                    firewall_rules,
                    neighbors,
                    logs,
                } => {
                    app.apply_loaded_data(system, interfaces, ip_addresses, ip_routes, dhcp_leases, firewall_rules, neighbors, logs);
                }
                app::AppEvent::PingFinished(result) => {
                    app.status_message = format!("✅ Ping completed for {}: {}% loss", result.target, result.packet_loss_pct);
                    app.ping_state = PingState::Completed { result };
                }
            }
        }

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // 1. Quick Host Switcher Modal Handler (Ctrl+O)
                if app.show_host_switch_modal {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.host_switch_selected > 0 {
                                app.host_switch_selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.host_switch_selected + 1 < app.available_hosts.len() {
                                app.host_switch_selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            app.switch_host(app.host_switch_selected, tx.clone());
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.show_host_switch_modal = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // 2. Help Modal Handler (?)
                if app.show_help_modal {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                            app.show_help_modal = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // 3. Ping Modal Handler
                match &mut app.ping_state {
                    PingState::InputtingTarget { input } => {
                        match key.code {
                            KeyCode::Enter => {
                                let target = input.clone();
                                app.trigger_ping(target, tx.clone());
                            }
                            KeyCode::Esc => {
                                app.ping_state = PingState::Inactive;
                            }
                            KeyCode::Backspace => {
                                input.pop();
                            }
                            KeyCode::Char(c) => {
                                input.push(c);
                            }
                            _ => {}
                        }
                        continue;
                    }
                    PingState::Running { .. } => {
                        if key.code == KeyCode::Esc {
                            app.ping_state = PingState::Inactive;
                        }
                        continue;
                    }
                    PingState::Completed { .. } => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                                app.ping_state = PingState::Inactive;
                            }
                            _ => {}
                        }
                        continue;
                    }
                    PingState::Inactive => {}
                }

                // 4. Detail Modal Handler
                if app.show_detail_modal {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                            app.show_detail_modal = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // 5. Main Navigation Handler
                match app.input_mode {
                    InputMode::Normal => match (key.code, key.modifiers) {
                        // Quit
                        (KeyCode::Char('q'), _) => return Ok(()),
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),

                        // Detail Modal (Enter)
                        (KeyCode::Enter, _) => {
                            app.show_detail_modal = true;
                        }

                        // Quick Host Switcher Modal (Ctrl+O)
                        (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                            app.open_host_switch_prompt();
                        }

                        // Help Modal (?)
                        (KeyCode::Char('?'), _) => {
                            app.toggle_help_modal();
                        }

                        // Ping Tool (p)
                        (KeyCode::Char('p'), _) => {
                            app.open_ping_prompt();
                        }

                        // Safe Mode Toggle (Ctrl+X)
                        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                            app.toggle_safe_mode();
                        }

                        // Cycle Theme (t)
                        (KeyCode::Char('t'), _) => {
                            app.cycle_theme();
                        }

                        // Navigation between Tabs
                        (KeyCode::Tab, _) => app.next_tab(),
                        (KeyCode::BackTab, _) => app.prev_tab(),
                        (KeyCode::Right, _) | (KeyCode::Char('l'), _) => app.next_tab(),
                        (KeyCode::Left, _) | (KeyCode::Char('h'), _) => app.prev_tab(),

                        // Item selection in Table
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.select_next(),
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.select_prev(),

                        // Filter mode
                        (KeyCode::Char('/'), _) => {
                            app.input_mode = InputMode::Filtering;
                        }

                        // Reload data in background non-blocking task
                        (KeyCode::Char('r'), _) | (KeyCode::F(5), _) => {
                            app.trigger_background_reload(tx.clone());
                        }

                        _ => {}
                    },

                    InputMode::Filtering => match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.filter_query.pop();
                        }
                        KeyCode::Char(c) => {
                            app.filter_query.push(c);
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}
