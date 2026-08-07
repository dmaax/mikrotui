use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use russh::{client, ChannelMsg};
use russh_keys::key;
use crate::models::*;
use crate::ssh::parser;

#[derive(Clone, Debug)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: Option<String>,
    #[allow(dead_code)]
    pub key_path: Option<String>,
    pub demo_mode: bool,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            host: "192.168.88.1".to_string(),
            port: 22,
            user: "admin".to_string(),
            pass: None,
            key_path: None,
            demo_mode: true,
        }
    }
}

struct SshHandler;

#[async_trait::async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true) // Confia na chave do servidor MikroTik
    }
}

#[derive(Clone)]
pub struct RouterClient {
    pub config: SshConfig,
    pub is_connected: Arc<Mutex<bool>>,
    session: Arc<Mutex<Option<client::Handle<SshHandler>>>>,
}

impl RouterClient {
    pub fn new(config: SshConfig) -> Self {
        Self {
            config,
            is_connected: Arc::new(Mutex::new(false)),
            session: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        if self.config.demo_mode {
            let mut conn_lock = self.is_connected.lock().await;
            *conn_lock = true;
            return Ok(());
        }

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(60)),
            ..Default::default()
        });

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let mut handle = tokio::time::timeout(
            Duration::from_secs(10),
            client::connect(config, addr.as_str(), SshHandler),
        )
        .await
        .map_err(|_| anyhow!("Tempo limite de conexão SSH excedido ao tentar conectar em {}", addr))??;

        let pass = self.config.pass.as_deref().unwrap_or("");
        let auth_res = handle
            .authenticate_password(&self.config.user, pass)
            .await?;

        if !auth_res {
            return Err(anyhow!("Falha na autenticação SSH para usuário '{}' em {}", self.config.user, addr));
        }

        let mut conn_lock = self.is_connected.lock().await;
        *conn_lock = true;
        let mut sess_lock = self.session.lock().await;
        *sess_lock = Some(handle);

        Ok(())
    }

    pub async fn fetch_system_resource(&self) -> Result<SystemResource> {
        if self.config.demo_mode {
            return Ok(SystemResource {
                uptime: "4w2d18h42m".to_string(),
                version: "7.14.3 (stable)".to_string(),
                build_time: "2024-04-12 11:20:04".to_string(),
                free_memory: "214.5MiB".to_string(),
                total_memory: "256.0MiB".to_string(),
                cpu: "ARMv7".to_string(),
                cpu_count: "4".to_string(),
                cpu_frequency: "896MHz".to_string(),
                cpu_load: 12,
                free_hdd_space: "89.2MiB".to_string(),
                total_hdd_space: "128.0MiB".to_string(),
                architecture_name: "arm".to_string(),
                board_name: "hEX S".to_string(),
                platform: "MikroTik".to_string(),
            });
        }

        let mut raw = self.exec_command("/system resource print").await?;
        if raw.contains("expected end of command") || raw.is_empty() {
            raw = self.exec_command("/system/resource/print").await?;
        }
        Ok(parser::parse_system_resource(&raw))
    }

    pub async fn fetch_interfaces(&self) -> Result<Vec<Interface>> {
        if self.config.demo_mode {
            return Ok(vec![
                Interface {
                    id: "0".to_string(),
                    name: "ether1-WAN".to_string(),
                    interface_type: "ether".to_string(),
                    mtu: "1500".to_string(),
                    mac_address: "DC:2C:6E:01:A0:01".to_string(),
                    running: true,
                    disabled: false,
                    rx_byte: 1458920412,
                    tx_byte: 349102488,
                    rx_packet: 1284900,
                    tx_packet: 894100,
                    comment: "ISP Connection".to_string(),
                },
                Interface {
                    id: "1".to_string(),
                    name: "ether2-LAN".to_string(),
                    interface_type: "ether".to_string(),
                    mtu: "1500".to_string(),
                    mac_address: "DC:2C:6E:01:A0:02".to_string(),
                    running: true,
                    disabled: false,
                    rx_byte: 984021948,
                    tx_byte: 1824091490,
                    rx_packet: 924100,
                    tx_packet: 1492000,
                    comment: "Local Bridge Trunk".to_string(),
                },
                Interface {
                    id: "2".to_string(),
                    name: "ether3-Server".to_string(),
                    interface_type: "ether".to_string(),
                    mtu: "1500".to_string(),
                    mac_address: "DC:2C:6E:01:A0:03".to_string(),
                    running: true,
                    disabled: false,
                    rx_byte: 45019280,
                    tx_byte: 120491820,
                    rx_packet: 54100,
                    tx_packet: 102400,
                    comment: "NAS & Server Port".to_string(),
                },
                Interface {
                    id: "3".to_string(),
                    name: "wireguard1".to_string(),
                    interface_type: "wireguard".to_string(),
                    mtu: "1420".to_string(),
                    mac_address: "".to_string(),
                    running: true,
                    disabled: false,
                    rx_byte: 5410928,
                    tx_byte: 9104812,
                    rx_packet: 12900,
                    tx_packet: 20400,
                    comment: "Site-to-Site VPN".to_string(),
                },
            ]);
        }

        let raw = self.exec_command("/interface print terse without-paging").await?;
        let mut parsed = parser::parse_interfaces(&raw);

        if parsed.is_empty() {
            let raw_detail = self.exec_command("/interface print detail without-paging").await?;
            parsed = parser::parse_interfaces(&raw_detail);
        }

        if parsed.is_empty() {
            let raw_simple = self.exec_command("/interface print without-paging").await?;
            parsed = parser::parse_interfaces(&raw_simple);
        }

        Ok(parsed)
    }

    pub async fn fetch_ip_addresses(&self) -> Result<Vec<IpAddress>> {
        if self.config.demo_mode {
            return Ok(vec![
                IpAddress {
                    id: "0".to_string(),
                    address: "192.168.88.1/24".to_string(),
                    network: "192.168.88.0".to_string(),
                    interface: "ether2-LAN".to_string(),
                    disabled: false,
                    dynamic: false,
                    comment: "Default Gateway".to_string(),
                },
                IpAddress {
                    id: "1".to_string(),
                    address: "203.0.113.45/30".to_string(),
                    network: "203.0.113.44".to_string(),
                    interface: "ether1-WAN".to_string(),
                    disabled: false,
                    dynamic: true,
                    comment: "Public Static IP".to_string(),
                },
                IpAddress {
                    id: "2".to_string(),
                    address: "10.10.0.1/24".to_string(),
                    network: "10.10.0.0".to_string(),
                    interface: "wireguard1".to_string(),
                    disabled: false,
                    dynamic: false,
                    comment: "VPN Subnet".to_string(),
                },
            ]);
        }

        let raw = self.exec_command("/ip address print terse without-paging").await?;
        let mut parsed = parser::parse_ip_addresses(&raw);

        if parsed.is_empty() {
            let raw_detail = self.exec_command("/ip address print detail without-paging").await?;
            parsed = parser::parse_ip_addresses(&raw_detail);
        }

        if parsed.is_empty() {
            let raw_simple = self.exec_command("/ip address print without-paging").await?;
            parsed = parser::parse_ip_addresses(&raw_simple);
        }

        Ok(parsed)
    }

    pub async fn fetch_ip_routes(&self) -> Result<Vec<IpRoute>> {
        if self.config.demo_mode {
            return Ok(vec![
                IpRoute {
                    id: "0".to_string(),
                    dst_address: "0.0.0.0/0".to_string(),
                    gateway: "203.0.113.46".to_string(),
                    distance: 1,
                    routing_table: "main".to_string(),
                    active: true,
                    dynamic: true,
                    disabled: false,
                    comment: "Default Internet Route".to_string(),
                },
                IpRoute {
                    id: "1".to_string(),
                    dst_address: "192.168.88.0/24".to_string(),
                    gateway: "ether2-LAN".to_string(),
                    distance: 0,
                    routing_table: "main".to_string(),
                    active: true,
                    dynamic: false,
                    disabled: false,
                    comment: "LAN Connected".to_string(),
                },
                IpRoute {
                    id: "2".to_string(),
                    dst_address: "10.10.0.0/24".to_string(),
                    gateway: "wireguard1".to_string(),
                    distance: 1,
                    routing_table: "main".to_string(),
                    active: true,
                    dynamic: false,
                    disabled: false,
                    comment: "Remote Branch VPN".to_string(),
                },
            ]);
        }

        let raw = self.exec_command("/ip route print terse without-paging").await?;
        let mut parsed = parser::parse_ip_routes(&raw);

        if parsed.is_empty() {
            let raw_detail = self.exec_command("/ip route print detail without-paging").await?;
            parsed = parser::parse_ip_routes(&raw_detail);
        }

        if parsed.is_empty() {
            let raw_simple = self.exec_command("/ip route print without-paging").await?;
            parsed = parser::parse_ip_routes(&raw_simple);
        }

        Ok(parsed)
    }

    pub async fn fetch_dhcp_leases(&self) -> Result<Vec<DhcpLease>> {
        if self.config.demo_mode {
            return Ok(vec![
                DhcpLease {
                    id: "0".to_string(),
                    address: "192.168.88.100".to_string(),
                    mac_address: "A4:83:E7:12:45:90".to_string(),
                    host_name: "Eduardo-Workstation".to_string(),
                    server: "defconf".to_string(),
                    status: "bound".to_string(),
                    expires_after: "23h54m".to_string(),
                    dynamic: true,
                    disabled: false,
                },
                DhcpLease {
                    id: "1".to_string(),
                    address: "192.168.88.105".to_string(),
                    mac_address: "BC:D1:D3:88:99:11".to_string(),
                    host_name: "Smart-TV-LivingRoom".to_string(),
                    server: "defconf".to_string(),
                    status: "bound".to_string(),
                    expires_after: "18h12m".to_string(),
                    dynamic: true,
                    disabled: false,
                },
                DhcpLease {
                    id: "2".to_string(),
                    address: "192.168.88.200".to_string(),
                    mac_address: "00:11:32:44:55:66".to_string(),
                    host_name: "Synology-NAS".to_string(),
                    server: "defconf".to_string(),
                    status: "bound".to_string(),
                    expires_after: "infinite".to_string(),
                    dynamic: false,
                    disabled: false,
                },
            ]);
        }

        let raw = self.exec_command("/ip dhcp-server lease print terse without-paging").await?;
        let mut parsed = parser::parse_dhcp_leases(&raw);

        if parsed.is_empty() {
            let raw_detail = self.exec_command("/ip dhcp-server lease print detail without-paging").await?;
            parsed = parser::parse_dhcp_leases(&raw_detail);
        }

        if parsed.is_empty() {
            let raw_simple = self.exec_command("/ip dhcp-server lease print without-paging").await?;
            parsed = parser::parse_dhcp_leases(&raw_simple);
        }

        Ok(parsed)
    }

    pub async fn fetch_firewall_rules(&self) -> Result<Vec<FirewallRule>> {
        if self.config.demo_mode {
            return Ok(vec![
                FirewallRule {
                    id: "0".to_string(),
                    chain: "input".to_string(),
                    action: "accept".to_string(),
                    src_address: "".to_string(),
                    dst_address: "".to_string(),
                    protocol: "icmp".to_string(),
                    dst_port: "".to_string(),
                    bytes: 8491024,
                    packets: 98120,
                    disabled: false,
                    comment: "defconf: accept ICMP".to_string(),
                },
                FirewallRule {
                    id: "1".to_string(),
                    chain: "input".to_string(),
                    action: "accept".to_string(),
                    src_address: "192.168.88.0/24".to_string(),
                    dst_address: "".to_string(),
                    protocol: "tcp".to_string(),
                    dst_port: "22,80,443,8291".to_string(),
                    bytes: 194810240,
                    packets: 1490210,
                    disabled: false,
                    comment: "Allow Management Ports from LAN".to_string(),
                },
                FirewallRule {
                    id: "2".to_string(),
                    chain: "input".to_string(),
                    action: "drop".to_string(),
                    src_address: "".to_string(),
                    dst_address: "".to_string(),
                    protocol: "".to_string(),
                    dst_port: "".to_string(),
                    bytes: 49102400,
                    packets: 489100,
                    disabled: false,
                    comment: "defconf: drop all not coming from LAN".to_string(),
                },
            ]);
        }

        let raw = self.exec_command("/ip firewall filter print terse without-paging").await?;
        let mut parsed = parser::parse_firewall_rules(&raw);

        if parsed.is_empty() {
            let raw_detail = self.exec_command("/ip firewall filter print detail without-paging").await?;
            parsed = parser::parse_firewall_rules(&raw_detail);
        }

        if parsed.is_empty() {
            let raw_simple = self.exec_command("/ip firewall filter print without-paging").await?;
            parsed = parser::parse_firewall_rules(&raw_simple);
        }

        Ok(parsed)
    }

    pub async fn fetch_neighbors(&self) -> Result<Vec<Neighbor>> {
        if self.config.demo_mode {
            return Ok(vec![
                Neighbor {
                    id: "0".to_string(),
                    interface: "ether3".to_string(),
                    identity: "Switch-Core-01".to_string(),
                    mac_address: "DC:2C:6E:02:B0:10".to_string(),
                    ip_address: "192.168.88.2".to_string(),
                    platform: "MikroTik".to_string(),
                    board: "CRS326-24G-2S+".to_string(),
                    version: "7.16.1 (stable)".to_string(),
                },
                Neighbor {
                    id: "1".to_string(),
                    interface: "ether4".to_string(),
                    identity: "AP-Office-Floor2".to_string(),
                    mac_address: "48:A9:8A:11:22:33".to_string(),
                    ip_address: "192.168.88.5".to_string(),
                    platform: "MikroTik".to_string(),
                    board: "cAP ac".to_string(),
                    version: "7.14.3 (stable)".to_string(),
                },
                Neighbor {
                    id: "2".to_string(),
                    interface: "ether1-WAN".to_string(),
                    identity: "Gateway-ISP".to_string(),
                    mac_address: "00:1B:17:88:99:AA".to_string(),
                    ip_address: "203.0.113.46".to_string(),
                    platform: "Cisco".to_string(),
                    board: "ASR1001-X".to_string(),
                    version: "IOS-XE 17.3".to_string(),
                },
            ]);
        }

        let raw = self.exec_command("/ip neighbor print terse without-paging").await?;
        let mut parsed = parser::parse_neighbors(&raw);

        if parsed.is_empty() {
            let raw_detail = self.exec_command("/ip neighbor print detail without-paging").await?;
            parsed = parser::parse_neighbors(&raw_detail);
        }

        if parsed.is_empty() {
            let raw_simple = self.exec_command("/ip neighbor print without-paging").await?;
            parsed = parser::parse_neighbors(&raw_simple);
        }

        Ok(parsed)
    }

    pub async fn fetch_logs(&self) -> Result<Vec<LogEntry>> {
        if self.config.demo_mode {
            return Ok(vec![
                LogEntry {
                    time: "14:10:01".to_string(),
                    topics: "system,info".to_string(),
                    message: "router rebooted".to_string(),
                },
                LogEntry {
                    time: "14:10:05".to_string(),
                    topics: "interface,info".to_string(),
                    message: "ether1-WAN link up (1000M Full)".to_string(),
                },
                LogEntry {
                    time: "14:10:08".to_string(),
                    topics: "dhcp,info".to_string(),
                    message: "assigned 192.168.88.100 to Eduardo-Workstation".to_string(),
                },
                LogEntry {
                    time: "14:12:30".to_string(),
                    topics: "ssh,info".to_string(),
                    message: "user admin logged in from 192.168.88.100 via SSH".to_string(),
                },
                LogEntry {
                    time: "14:13:00".to_string(),
                    topics: "system,info,safe-mode".to_string(),
                    message: "Safe Mode active for session (Read-Only Mode Enforcement)".to_string(),
                },
            ]);
        }

        let raw = self.exec_command("/log print follow=no").await?;
        Ok(parser::parse_logs(&raw))
    }

    pub async fn run_ping(&self, target: &str, count: u32) -> Result<PingResult> {
        if self.config.demo_mode {
            return Ok(PingResult {
                target: target.to_string(),
                sent: count,
                received: count,
                packet_loss_pct: 0,
                min_rtt_ms: 8,
                avg_rtt_ms: 11,
                max_rtt_ms: 14,
                sequences: vec![
                    PingSeq { seq: 0, host: target.to_string(), size: 56, ttl: 117, rtt_ms: 10, status: "ok".to_string() },
                    PingSeq { seq: 1, host: target.to_string(), size: 56, ttl: 117, rtt_ms: 12, status: "ok".to_string() },
                    PingSeq { seq: 2, host: target.to_string(), size: 56, ttl: 117, rtt_ms: 11, status: "ok".to_string() },
                    PingSeq { seq: 3, host: target.to_string(), size: 56, ttl: 117, rtt_ms: 8, status: "ok".to_string() },
                    PingSeq { seq: 4, host: target.to_string(), size: 56, ttl: 117, rtt_ms: 14, status: "ok".to_string() },
                ],
                raw_output: "Demo Mode Ping".to_string(),
            });
        }

        let cmd = format!("/ping {} count={}", target, count);
        let raw = self.exec_command(&cmd).await?;
        Ok(parser::parse_ping_output(target, &raw))
    }

    pub async fn exec_command(&self, cmd: &str) -> Result<String> {
        let trimmed = cmd.trim();
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();

        for t in tokens {
            if t == "add" || t == "set" || t == "remove" || t == "enable" || t == "disable" {
                return Err(anyhow!("READ-ONLY ENFORCED: Write operations are disabled in MikroTUI Phase 1!"));
            }
        }

        match self.try_exec_command(cmd).await {
            Ok(output) if !output.is_empty() => Ok(output),
            _ => {
                {
                    let mut conn_lock = self.is_connected.lock().await;
                    *conn_lock = false;
                }
                self.connect().await?;
                self.try_exec_command(cmd).await
            }
        }
    }

    async fn try_exec_command(&self, cmd: &str) -> Result<String> {
        let is_conn = *self.is_connected.lock().await;
        if !is_conn {
            self.connect().await?;
        }

        let mut sess_guard = self.session.lock().await;
        let handle = sess_guard
            .as_mut()
            .ok_or_else(|| anyhow!("Sessão SSH não foi inicializada"))?;

        let mut channel = handle.channel_open_session().await?;
        channel.exec(true, cmd).await?;

        let mut output = Vec::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    output.extend_from_slice(&data);
                }
                ChannelMsg::ExtendedData { data, ext: _ } => {
                    output.extend_from_slice(&data);
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok(String::from_utf8_lossy(&output).to_string())
    }
}
