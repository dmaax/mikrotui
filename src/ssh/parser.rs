use std::collections::HashMap;
use crate::models::*;

/// Remove códigos de escape ANSI (ex: cores de terminal \x1b[32m) e caracteres de controle \r
pub fn strip_ansi_codes(input: &str) -> String {
    let mut clean = String::with_capacity(input.len());
    let mut in_escape = false;

    for c in input.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c != '\r' {
            clean.push(c);
        }
    }

    clean
}

/// Converte a saída de comandos do RouterOS CLI em lista de dicionários Chave-Valor.
pub fn parse_routeros_output(raw_input: &str) -> Vec<HashMap<String, String>> {
    let cleaned = strip_ansi_codes(raw_input);
    let lines: Vec<&str> = cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("Flags:") && !l.starts_with("#"))
        .collect();

    if lines.is_empty() {
        return Vec::new();
    }

    // Se for formato multilinha "key: value" (como no /system resource print do ROS v7)
    let is_colon_multiline = lines.iter().all(|l| l.contains(':') && !l.contains('=')) || lines.iter().any(|l| l.starts_with("uptime:"));

    if is_colon_multiline {
        let mut map = HashMap::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        return vec![map];
    }

    // Verifica se a saída está no formato key=value ou em formato tabular sem '='
    let has_key_value = lines.iter().any(|l| l.contains('='));

    if !has_key_value {
        return parse_tabular_output(&cleaned);
    }

    let mut items = Vec::new();
    let mut current_item: Option<HashMap<String, String>> = None;

    for line in lines {
        let is_new_item = line.chars().next().map_or(false, |c| c.is_ascii_digit());

        if is_new_item {
            if let Some(item) = current_item.take() {
                if !item.is_empty() {
                    items.push(item);
                }
            }
            let mut map = HashMap::new();

            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                map.insert(".id".to_string(), parts[0].to_string());
                if parts.len() > 1 && !parts[1].contains('=') {
                    let flags = parts[1];
                    if flags.contains('R') { map.insert("running".to_string(), "yes".to_string()); }
                    if flags.contains('X') { map.insert("disabled".to_string(), "yes".to_string()); }
                    if flags.contains('D') { map.insert("dynamic".to_string(), "yes".to_string()); }
                    if flags.contains('A') { map.insert("active".to_string(), "yes".to_string()); }
                    if flags.contains('I') { map.insert("invalid".to_string(), "yes".to_string()); }
                }
            }

            let pairs = parse_key_value_pairs(line);
            for (k, v) in pairs {
                map.insert(k, v);
            }
            current_item = Some(map);
        } else if let Some(ref mut map) = current_item {
            let pairs = parse_key_value_pairs(line);
            for (k, v) in pairs {
                map.insert(k, v);
            }
        }
    }

    if let Some(item) = current_item {
        if !item.is_empty() {
            items.push(item);
        }
    }

    items
}

fn parse_key_value_pairs(input: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();

    while chars.peek().is_some() {
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        if chars.peek().is_none() {
            break;
        }

        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            } else {
                key.push(c);
                chars.next();
            }
        }

        if chars.peek() == Some(&'=') {
            chars.next();

            let mut value = String::new();
            if chars.peek() == Some(&'"') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '"' {
                        break;
                    } else if c == '\\' {
                        if let Some(&escaped) = chars.peek() {
                            value.push(escaped);
                            chars.next();
                        }
                    } else {
                        value.push(c);
                    }
                }
            } else {
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    } else {
                        value.push(c);
                        chars.next();
                    }
                }
            }

            if !key.is_empty() {
                result.push((key, value));
            }
        }
    }

    result
}

fn parse_tabular_output(output: &str) -> Vec<HashMap<String, String>> {
    let mut items = Vec::new();
    let lines: Vec<&str> = output.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    for line in lines {
        if line.starts_with("Flags:") || line.starts_with("#") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
            let mut map = HashMap::new();
            map.insert(".id".to_string(), parts[0].to_string());

            if let Some(ip_idx) = parts.iter().enumerate().position(|(idx, p)| idx > 0 && p.contains('/')) {
                for p in &parts[1..ip_idx] {
                    if p.contains('D') { map.insert("dynamic".to_string(), "yes".to_string()); }
                    if p.contains('X') { map.insert("disabled".to_string(), "yes".to_string()); }
                    if p.contains('I') { map.insert("invalid".to_string(), "yes".to_string()); }
                }

                map.insert("address".to_string(), parts[ip_idx].to_string());

                if ip_idx + 1 < parts.len() {
                    map.insert("network".to_string(), parts[ip_idx + 1].to_string());
                }

                if ip_idx + 2 < parts.len() {
                    map.insert("interface".to_string(), parts[ip_idx + 2].to_string());
                }
            } else {
                if parts.len() > 1 { map.insert("address".to_string(), parts[1].to_string()); }
                if parts.len() > 2 { map.insert("network".to_string(), parts[2].to_string()); }
                if parts.len() > 3 { map.insert("interface".to_string(), parts[3].to_string()); }
            }

            items.push(map);
        }
    }

    items
}

pub fn parse_system_resource(raw: &str) -> SystemResource {
    let items = parse_routeros_output(raw);
    let mut res = SystemResource::default();
    if let Some(map) = items.first() {
        res.uptime = map.get("uptime").cloned().unwrap_or_else(|| "0s".to_string());
        res.version = map.get("version").cloned().unwrap_or_else(|| "N/A".to_string());
        res.build_time = map.get("build-time").cloned().unwrap_or_default();
        res.free_memory = map.get("free-memory").cloned().unwrap_or_default();
        res.total_memory = map.get("total-memory").cloned().unwrap_or_default();
        res.cpu = map.get("cpu").cloned().unwrap_or_default();
        res.cpu_count = map.get("cpu-count").cloned().unwrap_or_else(|| "1".to_string());
        res.cpu_frequency = map.get("cpu-frequency").cloned().unwrap_or_default();
        res.cpu_load = map.get("cpu-load").and_then(|v| v.trim_end_matches('%').parse().ok()).unwrap_or(0);
        res.free_hdd_space = map.get("free-hdd-space").cloned().unwrap_or_default();
        res.total_hdd_space = map.get("total-hdd-space").cloned().unwrap_or_default();
        res.architecture_name = map.get("architecture-name").cloned().unwrap_or_default();
        res.board_name = map.get("board-name").cloned().unwrap_or_default();
        res.platform = map.get("platform").cloned().unwrap_or_default();
    }
    res
}

pub fn parse_interfaces(raw: &str) -> Vec<Interface> {
    let items = parse_routeros_output(raw);
    items.into_iter().map(|map| {
        let name = map.get("name")
            .or_else(|| map.get("default-name"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let itype = map.get("type")
            .cloned()
            .unwrap_or_else(|| "ether".to_string());

        let rx_byte = map.get("rx-byte")
            .or_else(|| map.get("rx-bytes"))
            .or_else(|| map.get("bytes-received"))
            .or_else(|| map.get("rx-byte-count"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let tx_byte = map.get("tx-byte")
            .or_else(|| map.get("tx-bytes"))
            .or_else(|| map.get("bytes-sent"))
            .or_else(|| map.get("tx-byte-count"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let rx_packet = map.get("rx-packet")
            .or_else(|| map.get("rx-packets"))
            .or_else(|| map.get("packets-received"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let tx_packet = map.get("tx-packet")
            .or_else(|| map.get("tx-packets"))
            .or_else(|| map.get("packets-sent"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        Interface {
            id: map.get(".id").cloned().unwrap_or_default(),
            name,
            interface_type: itype,
            mtu: map.get("mtu").or_else(|| map.get("actual-mtu")).cloned().unwrap_or_else(|| "1500".to_string()),
            mac_address: map.get("mac-address").or_else(|| map.get("orig-mac-address")).cloned().unwrap_or_default(),
            running: map.get("running").map(|v| v == "yes" || v == "true" || v == "R").unwrap_or(false),
            disabled: map.get("disabled").map(|v| v == "yes" || v == "true" || v == "X").unwrap_or(false),
            rx_byte,
            tx_byte,
            rx_packet,
            tx_packet,
            comment: map.get("comment").cloned().unwrap_or_default(),
        }
    }).collect()
}

pub fn parse_ip_addresses(raw: &str) -> Vec<IpAddress> {
    let items = parse_routeros_output(raw);
    items.into_iter().filter_map(|map| {
        let address = map.get("address")
            .cloned()
            .or_else(|| {
                map.values().find(|v| v.contains('/') && (v.contains('.') || v.contains(':'))).cloned()
            })?;

        let interface = map.get("interface")
            .or_else(|| map.get("actual-interface"))
            .cloned()
            .unwrap_or_else(|| {
                map.get("network").cloned().unwrap_or_default()
            });

        Some(IpAddress {
            id: map.get(".id").cloned().unwrap_or_default(),
            address,
            network: map.get("network").cloned().unwrap_or_default(),
            interface,
            disabled: map.get("disabled").map(|v| v == "yes" || v == "true" || v == "X").unwrap_or(false),
            dynamic: map.get("dynamic").map(|v| v == "yes" || v == "true" || v == "D").unwrap_or(false),
            comment: map.get("comment").cloned().unwrap_or_default(),
        })
    }).collect()
}

pub fn parse_ip_routes(raw: &str) -> Vec<IpRoute> {
    let items = parse_routeros_output(raw);
    items.into_iter().map(|map| {
        IpRoute {
            id: map.get(".id").cloned().unwrap_or_default(),
            dst_address: map.get("dst-address").cloned().unwrap_or_default(),
            gateway: map.get("gateway").cloned().unwrap_or_default(),
            distance: map.get("distance").and_then(|v| v.parse().ok()).unwrap_or(1),
            routing_table: map.get("routing-table").cloned().unwrap_or_else(|| "main".to_string()),
            active: map.get("active").map(|v| v == "yes" || v == "true" || v == "A").unwrap_or(true),
            dynamic: map.get("dynamic").map(|v| v == "yes" || v == "true" || v == "D").unwrap_or(false),
            disabled: map.get("disabled").map(|v| v == "yes" || v == "true" || v == "X").unwrap_or(false),
            comment: map.get("comment").cloned().unwrap_or_default(),
        }
    }).collect()
}

pub fn parse_dhcp_leases(raw: &str) -> Vec<DhcpLease> {
    let items = parse_routeros_output(raw);
    items.into_iter().map(|map| {
        DhcpLease {
            id: map.get(".id").cloned().unwrap_or_default(),
            address: map.get("address").cloned().unwrap_or_default(),
            mac_address: map.get("mac-address").cloned().unwrap_or_default(),
            host_name: map.get("host-name").cloned().unwrap_or_default(),
            server: map.get("server").cloned().unwrap_or_default(),
            status: map.get("status").cloned().unwrap_or_else(|| "bound".to_string()),
            expires_after: map.get("expires-after").cloned().unwrap_or_default(),
            dynamic: map.get("dynamic").map(|v| v == "yes" || v == "true").unwrap_or(false),
            disabled: map.get("disabled").map(|v| v == "yes" || v == "true").unwrap_or(false),
        }
    }).collect()
}

pub fn parse_firewall_rules(raw: &str) -> Vec<FirewallRule> {
    let items = parse_routeros_output(raw);
    items.into_iter().map(|map| {
        FirewallRule {
            id: map.get(".id").cloned().unwrap_or_default(),
            chain: map.get("chain").cloned().unwrap_or_else(|| "input".to_string()),
            action: map.get("action").cloned().unwrap_or_else(|| "accept".to_string()),
            src_address: map.get("src-address").cloned().unwrap_or_default(),
            dst_address: map.get("dst-address").cloned().unwrap_or_default(),
            protocol: map.get("protocol").cloned().unwrap_or_default(),
            dst_port: map.get("dst-port").cloned().unwrap_or_default(),
            bytes: map.get("bytes").and_then(|v| v.parse().ok()).unwrap_or(0),
            packets: map.get("packets").and_then(|v| v.parse().ok()).unwrap_or(0),
            disabled: map.get("disabled").map(|v| v == "yes" || v == "true").unwrap_or(false),
            comment: map.get("comment").cloned().unwrap_or_default(),
        }
    }).collect()
}

pub fn parse_neighbors(raw: &str) -> Vec<Neighbor> {
    let items = parse_routeros_output(raw);
    items.into_iter().map(|map| {
        let interface = map.get("interface").cloned().unwrap_or_else(|| "unknown".to_string());
        let identity = map.get("identity")
            .or_else(|| map.get("system-caps"))
            .or_else(|| map.get("system-description"))
            .cloned()
            .unwrap_or_else(|| "Unknown-Device".to_string());

        Neighbor {
            id: map.get(".id").cloned().unwrap_or_default(),
            interface,
            identity,
            mac_address: map.get("mac-address").cloned().unwrap_or_default(),
            ip_address: map.get("address").or_else(|| map.get("ip-address")).cloned().unwrap_or_default(),
            platform: map.get("platform").cloned().unwrap_or_else(|| "MikroTik".to_string()),
            board: map.get("board").or_else(|| map.get("board-name")).cloned().unwrap_or_default(),
            version: map.get("version").or_else(|| map.get("software-version")).cloned().unwrap_or_default(),
        }
    }).collect()
}

pub fn parse_logs(raw: &str) -> Vec<LogEntry> {
    let cleaned = strip_ansi_codes(raw);
    let mut entries = Vec::new();
    for line in cleaned.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() == 3 {
            entries.push(LogEntry {
                time: parts[0].to_string(),
                topics: parts[1].to_string(),
                message: parts[2].to_string(),
            });
        } else {
            entries.push(LogEntry {
                time: "".to_string(),
                topics: "info".to_string(),
                message: line.to_string(),
            });
        }
    }
    entries
}

pub fn parse_ping_output(target: &str, raw: &str) -> PingResult {
    let cleaned = strip_ansi_codes(raw);
    let mut res = PingResult {
        target: target.to_string(),
        raw_output: cleaned.clone(),
        ..Default::default()
    };

    let mut seq_list = Vec::new();

    for line in cleaned.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("SEQ") || trimmed.starts_with("HOST") {
            continue;
        }

        if trimmed.contains("sent=") || trimmed.contains("packet-loss=") {
            let pairs = parse_key_value_pairs(trimmed);
            for (k, v) in pairs {
                match k.as_str() {
                    "sent" => res.sent = v.parse().unwrap_or(0),
                    "received" => res.received = v.parse().unwrap_or(0),
                    "packet-loss" => res.packet_loss_pct = v.trim_end_matches('%').parse().unwrap_or(0),
                    "min-rtt" => res.min_rtt_ms = v.trim_end_matches("ms").parse().unwrap_or(0),
                    "avg-rtt" => res.avg_rtt_ms = v.trim_end_matches("ms").parse().unwrap_or(0),
                    "max-rtt" => res.max_rtt_ms = v.trim_end_matches("ms").parse().unwrap_or(0),
                    _ => {}
                }
            }
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(seq) = parts[0].parse::<usize>() {
                let host = parts[1].to_string();
                let size = parts[2].parse::<usize>().unwrap_or(56);
                let ttl = parts[3].parse::<u32>().unwrap_or(64);
                let rtt_str = if parts.len() > 4 { parts[4] } else { "0ms" };
                let rtt_ms = rtt_str.trim_end_matches("ms").parse::<u32>().unwrap_or(0);
                let status = if parts.len() > 5 { parts[5].to_string() } else { "ok".to_string() };

                seq_list.push(PingSeq {
                    seq,
                    host,
                    size,
                    ttl,
                    rtt_ms,
                    status,
                });
            }
        }
    }

    if res.sent == 0 {
        res.sent = seq_list.len() as u32;
    }
    if res.received == 0 {
        res.received = seq_list.iter().filter(|s| s.status == "ok" || s.rtt_ms > 0).count() as u32;
    }
    res.sequences = seq_list;
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_parser() {
        let raw = "0 interface=ether3 identity=\"Switch-Core-01\" mac-address=DC:2C:6E:02:B0:10 address=192.168.88.2 platform=\"MikroTik\" board=\"CRS326-24G-2S+\" version=\"7.16.1 (stable)\"\n1 interface=ether4 identity=\"AP-Office-Floor2\" mac-address=48:A9:8A:11:22:33 address=192.168.88.5 platform=\"MikroTik\" board=\"cAP ac\" version=\"7.14.3 (stable)\"";
        let parsed = parse_neighbors(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].interface, "ether3");
        assert_eq!(parsed[0].identity, "Switch-Core-01");
        assert_eq!(parsed[0].mac_address, "DC:2C:6E:02:B0:10");
        assert_eq!(parsed[0].ip_address, "192.168.88.2");
        assert_eq!(parsed[0].board, "CRS326-24G-2S+");
        assert_eq!(parsed[1].identity, "AP-Office-Floor2");
        assert_eq!(parsed[1].board, "cAP ac");
    }

    #[test]
    fn test_ping_parser() {
        let raw = "  SEQ HOST SIZE TTL TIME STATUS\n    0 8.8.8.8 56 117 12ms ok\n    1 8.8.8.8 56 117 11ms ok\n  sent=5 received=5 packet-loss=0% min-rtt=11ms avg-rtt=12ms max-rtt=13ms";
        let res = parse_ping_output("8.8.8.8", raw);
        assert_eq!(res.target, "8.8.8.8");
        assert_eq!(res.sequences.len(), 2);
        assert_eq!(res.sequences[0].rtt_ms, 12);
        assert_eq!(res.sequences[1].rtt_ms, 11);
        assert_eq!(res.sent, 5);
        assert_eq!(res.received, 5);
        assert_eq!(res.packet_loss_pct, 0);
        assert_eq!(res.min_rtt_ms, 11);
        assert_eq!(res.avg_rtt_ms, 12);
        assert_eq!(res.max_rtt_ms, 13);
    }

    #[test]
    fn test_system_resource_colon_parser() {
        let raw = "   uptime: 6d13h9m55s\n  version: 7.16.1 (stable)\n cpu-load: 7%\n board-name: CRS326-24G-2S+";
        let res = parse_system_resource(raw);
        assert_eq!(res.uptime, "6d13h9m55s");
        assert_eq!(res.version, "7.16.1 (stable)");
        assert_eq!(res.cpu_load, 7);
        assert_eq!(res.board_name, "CRS326-24G-2S+");
    }

    #[test]
    fn test_ip_address_tabular_parser() {
        let raw = "\x1b[0mFlags: X - disabled, I - invalid, D - dynamic \n 0 D 192.168.88.1/24    192.168.88.0    bridge\n 1   100.66.5.3/24       100.66.5.0      ether1";
        let parsed = parse_ip_addresses(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].address, "192.168.88.1/24");
        assert_eq!(parsed[0].interface, "bridge");
        assert_eq!(parsed[1].address, "100.66.5.3/24");
        assert_eq!(parsed[1].interface, "ether1");
    }

    #[test]
    fn test_terse_interface_parser() {
        let raw = "0 R name=ether1 default-name=ether1 type=ether mtu=1500 mac-address=DC:2C:6E:01:A0:01 comment=\"WAN Port\"\n1   name=ether2 type=ether mtu=1500 mac-address=DC:2C:6E:01:A0:02";
        let parsed = parse_interfaces(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "ether1");
        assert_eq!(parsed[0].comment, "WAN Port");
        assert!(parsed[0].running);
        assert_eq!(parsed[1].name, "ether2");
        assert!(!parsed[1].running);
    }
}
