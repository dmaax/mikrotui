mod app;
mod config;
mod models;
mod secrets;
mod ssh;
mod ui;
mod wizard;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use app::{App, InputMode, PingState};
use config::AppConfig;
use ssh::{HostKeyPolicy, RouterClient, SshConfig};

#[derive(Parser, Debug)]
#[command(name = "mikrotui", version = env!("CARGO_PKG_VERSION"), about = "WinBox-style TUI for MikroTik RouterOS over SSH (read-only)")]
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

    /// SSH password (visible in `ps` and shell history — prefer --password-stdin)
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Read the SSH password from stdin, e.g. `pass show router | mikrotui --password-stdin`
    #[arg(long, conflicts_with = "password")]
    password_stdin: bool,

    /// Record an unknown host key on first connection instead of refusing it
    #[arg(long)]
    accept_new_hostkey: bool,

    /// known_hosts file to verify the router against [default: ~/.ssh/known_hosts]
    #[arg(long, value_name = "PATH")]
    known_hosts: Option<PathBuf>,

    /// Authenticate with an SSH private key instead of a password (Ed25519 or ECDSA)
    #[arg(short = 'i', long, value_name = "PATH")]
    identity: Option<PathBuf>,

    /// Refresh the visible tab automatically every N seconds (omit to refresh only on 'r')
    #[arg(long, value_name = "SECONDS")]
    refresh: Option<u64>,

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
    /// Move passwords obfuscated in config.json into the OS keyring
    Migrate,
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
                HostCommands::Migrate => {
                    wizard::run_migrate_secrets()?;
                    return Ok(());
                }
            },
            Commands::Dump {
                resource,
                format,
                demo,
            } => {
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
    let ssh_config = match determine_ssh_config(&cli, true)? {
        Some(cfg) => cfg,
        None => return Ok(()),
    };

    // Connect *before* entering raw mode. Host key acceptance and password prompts need
    // a usable terminal, and a failure here should print a readable error rather than
    // leaving the user staring at an empty TUI.
    let client = connect_interactively(ssh_config).await?;
    let verified = client.host_key_verified().await;

    // Terminal initialization
    install_panic_hook();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::with_client(client, verified);
    app.refresh_interval = cli.refresh.filter(|s| *s > 0).map(Duration::from_secs);
    let _ = app.load_initial_data().await;

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

/// Restore the terminal before a panic prints.
///
/// A panic inside the draw loop unwinds past the cleanup at the end of `main`, leaving the
/// terminal in raw mode on the alternate screen: no echo, no working newline, and the
/// panic message itself invisible. The hook puts the terminal back first, then defers to
/// the default handler so the message and backtrace still appear.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
}

/// Connect, resolving host key and password questions interactively.
///
/// An unknown host key is shown with its fingerprint and accepted only if the user says
/// so; a *changed* key is always fatal and is never offered for acceptance.
async fn connect_interactively(mut ssh_config: SshConfig) -> Result<RouterClient> {
    if ssh_config.demo_mode {
        let client = RouterClient::new(ssh_config);
        client.connect().await?;
        return Ok(client);
    }

    loop {
        let client = RouterClient::new(ssh_config.clone());
        match client.connect().await {
            Ok(()) => return Ok(client),
            Err(err) => {
                // Ask about the host key only when that is genuinely what failed.
                let Some(issue) = client.last_host_key_issue().await else {
                    return Err(err);
                };

                // Only a host that has never been seen may be resolved by accepting
                // the key. Anything else means known_hosts already has an opinion about
                // this router, and re-learning would be exactly the downgrade an attacker
                // wants.
                if !issue.is_first_contact() {
                    return Err(anyhow!("{issue}"));
                }

                println!(
                    "\n🔑 The authenticity of host '{}:{}' cannot be established.",
                    ssh_config.host, ssh_config.port
                );
                println!(
                    "   {} key fingerprint is {}",
                    issue.key_type(),
                    issue.fingerprint()
                );
                println!("   Verify it on the router with: /ip ssh print\n");

                // Without a terminal there is nobody to answer, and inquire's own
                // "not a TTY" error says nothing about how to proceed. Report the
                // issue instead: it names --accept-new-hostkey.
                let accept =
                    match inquire::Confirm::new("Accept this key and add it to known_hosts?")
                        .with_default(false)
                        .prompt()
                    {
                        Ok(answer) => answer,
                        Err(inquire::InquireError::NotTTY) => return Err(anyhow!("{issue}")),
                        Err(e) => return Err(e.into()),
                    };

                if !accept {
                    return Err(anyhow!("host key rejected; not connecting"));
                }

                ssh_config.host_key_policy = HostKeyPolicy::AcceptNew;
                // Loop and retry, this time recording the key.
            }
        }
    }
}

async fn run_dump_command(
    cli: &CliArgs,
    resource: &str,
    format: &str,
    force_demo: bool,
) -> Result<()> {
    // `--demo` on the subcommand has to be honoured *before* host selection, otherwise
    // it still walks into the interactive first-run wizard.
    let ssh_config = if force_demo || cli.demo {
        SshConfig::default()
    } else {
        determine_ssh_config(cli, false)?
            .ok_or_else(|| anyhow!("No host configuration selected"))?
    };

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
    // Reject a write command before opening a connection, so `mikrotui exec` cannot even
    // be used to probe which credentials a router accepts for a mutating operation.
    ssh::guard::ensure_read_only(command).map_err(|e| anyhow!(e))?;

    let ssh_config = if force_demo || cli.demo {
        SshConfig::default()
    } else {
        determine_ssh_config(cli, true)?.ok_or_else(|| anyhow!("No host configuration selected"))?
    };

    let client = if ssh_config.demo_mode {
        let c = RouterClient::new(ssh_config);
        c.connect().await?;
        c
    } else {
        connect_interactively(ssh_config).await?
    };

    let output = client.exec_command(command).await?;
    println!("=== Raw Response from MikroTik CLI ===");
    println!("{}", output);
    println!("=====================================");

    Ok(())
}

/// Host key settings shared by every code path that builds an `SshConfig`.
fn host_key_settings(cli: &CliArgs) -> (HostKeyPolicy, Option<PathBuf>) {
    let policy = if cli.accept_new_hostkey {
        HostKeyPolicy::AcceptNew
    } else {
        HostKeyPolicy::Strict
    };
    (policy, cli.known_hosts.clone())
}

/// Resolve the password for a host, in the documented precedence order.
///
/// `interactive` is false for `dump`/`exec` piping into scripts, where a hanging prompt
/// would be worse than a clear failure.
fn resolve_password(
    cli: &CliArgs,
    user: &str,
    host: &str,
    port: u16,
    stored: Option<String>,
    from_file: bool,
    interactive: bool,
) -> Result<Option<String>> {
    if let Some(p) = &cli.password {
        eprintln!(
            "⚠️  --password is visible in `ps` output and your shell history. \
             Prefer --password-stdin or MIKROTUI_PASSWORD."
        );
        return Ok(Some(p.clone()));
    }

    if cli.password_stdin {
        return Ok(Some(secrets::read_from_stdin()?));
    }

    if let Some(p) = secrets::read_from_env() {
        return Ok(Some(p));
    }

    let account = secrets::account_id(user, host, port);
    if let Some(p) = secrets::keyring_get(&account) {
        return Ok(Some(p));
    }

    if let Some(p) = stored {
        if from_file {
            eprintln!(
                "⚠️  Using the password stored in config.json for {account}. It is only \
                 obfuscated, not encrypted — anyone who can read that file can recover it. \
                 Run `mikrotui host migrate` to move it into the OS keyring."
            );
        }
        return Ok(Some(p));
    }

    if interactive {
        return Ok(Some(secrets::prompt(&account)?));
    }

    Ok(None)
}

/// Read an encrypted key's passphrase now, while a terminal is still available.
///
/// `connect` may later run inside the TUI, where there is nowhere to ask.
fn collect_key_passphrase(path: &Path, interactive: bool) -> Result<Option<String>> {
    let prompt = |label: &str| -> Result<String> {
        Ok(inquire::Password::new(label)
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .without_confirmation()
            .prompt()?)
    };

    // Validating here also surfaces a missing, unreadable or RSA key before the TUI takes
    // over the screen, where the error would be invisible.
    ssh::identity::prepare(path, interactive.then_some(&prompt))
}

fn determine_ssh_config(cli: &CliArgs, interactive: bool) -> Result<Option<SshConfig>> {
    let (host_key_policy, known_hosts) = host_key_settings(cli);

    if cli.demo {
        return Ok(Some(SshConfig {
            demo_mode: true,
            host_key_policy,
            known_hosts,
            ..SshConfig::default()
        }));
    }

    if let Some(host) = &cli.host {
        let user = cli.user.clone().unwrap_or_else(|| "admin".to_string());
        let port = cli.port.unwrap_or(22);
        // A key replaces the password entirely; do not prompt for one that is unused.
        let (pass, key_path, key_passphrase) = match cli.identity.clone() {
            Some(path) => {
                let passphrase = collect_key_passphrase(&path, interactive)?;
                (None, Some(path), passphrase)
            }
            None => (
                resolve_password(cli, &user, host, port, None, false, interactive)?,
                None,
                None,
            ),
        };
        return Ok(Some(SshConfig {
            host: host.clone(),
            port,
            user,
            pass,
            key_path,
            key_passphrase,
            demo_mode: false,
            host_key_policy,
            known_hosts,
        }));
    }

    if AppConfig::exists() {
        let config = AppConfig::load()?;
        if !config.hosts.is_empty() {
            let host_cfg = wizard::prompt_select_host(&config)?;
            // A key configured for this host, or one given on the command line.
            let identity = cli
                .identity
                .clone()
                .or_else(|| host_cfg.identity_file.clone());
            if let Some(path) = &identity {
                let key_passphrase = collect_key_passphrase(path, interactive)?;
                return Ok(Some(SshConfig {
                    host: host_cfg.host.clone(),
                    port: host_cfg.port,
                    user: host_cfg.user.clone(),
                    pass: None,
                    key_path: identity.clone(),
                    key_passphrase,
                    demo_mode: false,
                    host_key_policy,
                    known_hosts,
                }));
            }

            let stored = secrets::keyring_get(&host_cfg.account_id())
                .map(|p| (p, false))
                .or_else(|| host_cfg.file_password().map(|p| (p, true)));
            let (stored, from_file) = match stored {
                Some((p, f)) => (Some(p), f),
                None => (None, false),
            };
            let pass = resolve_password(
                cli,
                &host_cfg.user,
                &host_cfg.host,
                host_cfg.port,
                stored,
                from_file,
                interactive,
            )?;
            return Ok(Some(SshConfig {
                host: host_cfg.host.clone(),
                port: host_cfg.port,
                user: host_cfg.user.clone(),
                pass,
                key_path: None,
                key_passphrase: None,
                demo_mode: false,
                host_key_policy,
                known_hosts,
            }));
        }
    }

    let first_run = wizard::handle_first_time_run()?;
    Ok(first_run.map(|mut cfg| {
        cfg.host_key_policy = host_key_policy;
        cfg.known_hosts = known_hosts;
        cfg
    }))
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<app::AppEvent>(32);

    loop {
        // Process background events from Tokio channel
        while let Ok(event) = rx.try_recv() {
            app.handle_event(event);
        }

        // A tab the user has not opened yet holds nothing, since a refresh only fetches
        // what is on screen.
        if app.active_tab_needs_data() || app.auto_refresh_due() {
            app.trigger_background_reload(tx.clone());
        }

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Windows reports both press and release for every keystroke; without
                // this each key would act twice.
                if key.kind != KeyEventKind::Press {
                    continue;
                }

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

                        // Scrolling by screenfuls and jumping to either end. Lists longer
                        // than the terminal are common (firewall rules, DHCP leases, logs)
                        // and row-by-row is not a practical way to cross them.
                        (KeyCode::PageDown, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                            app.page_down()
                        }
                        (KeyCode::PageUp, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                            app.page_up()
                        }
                        (KeyCode::Home, _) | (KeyCode::Char('g'), _) => app.select_first(),
                        (KeyCode::End, _) | (KeyCode::Char('G'), _) => app.select_last(),

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
                            // The list just changed size under the selection.
                            app.clamp_selection();
                        }
                        KeyCode::Char(c) => {
                            app.filter_query.push(c);
                            app.clamp_selection();
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}
