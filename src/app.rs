use crate::config::{AppConfig, HostConfig};
use crate::models::*;
use crate::ssh::{RouterClient, SshConfig};
use crate::ui::theme::Theme;
use anyhow::Result;
use std::cell::Cell;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    System,
    Interfaces,
    IpAddresses,
    IpRoutes,
    DhcpLeases,
    Firewall,
    Neighbors,
    Logs,
}

impl Tab {
    pub const ALL: [Tab; 8] = [
        Tab::System,
        Tab::Interfaces,
        Tab::IpAddresses,
        Tab::IpRoutes,
        Tab::DhcpLeases,
        Tab::Firewall,
        Tab::Neighbors,
        Tab::Logs,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::System => "System Resources",
            Tab::Interfaces => "Interfaces",
            Tab::IpAddresses => "IP Addresses",
            Tab::IpRoutes => "IP Routes",
            Tab::DhcpLeases => "DHCP Leases",
            Tab::Firewall => "Firewall Rules",
            Tab::Neighbors => "Network Neighbors",
            Tab::Logs => "System Logs",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Tab::System => "⚙ ",
            Tab::Interfaces => "🔌",
            Tab::IpAddresses => "🌐",
            Tab::IpRoutes => "🔀",
            Tab::DhcpLeases => "💻",
            Tab::Firewall => "🛡 ",
            Tab::Neighbors => "📡 ",
            Tab::Logs => "📜",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filtering,
}

#[derive(Debug, Clone)]
pub enum PingState {
    Inactive,
    InputtingTarget { input: String },
    Running { target: String },
    Completed { result: PingResult },
}

/// One refresh's worth of router state. `None` means that fetch failed; an empty `Vec`
/// means the router genuinely has none of that resource.
#[derive(Default)]
pub struct LoadedData {
    pub system: Option<SystemResource>,
    pub interfaces: Option<Vec<Interface>>,
    pub ip_addresses: Option<Vec<IpAddress>>,
    pub ip_routes: Option<Vec<IpRoute>>,
    pub dhcp_leases: Option<Vec<DhcpLease>>,
    pub firewall_rules: Option<Vec<FirewallRule>>,
    pub neighbors: Option<Vec<Neighbor>>,
    pub logs: Option<Vec<LogEntry>>,
}

pub enum AppEvent {
    /// A refresh could not reach the router. Previously every failure was swallowed by
    /// `.ok()` and still reported as success, which would have hidden host key
    /// rejections entirely.
    LoadFailed {
        generation: u64,
        error: String,
    },
    HostKeyVerified {
        generation: u64,
        verified: bool,
    },
    /// Boxed: inline this payload is ~490 bytes and would set the size of every event.
    DataLoaded {
        generation: u64,
        data: Box<LoadedData>,
    },
    PingFinished(PingResult),
    PingFailed {
        target: String,
        error: String,
    },
}

pub struct App {
    pub active_tab: Tab,
    pub selected_index: usize,
    /// First row drawn by the active table, carried between frames so scrolling is
    /// continuous. `Cell` because rendering takes `&App` while the table widget needs to
    /// report back the offset it settled on.
    pub table_offset: Cell<usize>,
    /// How many rows the active table last had room for. Written by the renderer, read by
    /// the page-up/page-down handlers, which cannot otherwise know the terminal height.
    pub viewport_rows: Cell<usize>,
    /// Whether the current session's host key matched `known_hosts`.
    pub host_key_verified: bool,
    /// Incremented whenever the active router changes, so results from a refresh started
    /// against the previous host can be recognised and discarded.
    pub reload_generation: u64,
    pub input_mode: InputMode,
    pub filter_query: String,
    pub client: RouterClient,
    pub theme: Theme,
    pub show_detail_modal: bool,
    pub show_help_modal: bool,
    pub show_host_switch_modal: bool,
    pub host_switch_selected: usize,
    pub available_hosts: Vec<HostConfig>,
    pub ping_state: PingState,

    // Data models
    pub system_resource: SystemResource,
    pub interfaces: Vec<Interface>,
    pub ip_addresses: Vec<IpAddress>,
    pub ip_routes: Vec<IpRoute>,
    pub dhcp_leases: Vec<DhcpLease>,
    pub firewall_rules: Vec<FirewallRule>,
    pub neighbors: Vec<Neighbor>,
    pub logs: Vec<LogEntry>,

    pub status_message: String,
    pub is_loading: bool,
}

impl App {
    /// Build the app around a client that already connected, so the TUI reuses the
    /// session whose host key was verified (and accepted) before raw mode was entered.
    pub fn with_client(client: RouterClient, host_key_verified: bool) -> Self {
        let demo = client.config.demo_mode;
        Self {
            active_tab: Tab::System,
            selected_index: 0,
            table_offset: Cell::new(0),
            viewport_rows: Cell::new(1),
            host_key_verified,
            reload_generation: 0,
            input_mode: InputMode::Normal,
            filter_query: String::new(),
            client,
            theme: Theme::winbox_dark(),
            show_detail_modal: false,
            show_help_modal: false,
            show_host_switch_modal: false,
            host_switch_selected: 0,
            available_hosts: Vec::new(),
            ping_state: PingState::Inactive,
            system_resource: SystemResource::default(),
            interfaces: Vec::new(),
            ip_addresses: Vec::new(),
            ip_routes: Vec::new(),
            dhcp_leases: Vec::new(),
            firewall_rules: Vec::new(),
            neighbors: Vec::new(),
            logs: Vec::new(),
            status_message: if demo {
                "Demo mode — showing sample data, no router is connected.".to_string()
            } else {
                "Connected. Read-only guard active — use a RouterOS `read` account for a real guarantee.".to_string()
            },
            is_loading: false,
        }
    }

    pub fn open_host_switch_prompt(&mut self) {
        if let Ok(config) = AppConfig::load() {
            self.available_hosts = config.hosts;
        }
        self.host_switch_selected = 0;
        self.show_host_switch_modal = true;
    }

    pub fn switch_host(&mut self, idx: usize, tx: mpsc::Sender<AppEvent>) {
        if let Some(host_cfg) = self.available_hosts.get(idx).cloned() {
            // Inside the TUI there is no way to prompt for a host key decision, so an
            // unknown host is refused rather than trusted; the error explains how to
            // accept it from the command line.
            let from_keyring = crate::secrets::keyring_get(&host_cfg.account_id());
            let from_file = from_keyring.is_none() && host_cfg.file_password().is_some();

            let new_ssh_config = SshConfig {
                host: host_cfg.host.clone(),
                port: host_cfg.port,
                user: host_cfg.user.clone(),
                pass: from_keyring.or_else(|| host_cfg.file_password()),
                key_path: None,
                demo_mode: false,
                host_key_policy: crate::ssh::HostKeyPolicy::Strict,
                known_hosts: self.client.config.known_hosts.clone(),
            };

            self.client = RouterClient::new(new_ssh_config);
            self.host_key_verified = false;

            // Abandon whatever the previous router's refresh was doing. Its task is still
            // running and will still send its results; without a new generation number
            // those rows would land here and be shown as the new host's data, under a
            // header naming the new host and a badge reporting the old host's key.
            self.reload_generation = self.reload_generation.wrapping_add(1);
            self.is_loading = false;
            self.show_host_switch_modal = false;
            self.status_message = if from_file {
                format!(
                    "Connecting to [{}] ({})... ⚠️ password read from config.json, which is \
                     only obfuscated — run 'mikrotui host migrate'",
                    host_cfg.name, host_cfg.host
                )
            } else {
                format!(
                    "Connecting to router [{}] ({})...",
                    host_cfg.name, host_cfg.host
                )
            };

            // Reset current view models
            self.system_resource = SystemResource::default();
            self.interfaces.clear();
            self.ip_addresses.clear();
            self.ip_routes.clear();
            self.dhcp_leases.clear();
            self.firewall_rules.clear();
            self.neighbors.clear();
            self.logs.clear();
            self.selected_index = 0;

            // Trigger background reload from new host
            self.trigger_background_reload(tx);
        }
    }

    pub fn open_ping_prompt(&mut self) {
        let default_target = self.get_selected_ip_or_default();
        self.ping_state = PingState::InputtingTarget {
            input: default_target,
        };
    }

    pub fn trigger_ping(&mut self, target: String, tx: mpsc::Sender<AppEvent>) {
        let target = target.trim().to_string();
        if target.is_empty() {
            self.ping_state = PingState::Inactive;
            return;
        }

        // The target is interpolated straight into a RouterOS command line, so anything
        // the CLI treats as punctuation would let the ping box reach commands the read-only
        // guard is meant to stand in front of. A host or address has none of it.
        if !is_pingable_target(&target) {
            self.ping_state = PingState::Inactive;
            self.status_message = format!(
                "❌ '{target}' is not a valid host or address (letters, digits, '.', ':', '-' and '_' only)"
            );
            return;
        }
        self.ping_state = PingState::Running {
            target: target.clone(),
        };
        let client = self.client.clone();

        tokio::spawn(async move {
            // Dropping the error here left PingState::Running in place with no way out but
            // Esc, and no indication of what went wrong.
            let event = match client.run_ping(&target, 5).await {
                Ok(res) => AppEvent::PingFinished(res),
                Err(err) => AppEvent::PingFailed {
                    target,
                    error: err.to_string(),
                },
            };
            let _ = tx.send(event).await;
        });
    }

    /// Accept a ping result, unless the user already dismissed the modal.
    ///
    /// `Esc` during a ping sets the state back to `Inactive`, but the request is still in
    /// flight; storing its result unconditionally reopened the modal the user had just
    /// closed. A result whose target no longer matches belongs to a superseded ping and is
    /// dropped for the same reason.
    pub fn finish_ping(&mut self, result: PingResult) {
        let awaited = match &self.ping_state {
            PingState::Running { target } => target == &result.target,
            _ => false,
        };
        if !awaited {
            return;
        }

        self.status_message = format!(
            "✅ Ping completed for {}: {}% loss",
            result.target, result.packet_loss_pct
        );
        self.ping_state = PingState::Completed { result };
    }

    /// Close a ping that could not run, unless the user already dismissed it.
    pub fn fail_ping(&mut self, target: &str, error: String) {
        let awaited = match &self.ping_state {
            PingState::Running { target: t } => t == target,
            _ => false,
        };
        if !awaited {
            return;
        }
        self.ping_state = PingState::Inactive;
        self.status_message = format!("❌ Ping to {target} failed: {error}");
    }

    pub fn get_selected_ip_or_default(&self) -> String {
        match self.active_tab {
            Tab::IpAddresses => {
                if let Some(item) = self.filtered_ip_addresses().get(self.selected_index) {
                    return item
                        .address
                        .split('/')
                        .next()
                        .unwrap_or("8.8.8.8")
                        .to_string();
                }
            }
            Tab::IpRoutes => {
                if let Some(item) = self.filtered_ip_routes().get(self.selected_index) {
                    if !item.gateway.is_empty() && !item.gateway.contains("ether") {
                        return item.gateway.clone();
                    }
                    if !item.dst_address.is_empty() && item.dst_address != "0.0.0.0/0" {
                        return item
                            .dst_address
                            .split('/')
                            .next()
                            .unwrap_or("8.8.8.8")
                            .to_string();
                    }
                }
            }
            Tab::DhcpLeases => {
                if let Some(item) = self.filtered_dhcp_leases().get(self.selected_index) {
                    return item.address.clone();
                }
            }
            Tab::Neighbors => {
                if let Some(item) = self.filtered_neighbors().get(self.selected_index) {
                    if !item.ip_address.is_empty() {
                        return item.ip_address.clone();
                    }
                }
            }
            _ => {}
        }
        "8.8.8.8".to_string()
    }

    pub fn trigger_background_reload(&mut self, tx: mpsc::Sender<AppEvent>) {
        if self.is_loading {
            self.status_message = "⚠️ Reload already in progress. Please wait...".to_string();
            return;
        }

        self.is_loading = true;
        self.status_message = "⏳ Refreshing data via SSH in background...".to_string();

        let client = self.client.clone();
        let generation = self.reload_generation;

        tokio::spawn(async move {
            // Bound the whole cycle. Every path out of this task has to emit an event:
            // `is_loading` stays set until one arrives, and while it is set every later
            // refresh is refused, so a task that never finishes disables reloading for
            // the rest of the session.
            let outcome = tokio::time::timeout(crate::ssh::REFRESH_TIMEOUT, async {
                // Connect explicitly so a rejected host key is reported instead of
                // showing up as eight silently empty result sets.
                client.connect().await?;

                let verified = client.host_key_verified().await;
                let data = LoadedData {
                    system: client.fetch_system_resource().await.ok(),
                    interfaces: client.fetch_interfaces().await.ok(),
                    ip_addresses: client.fetch_ip_addresses().await.ok(),
                    ip_routes: client.fetch_ip_routes().await.ok(),
                    dhcp_leases: client.fetch_dhcp_leases().await.ok(),
                    firewall_rules: client.fetch_firewall_rules().await.ok(),
                    neighbors: client.fetch_neighbors().await.ok(),
                    logs: client.fetch_logs().await.ok(),
                };
                Ok::<_, anyhow::Error>((verified, data))
            })
            .await;

            let event = match outcome {
                Ok(Ok((verified, data))) => {
                    let _ = tx
                        .send(AppEvent::HostKeyVerified {
                            generation,
                            verified,
                        })
                        .await;
                    AppEvent::DataLoaded {
                        generation,
                        data: Box::new(data),
                    }
                }
                Ok(Err(err)) => AppEvent::LoadFailed {
                    generation,
                    error: err.to_string(),
                },
                Err(_) => AppEvent::LoadFailed {
                    generation,
                    error: format!(
                        "refresh gave up after {}s without a reply from the router",
                        crate::ssh::REFRESH_TIMEOUT.as_secs()
                    ),
                },
            };

            let _ = tx.send(event).await;
        });
    }

    pub fn apply_loaded_data(&mut self, data: LoadedData) {
        let LoadedData {
            system,
            interfaces,
            ip_addresses,
            ip_routes,
            dhcp_leases,
            firewall_rules,
            neighbors,
            logs,
        } = data;

        if let Some(res) = system {
            if !res.board_name.is_empty() || !res.version.is_empty() {
                self.system_resource = res;
            }
        }
        if let Some(ifaces) = interfaces {
            if !ifaces.is_empty() {
                self.interfaces = ifaces;
            }
        }
        if let Some(addrs) = ip_addresses {
            if !addrs.is_empty() {
                self.ip_addresses = addrs;
            }
        }
        if let Some(routes) = ip_routes {
            if !routes.is_empty() {
                self.ip_routes = routes;
            }
        }
        if let Some(dhcp) = dhcp_leases {
            if !dhcp.is_empty() {
                self.dhcp_leases = dhcp;
            }
        }
        if let Some(fw) = firewall_rules {
            if !fw.is_empty() {
                self.firewall_rules = fw;
            }
        }
        if let Some(neigh) = neighbors {
            if !neigh.is_empty() {
                self.neighbors = neigh;
            }
        }
        if let Some(l) = logs {
            if !l.is_empty() {
                self.logs = l;
            }
        }

        // A refresh can return fewer rows than before (a lease expired, a rule was
        // removed on the router), leaving the selection past the end of the new list.
        self.clamp_selection();

        self.is_loading = false;
        self.status_message = "✅ Data successfully updated via SSH.".to_string();
    }

    pub fn toggle_help_modal(&mut self) {
        self.show_help_modal = !self.show_help_modal;
    }

    pub fn cycle_theme(&mut self) {
        let next_kind = self.theme.kind.next();
        self.theme = Theme::from_kind(next_kind);
        self.status_message = format!("Theme changed to: {}", next_kind.name());
    }

    pub async fn load_all_data(&mut self) -> Result<()> {
        self.is_loading = true;

        if let Ok(res) = self.client.fetch_system_resource().await {
            if !res.board_name.is_empty() || !res.version.is_empty() {
                self.system_resource = res;
            }
        }

        if let Ok(ifaces) = self.client.fetch_interfaces().await {
            if !ifaces.is_empty() {
                self.interfaces = ifaces;
            }
        }

        if let Ok(addrs) = self.client.fetch_ip_addresses().await {
            if !addrs.is_empty() {
                self.ip_addresses = addrs;
            }
        }

        if let Ok(routes) = self.client.fetch_ip_routes().await {
            if !routes.is_empty() {
                self.ip_routes = routes;
            }
        }

        if let Ok(dhcp) = self.client.fetch_dhcp_leases().await {
            if !dhcp.is_empty() {
                self.dhcp_leases = dhcp;
            }
        }

        if let Ok(fw) = self.client.fetch_firewall_rules().await {
            if !fw.is_empty() {
                self.firewall_rules = fw;
            }
        }

        if let Ok(neigh) = self.client.fetch_neighbors().await {
            if !neigh.is_empty() {
                self.neighbors = neigh;
            }
        }

        if let Ok(logs) = self.client.fetch_logs().await {
            if !logs.is_empty() {
                self.logs = logs;
            }
        }

        self.clamp_selection();
        self.is_loading = false;
        Ok(())
    }

    pub fn next_tab(&mut self) {
        let current_idx = Tab::ALL
            .iter()
            .position(|&t| t == self.active_tab)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % Tab::ALL.len();
        self.active_tab = Tab::ALL[next_idx];
        self.reset_scroll();
    }

    pub fn prev_tab(&mut self) {
        let current_idx = Tab::ALL
            .iter()
            .position(|&t| t == self.active_tab)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            Tab::ALL.len() - 1
        } else {
            current_idx - 1
        };
        self.active_tab = Tab::ALL[prev_idx];
        self.reset_scroll();
    }

    /// Return to the top of the list. Each tab holds a different number of rows, so a
    /// scroll offset carried across a tab switch would point at nothing.
    pub fn reset_scroll(&mut self) {
        self.selected_index = 0;
        self.table_offset.set(0);
    }

    /// Index of the highlighted row within a list of `len`, or `None` when it is empty.
    ///
    /// Read this rather than `selected_index` when rendering or acting on a row. A refresh
    /// or a filter edit can shrink a list under a stale index, and the highlight, the
    /// scroll position and the detail modal all have to agree on the same row.
    pub fn selection_in(&self, len: usize) -> Option<usize> {
        len.checked_sub(1).map(|last| self.selected_index.min(last))
    }

    /// Keep the selection inside the list.
    ///
    /// Typing into the filter shrinks the list under a selection that may sit past its new
    /// end, which would leave no row highlighted and make Enter open an empty detail modal.
    pub fn clamp_selection(&mut self) {
        let len = self.current_tab_len();
        self.selected_index = self.selected_index.min(len.saturating_sub(1));
    }

    /// Move down by one screenful, stopping at the last row.
    pub fn page_down(&mut self) {
        let len = self.current_tab_len();
        if len == 0 {
            return;
        }
        let page = self.viewport_rows.get().max(1);
        self.selected_index = (self.selected_index + page).min(len - 1);
    }

    /// Move up by one screenful, stopping at the first row.
    pub fn page_up(&mut self) {
        let page = self.viewport_rows.get().max(1);
        self.selected_index = self.selected_index.saturating_sub(page);
    }

    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub fn select_last(&mut self) {
        self.selected_index = self.current_tab_len().saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        let max_len = self.current_tab_len();
        if max_len > 0 {
            self.selected_index = (self.selected_index + 1) % max_len;
        }
    }

    pub fn select_prev(&mut self) {
        let max_len = self.current_tab_len();
        if max_len > 0 {
            if self.selected_index == 0 {
                self.selected_index = max_len - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn current_tab_len(&self) -> usize {
        match self.active_tab {
            Tab::System => 1,
            Tab::Interfaces => self.filtered_interfaces().len(),
            Tab::IpAddresses => self.filtered_ip_addresses().len(),
            Tab::IpRoutes => self.filtered_ip_routes().len(),
            Tab::DhcpLeases => self.filtered_dhcp_leases().len(),
            Tab::Firewall => self.filtered_firewall_rules().len(),
            Tab::Neighbors => self.filtered_neighbors().len(),
            Tab::Logs => self.filtered_logs().len(),
        }
    }

    /// Route a background event, ignoring anything from a superseded refresh.
    ///
    /// `Ctrl+O` can change router while a refresh is in flight; that task keeps running
    /// and still reports. Without this check its rows were applied to the new host and
    /// announced as success, and its host key verdict lit the badge for a router that was
    /// never contacted.
    pub fn handle_event(&mut self, event: AppEvent) {
        let generation = match &event {
            AppEvent::LoadFailed { generation, .. }
            | AppEvent::HostKeyVerified { generation, .. }
            | AppEvent::DataLoaded { generation, .. } => *generation,
            // Ping results carry their own target check.
            AppEvent::PingFinished(_) | AppEvent::PingFailed { .. } => self.reload_generation,
        };
        if generation != self.reload_generation {
            return;
        }

        match event {
            AppEvent::LoadFailed { error, .. } => self.report_load_failure(error),
            AppEvent::HostKeyVerified { verified, .. } => self.host_key_verified = verified,
            AppEvent::DataLoaded { data, .. } => self.apply_loaded_data(*data),
            AppEvent::PingFinished(result) => self.finish_ping(result),
            AppEvent::PingFailed { target, error } => self.fail_ping(&target, error),
        }
    }

    pub fn report_load_failure(&mut self, err: String) {
        self.is_loading = false;
        self.status_message = format!("❌ {err}");
    }

    pub fn filtered_interfaces(&self) -> Vec<&Interface> {
        if self.filter_query.is_empty() {
            self.interfaces.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.interfaces
                .iter()
                .filter(|i| {
                    i.name.to_lowercase().contains(&q)
                        || i.comment.to_lowercase().contains(&q)
                        || i.mac_address.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_ip_addresses(&self) -> Vec<&IpAddress> {
        if self.filter_query.is_empty() {
            self.ip_addresses.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.ip_addresses
                .iter()
                .filter(|i| {
                    i.address.to_lowercase().contains(&q)
                        || i.interface.to_lowercase().contains(&q)
                        || i.comment.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_ip_routes(&self) -> Vec<&IpRoute> {
        if self.filter_query.is_empty() {
            self.ip_routes.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.ip_routes
                .iter()
                .filter(|r| {
                    r.dst_address.to_lowercase().contains(&q)
                        || r.gateway.to_lowercase().contains(&q)
                        || r.comment.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_dhcp_leases(&self) -> Vec<&DhcpLease> {
        if self.filter_query.is_empty() {
            self.dhcp_leases.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.dhcp_leases
                .iter()
                .filter(|d| {
                    d.address.to_lowercase().contains(&q)
                        || d.host_name.to_lowercase().contains(&q)
                        || d.mac_address.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_firewall_rules(&self) -> Vec<&FirewallRule> {
        if self.filter_query.is_empty() {
            self.firewall_rules.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.firewall_rules
                .iter()
                .filter(|f| {
                    f.chain.to_lowercase().contains(&q)
                        || f.action.to_lowercase().contains(&q)
                        || f.comment.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_neighbors(&self) -> Vec<&Neighbor> {
        if self.filter_query.is_empty() {
            self.neighbors.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.neighbors
                .iter()
                .filter(|n| {
                    n.interface.to_lowercase().contains(&q)
                        || n.identity.to_lowercase().contains(&q)
                        || n.ip_address.to_lowercase().contains(&q)
                        || n.mac_address.to_lowercase().contains(&q)
                        || n.board.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn filtered_logs(&self) -> Vec<&LogEntry> {
        if self.filter_query.is_empty() {
            self.logs.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.logs
                .iter()
                .filter(|l| {
                    l.message.to_lowercase().contains(&q) || l.topics.to_lowercase().contains(&q)
                })
                .collect()
        }
    }
}

/// Whether `target` is something that can only be a host name or IP address.
///
/// Deliberately a character allowlist rather than an attempt to parse: the value is
/// interpolated into a RouterOS command line, and RouterOS treats ';', '[' and whitespace
/// as syntax.
fn is_pingable_target(target: &str) -> bool {
    !target.is_empty()
        && target.len() <= 253
        && target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' | '%'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Interface;
    use crate::ssh::{RouterClient, SshConfig};

    fn app() -> App {
        App::with_client(RouterClient::new(SshConfig::default()), false)
    }

    fn ping_result(target: &str) -> PingResult {
        PingResult {
            target: target.to_string(),
            ..Default::default()
        }
    }

    /// Esc during a ping used to be undone by the result arriving afterwards.
    #[test]
    fn a_dismissed_ping_does_not_reopen_when_its_result_arrives() {
        let mut app = app();
        app.ping_state = PingState::Running {
            target: "8.8.8.8".to_string(),
        };

        // Esc.
        app.ping_state = PingState::Inactive;

        app.finish_ping(ping_result("8.8.8.8"));
        assert!(
            matches!(app.ping_state, PingState::Inactive),
            "a dismissed ping must stay dismissed"
        );
    }

    #[test]
    fn a_ping_the_user_is_waiting_for_still_shows_its_result() {
        let mut app = app();
        app.ping_state = PingState::Running {
            target: "1.1.1.1".to_string(),
        };

        app.finish_ping(ping_result("1.1.1.1"));
        match &app.ping_state {
            PingState::Completed { result } => assert_eq!(result.target, "1.1.1.1"),
            other => panic!("expected a completed ping, got {other:?}"),
        }
    }

    /// Cancelling one ping and starting another must not show the stale result.
    #[test]
    fn a_superseded_ping_result_is_dropped() {
        let mut app = app();
        app.ping_state = PingState::Running {
            target: "1.1.1.1".to_string(),
        };

        app.finish_ping(ping_result("8.8.8.8"));
        assert!(
            matches!(&app.ping_state, PingState::Running { target } if target == "1.1.1.1"),
            "the in-flight ping should still be pending"
        );
    }

    /// Switching router while a refresh is in flight used to apply the previous
    /// router's rows to the new host, announce them as success, and light the host key
    /// badge for a router that was never contacted.
    #[test]
    fn results_from_a_superseded_refresh_are_discarded() {
        let mut app = app();
        let generation = app.reload_generation;
        app.is_loading = true;

        // Ctrl+O: the active router changes.
        app.reload_generation = app.reload_generation.wrapping_add(1);

        let data = LoadedData {
            interfaces: Some(vec![Interface {
                name: "PREVIOUS-ROUTER-ether1".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        };
        app.handle_event(AppEvent::DataLoaded {
            generation,
            data: Box::new(data),
        });
        app.handle_event(AppEvent::HostKeyVerified {
            generation,
            verified: true,
        });

        assert!(
            app.interfaces.is_empty(),
            "the old router's rows must not appear under the new host"
        );
        assert!(
            !app.host_key_verified,
            "the old router's key verdict must not light the badge"
        );

        // The current generation still applies.
        app.handle_event(AppEvent::HostKeyVerified {
            generation: app.reload_generation,
            verified: true,
        });
        assert!(app.host_key_verified);
    }

    /// The ping target is interpolated into a command line.
    #[test]
    fn ping_targets_that_are_not_hosts_are_refused() {
        for good in ["8.8.8.8", "router.example.com", "fe80::1", "my-host_1"] {
            assert!(is_pingable_target(good), "{good} should be accepted");
        }
        for bad in [
            "8.8.8.8; /ip dhcp-server lease make-static [find]",
            "8.8.8.8 count=1",
            "[/system reboot]",
            "",
        ] {
            assert!(!is_pingable_target(bad), "{bad:?} should be refused");
        }
    }

    #[test]
    fn a_ping_that_cannot_run_closes_the_modal() {
        let mut app = app();
        app.ping_state = PingState::Running {
            target: "1.1.1.1".to_string(),
        };

        app.fail_ping("1.1.1.1", "connection refused".to_string());

        assert!(
            matches!(app.ping_state, PingState::Inactive),
            "a failed ping must not leave the modal spinning"
        );
        assert!(app.status_message.contains("connection refused"));
    }

    /// `is_loading` gates every refresh, so a failure has to clear it or reloading is
    /// disabled for the rest of the session.
    #[test]
    fn a_failed_refresh_re_enables_reloading() {
        let mut app = app();
        app.is_loading = true;

        app.report_load_failure("router stopped replying".to_string());

        assert!(!app.is_loading, "a failure must release the reload gate");
        assert!(app.status_message.contains("router stopped replying"));
    }

    #[test]
    fn applying_data_re_enables_reloading() {
        let mut app = app();
        app.is_loading = true;

        app.apply_loaded_data(LoadedData::default());

        assert!(!app.is_loading);
    }
}
