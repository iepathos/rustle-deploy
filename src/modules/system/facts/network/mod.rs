//! Network fact collection

use crate::modules::system::facts::{
    DefaultInterface, FactError, InterfaceFacts, InterfaceIPv4, InterfaceIPv6,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::Ipv4Addr;

pub struct NetworkCollector;

impl Default for NetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect_network_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Get hostname information
        facts.extend(self.collect_hostname_facts().await?);

        // Get network interfaces
        let interfaces = self.collect_network_interfaces().await?;

        // Extract IP addresses and interface names
        let mut all_ipv4_addresses = Vec::new();
        let mut all_ipv6_addresses = Vec::new();
        let mut interface_names = Vec::new();

        for (name, interface) in &interfaces {
            interface_names.push(name.clone());

            if let Some(ipv4) = &interface.ipv4 {
                all_ipv4_addresses.push(ipv4.address.clone());
            }

            for ipv6 in &interface.ipv6 {
                all_ipv6_addresses.push(ipv6.address.clone());
            }
        }

        facts.insert("ansible_interfaces".to_string(), json!(interface_names));
        facts.insert(
            "ansible_all_ipv4_addresses".to_string(),
            json!(all_ipv4_addresses),
        );
        facts.insert(
            "ansible_all_ipv6_addresses".to_string(),
            json!(all_ipv6_addresses),
        );

        // Try to determine default interfaces
        if let Some(default_ipv4) = self.detect_default_ipv4_interface(&interfaces).await {
            facts.insert("ansible_default_ipv4".to_string(), json!(default_ipv4));
        }

        if let Some(default_ipv6) = self.detect_default_ipv6_interface(&interfaces).await {
            facts.insert("ansible_default_ipv6".to_string(), json!(default_ipv6));
        }

        Ok(facts)
    }

    async fn collect_hostname_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Get short hostname
        if let Ok(hostname) = hostname::get() {
            if let Ok(hostname_str) = hostname.into_string() {
                facts.insert("ansible_hostname".to_string(), json!(hostname_str.clone()));

                // Try to get FQDN
                let fqdn = self.get_fqdn(&hostname_str).await;
                facts.insert("ansible_fqdn".to_string(), json!(fqdn.clone()));

                // Extract domain from FQDN
                let domain = if fqdn.contains('.') {
                    fqdn.split_once('.').map(|x| x.1).unwrap_or("").to_string()
                } else {
                    "".to_string()
                };
                facts.insert("ansible_domain".to_string(), json!(domain));
            }
        }

        Ok(facts)
    }

    async fn get_fqdn(&self, hostname: &str) -> String {
        // Try to resolve FQDN using hostname command
        if let Ok(output) = tokio::process::Command::new("hostname")
            .arg("-f")
            .output()
            .await
        {
            if output.status.success() {
                let fqdn = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !fqdn.is_empty() && fqdn != hostname {
                    return fqdn;
                }
            }
        }

        // Fallback to short hostname
        hostname.to_string()
    }

    async fn collect_network_interfaces(
        &self,
    ) -> Result<HashMap<String, InterfaceFacts>, FactError> {
        #[cfg(unix)]
        {
            self.collect_unix_interfaces().await
        }
        #[cfg(windows)]
        {
            self.collect_windows_interfaces().await
        }
    }

    #[cfg(unix)]
    async fn collect_unix_interfaces(&self) -> Result<HashMap<String, InterfaceFacts>, FactError> {
        let mut interfaces = HashMap::new();

        // Try ifconfig first
        if let Ok(output) = tokio::process::Command::new("ifconfig").output().await {
            if output.status.success() {
                let ifconfig_output = String::from_utf8_lossy(&output.stdout);
                interfaces.extend(self.parse_ifconfig(&ifconfig_output)?);
            }
        }

        // On Linux, also try ip command as fallback
        #[cfg(target_os = "linux")]
        if interfaces.is_empty() {
            if let Ok(output) = tokio::process::Command::new("ip")
                .args(&["addr", "show"])
                .output()
                .await
            {
                if output.status.success() {
                    let ip_output = String::from_utf8_lossy(&output.stdout);
                    interfaces.extend(self.parse_ip_addr(&ip_output)?);
                }
            }
        }

        Ok(interfaces)
    }

    #[cfg(windows)]
    async fn collect_windows_interfaces(
        &self,
    ) -> Result<HashMap<String, InterfaceFacts>, FactError> {
        let mut interfaces = HashMap::new();

        // Use PowerShell to get network adapter information
        if let Ok(output) = tokio::process::Command::new("powershell")
            .arg("-Command")
            .arg("Get-NetAdapter | Get-NetIPAddress | ConvertTo-Json")
            .output()
            .await
        {
            if output.status.success() {
                let json_output = String::from_utf8_lossy(&output.stdout);
                interfaces.extend(self.parse_windows_network_adapters(&json_output)?);
            }
        }

        Ok(interfaces)
    }

    fn parse_ifconfig(
        &self,
        ifconfig_output: &str,
    ) -> Result<HashMap<String, InterfaceFacts>, FactError> {
        let mut interfaces = HashMap::new();
        let mut current_interface: Option<String> = None;
        let mut current_facts = InterfaceFacts {
            device: String::new(),
            active: false,
            type_: "unknown".to_string(),
            macaddress: None,
            mtu: None,
            ipv4: None,
            ipv6: Vec::new(),
        };

        for line in ifconfig_output.lines() {
            let line = line.trim();

            // New interface starts at beginning of line
            if !line.starts_with(' ') && !line.starts_with('\t') && line.contains(':') {
                // Save previous interface
                if let Some(ref name) = current_interface {
                    interfaces.insert(name.clone(), current_facts.clone());
                }

                // Start new interface
                let interface_name = line.split(':').next().unwrap_or("").trim().to_string();
                current_interface = Some(interface_name.clone());
                current_facts = InterfaceFacts {
                    device: interface_name,
                    active: line.contains("UP"),
                    type_: self.determine_interface_type(line),
                    macaddress: None,
                    mtu: None,
                    ipv4: None,
                    ipv6: Vec::new(),
                };
            } else if line.contains("inet ") && !line.contains("inet6") {
                // IPv4 address
                if let Some(ipv4_info) = self.parse_inet_line(line) {
                    current_facts.ipv4 = Some(ipv4_info);
                }
            } else if line.contains("inet6") {
                // IPv6 address
                if let Some(ipv6_info) = self.parse_inet6_line(line) {
                    current_facts.ipv6.push(ipv6_info);
                }
            } else if line.contains("ether") || line.contains("HWaddr") {
                // MAC address
                current_facts.macaddress = self.extract_mac_address(line);
            } else if line.contains("mtu") || line.contains("MTU") {
                // MTU
                current_facts.mtu = self.extract_mtu(line);
            }
        }

        // Save last interface
        if let Some(ref name) = current_interface {
            interfaces.insert(name.clone(), current_facts);
        }

        Ok(interfaces)
    }

    #[cfg(target_os = "linux")]
    fn parse_ip_addr(&self, ip_output: &str) -> Result<HashMap<String, InterfaceFacts>, FactError> {
        let mut interfaces = HashMap::new();
        let mut current_interface: Option<String> = None;
        let mut current_facts = InterfaceFacts {
            device: String::new(),
            active: false,
            type_: "unknown".to_string(),
            macaddress: None,
            mtu: None,
            ipv4: None,
            ipv6: Vec::new(),
        };

        for line in ip_output.lines() {
            let line = line.trim();

            if let Some(colon_pos) = line.find(':') {
                if line.chars().next().unwrap_or(' ').is_ascii_digit() {
                    // New interface
                    if let Some(ref name) = current_interface {
                        interfaces.insert(name.clone(), current_facts.clone());
                    }

                    let parts: Vec<&str> = line[..colon_pos].split_whitespace().collect();
                    if parts.len() >= 2 {
                        let interface_name = parts[1].trim_end_matches(':').to_string();
                        current_interface = Some(interface_name.clone());
                        current_facts = InterfaceFacts {
                            device: interface_name,
                            active: line.contains("UP"),
                            type_: "ether".to_string(),
                            macaddress: None,
                            mtu: None,
                            ipv4: None,
                            ipv6: Vec::new(),
                        };
                    }
                }
            } else if line.starts_with("inet ") {
                // IPv4 address
                if let Some(ipv4_info) = self.parse_inet_line(line) {
                    current_facts.ipv4 = Some(ipv4_info);
                }
            } else if line.starts_with("inet6 ") {
                // IPv6 address
                if let Some(ipv6_info) = self.parse_inet6_line(line) {
                    current_facts.ipv6.push(ipv6_info);
                }
            }
        }

        // Save last interface
        if let Some(ref name) = current_interface {
            interfaces.insert(name.clone(), current_facts);
        }

        Ok(interfaces)
    }

    #[cfg(windows)]
    fn parse_windows_network_adapters(
        &self,
        json_output: &str,
    ) -> Result<HashMap<String, InterfaceFacts>, FactError> {
        // Simplified Windows network parsing
        // In a real implementation, this would parse the PowerShell JSON output
        let mut interfaces = HashMap::new();

        // This is a placeholder implementation
        interfaces.insert(
            "Local Area Connection".to_string(),
            InterfaceFacts {
                device: "Local Area Connection".to_string(),
                active: true,
                type_: "ether".to_string(),
                macaddress: None,
                mtu: Some(1500),
                ipv4: None,
                ipv6: Vec::new(),
            },
        );

        Ok(interfaces)
    }

    fn parse_inet_line(&self, line: &str) -> Option<InterfaceIPv4> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut address = None;
        let mut netmask = None;
        let mut broadcast = None;

        for (i, part) in parts.iter().enumerate() {
            if *part == "inet" && i + 1 < parts.len() {
                address = Some(parts[i + 1].to_string());
            } else if *part == "netmask" && i + 1 < parts.len() {
                netmask = Some(parts[i + 1].to_string());
            } else if *part == "broadcast" && i + 1 < parts.len() {
                broadcast = Some(parts[i + 1].to_string());
            }
        }

        if let (Some(addr), Some(mask)) = (address, netmask) {
            let network = self.calculate_network(&addr, &mask);
            Some(InterfaceIPv4 {
                address: addr,
                netmask: mask,
                network,
                broadcast,
            })
        } else {
            None
        }
    }

    fn parse_inet6_line(&self, line: &str) -> Option<InterfaceIPv6> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        for (i, part) in parts.iter().enumerate() {
            if (*part == "inet6" || *part == "inet6:") && i + 1 < parts.len() {
                let addr_part = parts[i + 1];
                if let Some(addr) = addr_part.split('/').next() {
                    let prefix = addr_part
                        .split('/')
                        .nth(1)
                        .and_then(|p| p.parse::<u8>().ok())
                        .unwrap_or(64);

                    let scope = if line.contains("link") {
                        "link".to_string()
                    } else if line.contains("global") {
                        "global".to_string()
                    } else {
                        "unknown".to_string()
                    };

                    return Some(InterfaceIPv6 {
                        address: addr.to_string(),
                        prefix,
                        scope,
                    });
                }
            }
        }

        None
    }

    fn determine_interface_type(&self, line: &str) -> String {
        if line.contains("LOOPBACK") {
            "loopback".to_string()
        } else if line.contains("POINTOPOINT") {
            "tunnel".to_string()
        } else {
            "ether".to_string()
        }
    }

    fn extract_mac_address(&self, line: &str) -> Option<String> {
        // Look for MAC address pattern
        let mac_regex = regex::Regex::new(r"([0-9a-fA-F]{2}[:-]){5}[0-9a-fA-F]{2}").ok()?;
        mac_regex.find(line).map(|m| m.as_str().to_string())
    }

    fn extract_mtu(&self, line: &str) -> Option<u32> {
        // Look for MTU value
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if part.to_lowercase() == "mtu" && i + 1 < parts.len() {
                return parts[i + 1].parse::<u32>().ok();
            }
        }
        None
    }

    fn calculate_network(&self, address: &str, netmask: &str) -> String {
        // Simplified network calculation
        if let (Ok(addr), Ok(mask)) = (address.parse::<Ipv4Addr>(), netmask.parse::<Ipv4Addr>()) {
            let addr_u32 = u32::from(addr);
            let mask_u32 = u32::from(mask);
            let network_u32 = addr_u32 & mask_u32;
            let network_addr = Ipv4Addr::from(network_u32);
            network_addr.to_string()
        } else {
            "0.0.0.0".to_string()
        }
    }

    async fn detect_default_ipv4_interface(
        &self,
        interfaces: &HashMap<String, InterfaceFacts>,
    ) -> Option<DefaultInterface> {
        // Try to get default route
        #[cfg(unix)]
        {
            if let Ok(output) = tokio::process::Command::new("ip")
                .args(["route", "show", "default"])
                .output()
                .await
            {
                if output.status.success() {
                    let route_output = String::from_utf8_lossy(&output.stdout);
                    return self.parse_default_route(&route_output, interfaces);
                }
            }

            // Fallback to route command
            if let Ok(output) = tokio::process::Command::new("route")
                .args(["-n"])
                .output()
                .await
            {
                if output.status.success() {
                    let route_output = String::from_utf8_lossy(&output.stdout);
                    return self.parse_route_table(&route_output, interfaces);
                }
            }
        }

        None
    }

    async fn detect_default_ipv6_interface(
        &self,
        _interfaces: &HashMap<String, InterfaceFacts>,
    ) -> Option<DefaultInterface> {
        // IPv6 default route detection would be implemented here
        None
    }

    #[cfg(unix)]
    fn parse_default_route(
        &self,
        route_output: &str,
        interfaces: &HashMap<String, InterfaceFacts>,
    ) -> Option<DefaultInterface> {
        for line in route_output.lines() {
            if line.contains("default") {
                let parts: Vec<&str> = line.split_whitespace().collect();

                // Look for interface name and gateway
                let mut gateway = None;
                let mut interface = None;

                for (i, part) in parts.iter().enumerate() {
                    if *part == "via" && i + 1 < parts.len() {
                        gateway = Some(parts[i + 1].to_string());
                    } else if *part == "dev" && i + 1 < parts.len() {
                        interface = Some(parts[i + 1].to_string());
                    }
                }

                if let (Some(gw), Some(iface)) = (gateway, interface) {
                    if let Some(iface_facts) = interfaces.get(&iface) {
                        if let Some(ipv4) = &iface_facts.ipv4 {
                            return Some(DefaultInterface {
                                interface: iface,
                                address: ipv4.address.clone(),
                                gateway: gw,
                                network: ipv4.network.clone(),
                                netmask: ipv4.netmask.clone(),
                                broadcast: ipv4.broadcast.clone(),
                            });
                        }
                    }
                }
            }
        }

        None
    }

    #[cfg(unix)]
    fn parse_route_table(
        &self,
        route_output: &str,
        interfaces: &HashMap<String, InterfaceFacts>,
    ) -> Option<DefaultInterface> {
        for line in route_output.lines() {
            if line.starts_with("0.0.0.0") || line.contains("0.0.0.0") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    let gateway = parts[1].to_string();
                    let interface = parts[7].to_string();

                    if let Some(iface_facts) = interfaces.get(&interface) {
                        if let Some(ipv4) = &iface_facts.ipv4 {
                            return Some(DefaultInterface {
                                interface,
                                address: ipv4.address.clone(),
                                gateway,
                                network: ipv4.network.clone(),
                                netmask: ipv4.netmask.clone(),
                                broadcast: ipv4.broadcast.clone(),
                            });
                        }
                    }
                }
            }
        }

        None
    }
}
