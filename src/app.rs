use crate::models::*;
use crate::ssh::{RouterClient, SshConfig};
use crate::ui::theme::Theme;
use anyhow::Result;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    System,
    Interfaces,
    IpAddresses,
    IpRoutes,
    DhcpLeases,
    Firewall,
    Logs,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::System,
        Tab::Interfaces,
        Tab::IpAddresses,
        Tab::IpRoutes,
        Tab::DhcpLeases,
        Tab::Firewall,
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
    pub ping_state: PingState,

    // Data models
    pub system_resource: SystemResource,
    pub interfaces: Vec<Interface>,
    pub ip_addresses: Vec<IpAddress>,
    pub ip_routes: Vec<IpRoute>,
    pub dhcp_leases: Vec<DhcpLease>,
    pub firewall_rules: Vec<FirewallRule>,
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
            ping_state: PingState::Inactive,
            system_resource: SystemResource::default(),
            interfaces: Vec::new(),
            ip_addresses: Vec::new(),
            ip_routes: Vec::new(),
            dhcp_leases: Vec::new(),
            firewall_rules: Vec::new(),
            logs: Vec::new(),
            status_message: "Connected. SAFE MODE ENABLED BY DEFAULT (Read-Only)".to_string(),
            is_loading: false,
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
            let logs = client.fetch_logs().await.ok();

            let _ = tx.send(AppEvent::DataLoaded {
                system,
                interfaces,
                ip_addresses,
                ip_routes,
                dhcp_leases,
                firewall_rules,
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
        logs: Option<Vec<LogEntry>>,
    ) {
        if let Some(res) = system { if !res.board_name.is_empty() || !res.version.is_empty() { self.system_resource = res; } }
        if let Some(ifaces) = interfaces { if !ifaces.is_empty() { self.interfaces = ifaces; } }
        if let Some(addrs) = ip_addresses { if !addrs.is_empty() { self.ip_addresses = addrs; } }
        if let Some(routes) = ip_routes { if !routes.is_empty() { self.ip_routes = routes; } }
        if let Some(dhcp) = dhcp_leases { if !dhcp.is_empty() { self.dhcp_leases = dhcp; } }
        if let Some(fw) = firewall_rules { if !fw.is_empty() { self.firewall_rules = fw; } }
        if let Some(l) = logs { if !l.is_empty() { self.logs = l; } }

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
