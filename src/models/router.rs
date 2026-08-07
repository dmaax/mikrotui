use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemResource {
    pub uptime: String,
    pub version: String,
    pub build_time: String,
    pub free_memory: String,
    pub total_memory: String,
    pub cpu: String,
    pub cpu_count: String,
    pub cpu_frequency: String,
    pub cpu_load: u8,
    pub free_hdd_space: String,
    pub total_hdd_space: String,
    pub architecture_name: String,
    pub board_name: String,
    pub platform: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Interface {
    pub id: String,
    pub name: String,
    pub interface_type: String,
    pub mtu: String,
    pub mac_address: String,
    pub running: bool,
    pub disabled: bool,
    pub rx_byte: u64,
    pub tx_byte: u64,
    pub rx_packet: u64,
    pub tx_packet: u64,
    pub comment: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpAddress {
    pub id: String,
    pub address: String,
    pub network: String,
    pub interface: String,
    pub disabled: bool,
    pub dynamic: bool,
    pub comment: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpRoute {
    pub id: String,
    pub dst_address: String,
    pub gateway: String,
    pub distance: u32,
    pub routing_table: String,
    pub active: bool,
    pub dynamic: bool,
    pub disabled: bool,
    pub comment: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DhcpLease {
    pub id: String,
    pub address: String,
    pub mac_address: String,
    pub host_name: String,
    pub server: String,
    pub status: String,
    pub expires_after: String,
    pub dynamic: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FirewallRule {
    pub id: String,
    pub chain: String,
    pub action: String,
    pub src_address: String,
    pub dst_address: String,
    pub protocol: String,
    pub dst_port: String,
    pub bytes: u64,
    pub packets: u64,
    pub disabled: bool,
    pub comment: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: String,
    pub topics: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingSeq {
    pub seq: usize,
    pub host: String,
    pub size: usize,
    pub ttl: u32,
    pub rtt_ms: u32,
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingResult {
    pub target: String,
    pub sequences: Vec<PingSeq>,
    pub sent: u32,
    pub received: u32,
    pub packet_loss_pct: u32,
    pub min_rtt_ms: u32,
    pub avg_rtt_ms: u32,
    pub max_rtt_ms: u32,
    pub raw_output: String,
}
