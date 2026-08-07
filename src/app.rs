use crate::config::{AppConfig, HostConfig};
use crate::models::*;
use crate::ssh::{RouterClient, SshConfig};
use crate::ui::theme::Theme;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc;

pub fn format_bps(bps: u64) -> String {
    if bps >= 1_000_000_000 {
        format!("{:.1} Gbps", bps as f64 / 1_000_000_000.0)
    } else if bps >= 1_000_000 {
        format!("{:.1} Mbps", bps as f64 / 1_000_000.0)
    } else if bps >= 1_000 {
        format!("{:.1} Kbps", bps as f64 / 1_000.0)
    } else {
        format!("{} bps", bps)
    }
}

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

pub enum AppEvent {
    DataLoaded {
        system: Option<SystemResource>,
        interfaces: Option<Vec<Interface>>,
        ip_addresses: Option<Vec<IpAddress>>,
        ip_routes: Option<Vec<IpRoute>>,
        dhcp_leases: Option<Vec<DhcpLease>>,
        firewall_rules: Option<Vec<FirewallRule>>,
        neighbors: Option<Vec<Neighbor>>,
        logs: Option<Vec<LogEntry>>,
    },
    PingFinished(PingResult),
}

pub struct App {
    pub active_tab: Tab,
    pub selected_index: usize,
    pub safe_mode: bool,
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

    // Real-time Traffic History for Sparklines
    pub rx_history: Vec<u64>,
    pub tx_history: Vec<u64>,
    pub current_rx_bps: u64,
    pub current_tx_bps: u64,
    pub last_traffic_sample: Option<(Instant, HashMap<String, (u64, u64)>)>,

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
    pub fn new(config: SshConfig) -> Self {
        Self {
            active_tab: Tab::System,
            selected_index: 0,
            safe_mode: true,
            input_mode: InputMode::Normal,
            filter_query: String::new(),
            client: RouterClient::new(config),
            theme: Theme::winbox_dark(),
            show_detail_modal: false,
            show_help_modal: false,
            show_host_switch_modal: false,
            host_switch_selected: 0,
            available_hosts: Vec::new(),
            ping_state: PingState::Inactive,
            rx_history: Vec::new(),
            tx_history: Vec::new(),
            current_rx_bps: 0,
            current_tx_bps: 0,
            last_traffic_sample: None,
            system_resource: SystemResource::default(),
            interfaces: Vec::new(),
            ip_addresses: Vec::new(),
            ip_routes: Vec::new(),
            dhcp_leases: Vec::new(),
            firewall_rules: Vec::new(),
            neighbors: Vec::new(),
            logs: Vec::new(),
            status_message: "Connected. SAFE MODE ENABLED BY DEFAULT (Read-Only)".to_string(),
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
            let new_ssh_config = SshConfig {
                host: host_cfg.host.clone(),
                port: host_cfg.port,
                user: host_cfg.user.clone(),
                pass: host_cfg.get_password(),
                key_path: None,
                demo_mode: false,
            };

            self.client = RouterClient::new(new_ssh_config);
            self.show_host_switch_modal = false;
            self.status_message = format!("Connecting to router [{}] ({})...", host_cfg.name, host_cfg.host);

            // Reset current view models & traffic history
            self.system_resource = SystemResource::default();
            self.interfaces.clear();
            self.ip_addresses.clear();
            self.ip_routes.clear();
            self.dhcp_leases.clear();
            self.firewall_rules.clear();
            self.neighbors.clear();
            self.logs.clear();
            self.rx_history.clear();
            self.tx_history.clear();
            self.current_rx_bps = 0;
            self.current_tx_bps = 0;
            self.last_traffic_sample = None;
            self.selected_index = 0;

            // Trigger background reload from new host
            self.trigger_background_reload(tx);
        }
    }

    pub fn open_ping_prompt(&mut self) {
        let default_target = self.get_selected_ip_or_default();
        self.ping_state = PingState::InputtingTarget { input: default_target };
    }

    pub fn trigger_ping(&mut self, target: String, tx: mpsc::Sender<AppEvent>) {
        if target.trim().is_empty() {
            self.ping_state = PingState::Inactive;
            return;
        }
        self.ping_state = PingState::Running { target: target.clone() };
        let client = self.client.clone();

        tokio::spawn(async move {
            if let Ok(res) = client.run_ping(&target, 5).await {
                let _ = tx.send(AppEvent::PingFinished(res)).await;
            }
        });
    }

    pub fn get_selected_ip_or_default(&self) -> String {
        match self.active_tab {
            Tab::IpAddresses => {
                if let Some(item) = self.filtered_ip_addresses().get(self.selected_index) {
                    return item.address.split('/').next().unwrap_or("8.8.8.8").to_string();
                }
            }
            Tab::IpRoutes => {
                if let Some(item) = self.filtered_ip_routes().get(self.selected_index) {
                    if !item.gateway.is_empty() && !item.gateway.contains("ether") {
                        return item.gateway.clone();
                    }
                    if !item.dst_address.is_empty() && item.dst_address != "0.0.0.0/0" {
                        return item.dst_address.split('/').next().unwrap_or("8.8.8.8").to_string();
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

        tokio::spawn(async move {
            let system = client.fetch_system_resource().await.ok();
            let interfaces = client.fetch_interfaces().await.ok();
            let ip_addresses = client.fetch_ip_addresses().await.ok();
            let ip_routes = client.fetch_ip_routes().await.ok();
            let dhcp_leases = client.fetch_dhcp_leases().await.ok();
            let firewall_rules = client.fetch_firewall_rules().await.ok();
            let neighbors = client.fetch_neighbors().await.ok();
            let logs = client.fetch_logs().await.ok();

            let _ = tx.send(AppEvent::DataLoaded {
                system,
                interfaces,
                ip_addresses,
                ip_routes,
                dhcp_leases,
                firewall_rules,
                neighbors,
                logs,
            }).await;
        });
    }

    pub fn apply_loaded_data(
        &mut self,
        system: Option<SystemResource>,
        interfaces: Option<Vec<Interface>>,
        ip_addresses: Option<Vec<IpAddress>>,
        ip_routes: Option<Vec<IpRoute>>,
        dhcp_leases: Option<Vec<DhcpLease>>,
        firewall_rules: Option<Vec<FirewallRule>>,
        neighbors: Option<Vec<Neighbor>>,
        logs: Option<Vec<LogEntry>>,
    ) {
        if let Some(res) = system { if !res.board_name.is_empty() || !res.version.is_empty() { self.system_resource = res; } }
        if let Some(ifaces) = interfaces {
            if !ifaces.is_empty() {
                self.update_traffic_history(&ifaces);
                self.interfaces = ifaces;
            }
        }
        if let Some(addrs) = ip_addresses { if !addrs.is_empty() { self.ip_addresses = addrs; } }
        if let Some(routes) = ip_routes { if !routes.is_empty() { self.ip_routes = routes; } }
        if let Some(dhcp) = dhcp_leases { if !dhcp.is_empty() { self.dhcp_leases = dhcp; } }
        if let Some(fw) = firewall_rules { if !fw.is_empty() { self.firewall_rules = fw; } }
        if let Some(neigh) = neighbors { if !neigh.is_empty() { self.neighbors = neigh; } }
        if let Some(l) = logs { if !l.is_empty() { self.logs = l; } }

        self.is_loading = false;
        self.status_message = "✅ Data successfully updated via SSH.".to_string();
    }

    fn update_traffic_history(&mut self, ifaces: &[Interface]) {
        let now = Instant::now();
        let selected_iface_name = self.filtered_interfaces()
            .get(self.selected_index)
            .map(|i| i.name.clone())
            .unwrap_or_else(|| ifaces.first().map(|i| i.name.clone()).unwrap_or_default());

        if let Some((last_time, last_map)) = &self.last_traffic_sample {
            let elapsed_secs = now.duration_since(*last_time).as_secs_f64().max(0.1);

            if let Some(current_iface) = ifaces.iter().find(|i| i.name == selected_iface_name) {
                if let Some(&(prev_rx, prev_tx)) = last_map.get(&selected_iface_name) {
                    let delta_rx = current_iface.rx_byte.saturating_sub(prev_rx);
                    let delta_tx = current_iface.tx_byte.saturating_sub(prev_tx);

                    let calc_rx_bps = ((delta_rx as f64 * 8.0) / elapsed_secs) as u64;
                    let calc_tx_bps = ((delta_tx as f64 * 8.0) / elapsed_secs) as u64;

                    self.current_rx_bps = calc_rx_bps;
                    self.current_tx_bps = calc_tx_bps;

                    self.rx_history.push(self.current_rx_bps);
                    self.tx_history.push(self.current_tx_bps);
                }
            }
        } else {
            // Initial sample: push starting points
            self.rx_history.push(0);
            self.tx_history.push(0);
        }

        // Limit history buffer to 30 data points
        while self.rx_history.len() > 30 {
            self.rx_history.remove(0);
        }
        while self.tx_history.len() > 30 {
            self.tx_history.remove(0);
        }

        let mut new_map = HashMap::new();
        for i in ifaces {
            new_map.insert(i.name.clone(), (i.rx_byte, i.tx_byte));
        }
        self.last_traffic_sample = Some((now, new_map));
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
                self.update_traffic_history(&ifaces);
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

        self.is_loading = false;
        Ok(())
    }

    pub fn next_tab(&mut self) {
        let current_idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        let next_idx = (current_idx + 1) % Tab::ALL.len();
        self.active_tab = Tab::ALL[next_idx];
        self.selected_index = 0;
    }

    pub fn prev_tab(&mut self) {
        let current_idx = Tab::ALL.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        let prev_idx = if current_idx == 0 { Tab::ALL.len() - 1 } else { current_idx - 1 };
        self.active_tab = Tab::ALL[prev_idx];
        self.selected_index = 0;
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

    pub fn toggle_safe_mode(&mut self) {
        self.safe_mode = !self.safe_mode;
        if self.safe_mode {
            self.status_message = "Safe Mode: ENABLED (Ctrl+X)".to_string();
        } else {
            self.status_message = "Safe Mode: DISABLED (Warning!)".to_string();
        }
    }

    pub fn filtered_interfaces(&self) -> Vec<&Interface> {
        if self.filter_query.is_empty() {
            self.interfaces.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.interfaces.iter().filter(|i| {
                i.name.to_lowercase().contains(&q) || i.comment.to_lowercase().contains(&q) || i.mac_address.to_lowercase().contains(&q)
            }).collect()
        }
    }

    pub fn filtered_ip_addresses(&self) -> Vec<&IpAddress> {
        if self.filter_query.is_empty() {
            self.ip_addresses.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.ip_addresses.iter().filter(|i| {
                i.address.to_lowercase().contains(&q) || i.interface.to_lowercase().contains(&q) || i.comment.to_lowercase().contains(&q)
            }).collect()
        }
    }

    pub fn filtered_ip_routes(&self) -> Vec<&IpRoute> {
        if self.filter_query.is_empty() {
            self.ip_routes.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.ip_routes.iter().filter(|r| {
                r.dst_address.to_lowercase().contains(&q) || r.gateway.to_lowercase().contains(&q) || r.comment.to_lowercase().contains(&q)
            }).collect()
        }
    }

    pub fn filtered_dhcp_leases(&self) -> Vec<&DhcpLease> {
        if self.filter_query.is_empty() {
            self.dhcp_leases.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.dhcp_leases.iter().filter(|d| {
                d.address.to_lowercase().contains(&q) || d.host_name.to_lowercase().contains(&q) || d.mac_address.to_lowercase().contains(&q)
            }).collect()
        }
    }

    pub fn filtered_firewall_rules(&self) -> Vec<&FirewallRule> {
        if self.filter_query.is_empty() {
            self.firewall_rules.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.firewall_rules.iter().filter(|f| {
                f.chain.to_lowercase().contains(&q) || f.action.to_lowercase().contains(&q) || f.comment.to_lowercase().contains(&q)
            }).collect()
        }
    }

    pub fn filtered_neighbors(&self) -> Vec<&Neighbor> {
        if self.filter_query.is_empty() {
            self.neighbors.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.neighbors.iter().filter(|n| {
                n.interface.to_lowercase().contains(&q) || n.identity.to_lowercase().contains(&q) || n.ip_address.to_lowercase().contains(&q) || n.mac_address.to_lowercase().contains(&q) || n.board.to_lowercase().contains(&q)
            }).collect()
        }
    }

    pub fn filtered_logs(&self) -> Vec<&LogEntry> {
        if self.filter_query.is_empty() {
            self.logs.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.logs.iter().filter(|l| {
                l.message.to_lowercase().contains(&q) || l.topics.to_lowercase().contains(&q)
            }).collect()
        }
    }
}
