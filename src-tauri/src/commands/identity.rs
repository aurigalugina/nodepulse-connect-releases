use std::process::Command;

/// Cross-platform machine ID (stable per-install device fingerprint).
/// Returns empty string on failure — never blocks mesh join.
fn read_machine_id() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/machine-id")
            .unwrap_or_default()
            .trim()
            .to_string()
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok();
        if let Some(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                if line.contains("IOPlatformUUID") {
                    // line looks like: "IOPlatformUUID" = "XXXXXXXX-..."
                    if let Some(start) = line.rfind('"') {
                        let rest = &line[..start];
                        if let Some(start2) = rest.rfind('"') {
                            return rest[start2 + 1..].to_string();
                        }
                    }
                }
            }
        }
        String::new()
    }
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output()
            .ok();
        if let Some(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                if line.contains("MachineGuid") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(guid) = parts.last() {
                        return guid.to_string();
                    }
                }
            }
        }
        String::new()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        String::new()
    }
}

/// Primary NIC MAC address (secondary identifier only — not used for auth).
/// Returns empty string on failure.
fn read_primary_mac() -> String {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::net::UdpSocket;
        // Use a UDP connect trick to find the primary outbound interface name,
        // then parse MAC from /sys (Linux) or ifconfig (macOS).
        #[cfg(target_os = "linux")]
        {
            // Read from /sys/class/net — pick first non-loopback UP interface with a MAC.
            if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let iface = name.to_string_lossy();
                    if iface == "lo" {
                        continue;
                    }
                    let flags_path = format!("/sys/class/net/{}/flags", iface);
                    let flags_str = std::fs::read_to_string(&flags_path).unwrap_or_default();
                    let flags = u32::from_str_radix(flags_str.trim().trim_start_matches("0x"), 16).unwrap_or(0);
                    // IFF_UP = 0x1, IFF_LOOPBACK = 0x8
                    if flags & 0x1 == 0 || flags & 0x8 != 0 {
                        continue;
                    }
                    let mac_path = format!("/sys/class/net/{}/address", iface);
                    if let Ok(mac) = std::fs::read_to_string(&mac_path) {
                        let mac = mac.trim().to_string();
                        if mac != "00:00:00:00:00:00" && !mac.is_empty() {
                            return mac;
                        }
                    }
                }
            }
            String::new()
        }
        #[cfg(target_os = "macos")]
        {
            let out = Command::new("ifconfig").output().ok();
            if let Some(o) = out {
                let text = String::from_utf8_lossy(&o.stdout);
                let mut current_up = false;
                let mut current_loopback = false;
                for line in text.lines() {
                    if !line.starts_with('\t') && !line.starts_with(' ') {
                        current_up = line.contains("UP");
                        current_loopback = line.contains("LOOPBACK");
                    }
                    if current_up && !current_loopback {
                        let trimmed = line.trim();
                        if trimmed.starts_with("ether ") {
                            return trimmed[6..].split_whitespace().next().unwrap_or("").to_string();
                        }
                    }
                }
            }
            String::new()
        }
    }
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("getmac")
            .args(["/fo", "csv", "/nh"])
            .output()
            .ok();
        if let Some(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, ',').collect();
                if let Some(mac) = parts.first() {
                    let mac = mac.trim().trim_matches('"');
                    if mac != "N/A" && !mac.is_empty() {
                        return mac.to_string();
                    }
                }
            }
        }
        String::new()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        String::new()
    }
}

#[derive(serde::Serialize)]
pub struct DeviceIdentity {
    pub machine_id: String,
    pub mac_address: String,
}

#[tauri::command]
pub fn get_device_identity() -> DeviceIdentity {
    DeviceIdentity {
        machine_id: read_machine_id(),
        mac_address: read_primary_mac(),
    }
}
