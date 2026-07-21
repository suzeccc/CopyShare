use std::{
    net::{IpAddr, Ipv4Addr, TcpListener},
    process::Command,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    discovery::DISCOVERY_PORT,
    error::{AppError, AppResult},
    mobile::MOBILE_HTTP_PORT,
    models::AppConfig,
    network,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const FIREWALL_RULE_SYNC: &str = "CopyShare LAN Sync";
const FIREWALL_RULE_DISCOVERY: &str = "CopyShare LAN Discovery";
const FIREWALL_RULE_MOBILE: &str = "CopyShare Mobile Session";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnosticReport {
    pub generated_at: DateTime<Utc>,
    pub platform: String,
    pub preferred_local_ip: Option<String>,
    pub local_addresses: Vec<LocalNetworkAddress>,
    pub sync_running: bool,
    pub repair_supported: bool,
    pub checks: Vec<NetworkDiagnosticCheck>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalNetworkAddress {
    pub adapter_name: String,
    pub address: String,
    pub private: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnosticCheck {
    pub id: String,
    pub status: DiagnosticStatus,
    pub title: String,
    pub detail: String,
    pub recommendation: Option<String>,
    pub protocol: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WindowsFirewallSnapshot {
    private_firewall_enabled: bool,
    #[serde(default)]
    active_profiles: Vec<String>,
    #[serde(default)]
    sync_allowed: bool,
    #[serde(default)]
    discovery_allowed: bool,
    #[serde(default)]
    mobile_allowed: bool,
}

pub fn run(
    config: &AppConfig,
    sync_running: bool,
    discovery_running: bool,
    mobile_server_running: bool,
) -> NetworkDiagnosticReport {
    let local_addresses = local_network_addresses();
    let preferred_local_ip =
        network::preferred_local_ip(&config.trusted_devices, config.port).map(|ip| ip.to_string());
    let mut checks = vec![local_address_check(&local_addresses, &preferred_local_ip)];

    checks.push(sync_listener_check(config.port, sync_running));
    checks.push(discovery_listener_check(discovery_running));
    checks.push(mobile_listener_check(mobile_server_running));
    append_firewall_checks(&mut checks, config.port);

    NetworkDiagnosticReport {
        generated_at: Utc::now(),
        platform: std::env::consts::OS.to_string(),
        preferred_local_ip,
        local_addresses,
        sync_running,
        repair_supported: cfg!(target_os = "windows"),
        checks,
    }
}

pub fn repair_windows_firewall(sync_port: u16) -> AppResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = sync_port;
        return Err(AppError::InvalidInput(
            "自动修复防火墙目前仅支持 Windows".to_string(),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let executable = std::env::current_exe()?;
        let executable = escape_powershell_literal(&executable.to_string_lossy());
        let sync_rule_name = firewall_rule_name(FIREWALL_RULE_SYNC, "TCP", sync_port);
        let discovery_rule_name =
            firewall_rule_name(FIREWALL_RULE_DISCOVERY, "UDP", DISCOVERY_PORT);
        let mobile_rule_name = firewall_rule_name(FIREWALL_RULE_MOBILE, "TCP", MOBILE_HTTP_PORT);
        let inner_script = format!(
            r#"$ErrorActionPreference = 'Stop'
$exe = '{executable}'
$rules = @(
  @{{ Name = '{sync_rule_name}'; Protocol = 'TCP'; Port = {sync_port} }},
  @{{ Name = '{discovery_rule_name}'; Protocol = 'UDP'; Port = {DISCOVERY_PORT} }},
  @{{ Name = '{mobile_rule_name}'; Protocol = 'TCP'; Port = {MOBILE_HTTP_PORT} }}
)
Get-NetFirewallRule -Group 'CopyShare' -ErrorAction SilentlyContinue | Remove-NetFirewallRule -ErrorAction SilentlyContinue
foreach ($item in $rules) {{
  New-NetFirewallRule -DisplayName $item.Name -Group 'CopyShare' -Direction Inbound -Action Allow -Enabled True -Profile Private -Program $exe -Protocol $item.Protocol -LocalPort $item.Port | Out-Null
}}
"#,
        );
        let inner_encoded = encode_powershell(&inner_script);
        let outer_script = format!(
            "$process = Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand','{inner_encoded}') -Wait -PassThru; exit $process.ExitCode"
        );
        let output = run_powershell(&outer_script)?;
        if output.status.success() {
            return Ok(());
        }

        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(AppError::Tauri(if detail.is_empty() {
            "防火墙修复未完成，可能已取消管理员授权".to_string()
        } else {
            format!("防火墙修复失败：{detail}")
        }))
    }
}

fn local_network_addresses() -> Vec<LocalNetworkAddress> {
    let mut addresses = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|adapter| {
            let IpAddr::V4(address) = adapter.ip() else {
                return None;
            };
            is_usable_ipv4(address).then(|| LocalNetworkAddress {
                adapter_name: adapter.name,
                address: address.to_string(),
                private: address.is_private(),
            })
        })
        .collect::<Vec<_>>();
    addresses.sort_by(|left, right| {
        right
            .private
            .cmp(&left.private)
            .then_with(|| left.adapter_name.cmp(&right.adapter_name))
            .then_with(|| left.address.cmp(&right.address))
    });
    addresses.dedup_by(|left, right| {
        left.adapter_name == right.adapter_name && left.address == right.address
    });
    addresses
}

fn is_usable_ipv4(ip: Ipv4Addr) -> bool {
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_link_local())
}

fn local_address_check(
    addresses: &[LocalNetworkAddress],
    preferred_local_ip: &Option<String>,
) -> NetworkDiagnosticCheck {
    if addresses.is_empty() {
        return check(
            "local-address",
            DiagnosticStatus::Error,
            "局域网地址",
            "没有检测到可用的 IPv4 网络地址。",
            Some("请确认网卡已连接，并暂时停用冲突的 VPN 或虚拟网卡后重试。"),
            None,
            None,
        );
    }

    let private_count = addresses.iter().filter(|address| address.private).count();
    let displayed = addresses
        .iter()
        .take(4)
        .map(|address| format!("{} ({})", address.address, address.adapter_name))
        .collect::<Vec<_>>()
        .join("、");
    let preferred = preferred_local_ip
        .as_deref()
        .map(|ip| format!("首选地址 {ip}；"))
        .unwrap_or_default();

    if private_count == 0 {
        return check(
            "local-address",
            DiagnosticStatus::Warning,
            "局域网地址",
            format!("{preferred}检测到 {displayed}，但没有私有 IPv4 地址。"),
            Some("请确认当前设备连接到家庭或办公局域网。"),
            None,
            None,
        );
    }

    check(
        "local-address",
        DiagnosticStatus::Pass,
        "局域网地址",
        format!("{preferred}检测到 {private_count} 个私有 IPv4 地址：{displayed}"),
        None,
        None,
        None,
    )
}

fn sync_listener_check(port: u16, running: bool) -> NetworkDiagnosticCheck {
    if running {
        return check(
            "sync-listener",
            DiagnosticStatus::Pass,
            "同步与文件传输监听",
            format!("正在监听所有网卡的 TCP {port} 端口。"),
            None,
            Some("TCP"),
            Some(port),
        );
    }

    match TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)) {
        Ok(listener) => {
            drop(listener);
            check(
                "sync-listener",
                DiagnosticStatus::Warning,
                "同步与文件传输监听",
                format!("同步尚未启动；TCP {port} 端口当前可用。"),
                Some("启动同步后，其他设备才能连接并传输内容。"),
                Some("TCP"),
                Some(port),
            )
        }
        Err(error) => check(
            "sync-listener",
            DiagnosticStatus::Error,
            "同步与文件传输监听",
            format!("同步尚未启动，且 TCP {port} 端口不可用：{error}"),
            Some("关闭占用该端口的程序，或在基础设置中更换监听端口。"),
            Some("TCP"),
            Some(port),
        ),
    }
}

fn discovery_listener_check(running: bool) -> NetworkDiagnosticCheck {
    if running {
        check(
            "discovery-listener",
            DiagnosticStatus::Pass,
            "局域网自动发现",
            format!("自动发现服务正在使用 UDP {DISCOVERY_PORT}。"),
            None,
            Some("UDP"),
            Some(DISCOVERY_PORT),
        )
    } else {
        check(
            "discovery-listener",
            DiagnosticStatus::Error,
            "局域网自动发现",
            format!("自动发现服务未能监听 UDP {DISCOVERY_PORT}。"),
            Some("检查端口占用和防火墙；仍可在设备页手动输入对方 IP。"),
            Some("UDP"),
            Some(DISCOVERY_PORT),
        )
    }
}

fn mobile_listener_check(running: bool) -> NetworkDiagnosticCheck {
    if running {
        return check(
            "mobile-listener",
            DiagnosticStatus::Pass,
            "手机临时连接",
            format!("手机连接服务正在使用 TCP {MOBILE_HTTP_PORT}。"),
            None,
            Some("TCP"),
            Some(MOBILE_HTTP_PORT),
        );
    }

    match TcpListener::bind((Ipv4Addr::UNSPECIFIED, MOBILE_HTTP_PORT)) {
        Ok(listener) => {
            drop(listener);
            check(
                "mobile-listener",
                DiagnosticStatus::Pass,
                "手机临时连接",
                format!("TCP {MOBILE_HTTP_PORT} 端口可用，将在创建二维码会话时启用。"),
                None,
                Some("TCP"),
                Some(MOBILE_HTTP_PORT),
            )
        }
        Err(error) => check(
            "mobile-listener",
            DiagnosticStatus::Error,
            "手机临时连接",
            format!("TCP {MOBILE_HTTP_PORT} 端口不可用：{error}"),
            Some("关闭占用该端口的程序后重新检测。"),
            Some("TCP"),
            Some(MOBILE_HTTP_PORT),
        ),
    }
}

fn append_firewall_checks(checks: &mut Vec<NetworkDiagnosticCheck>, sync_port: u16) {
    #[cfg(not(target_os = "windows"))]
    {
        checks.push(check(
            "firewall",
            DiagnosticStatus::Unknown,
            "系统防火墙",
            "当前平台暂不支持自动读取防火墙规则。",
            Some(&format!(
                "请手动允许 TCP {sync_port}、UDP {DISCOVERY_PORT} 和 TCP {MOBILE_HTTP_PORT} 的入站访问。"
            )),
            None,
            None,
        ));
    }

    #[cfg(target_os = "windows")]
    match windows_firewall_snapshot(sync_port) {
        Ok(snapshot) => append_windows_firewall_checks(checks, sync_port, snapshot),
        Err(error) => {
            checks.push(check(
                "windows-network-profile",
                DiagnosticStatus::Unknown,
                "Windows 网络类型",
                format!("无法读取活动网络类型：{error}"),
                Some("可以重新检测，或在 Windows 设置中确认当前网络为“专用网络”。"),
                None,
                None,
            ));
            for (id, title, protocol, port) in firewall_targets(sync_port) {
                checks.push(check(
                    id,
                    DiagnosticStatus::Unknown,
                    title,
                    format!("无法确认 {protocol} {port} 的 Windows 防火墙状态。"),
                    Some("点击“修复防火墙”可重新创建 CopyShare 专用网络入站规则。"),
                    Some(protocol),
                    Some(port),
                ));
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn append_windows_firewall_checks(
    checks: &mut Vec<NetworkDiagnosticCheck>,
    sync_port: u16,
    snapshot: WindowsFirewallSnapshot,
) {
    let public_active = snapshot
        .active_profiles
        .iter()
        .any(|profile| profile.eq_ignore_ascii_case("public"));
    let profiles = if snapshot.active_profiles.is_empty() {
        "未识别".to_string()
    } else {
        snapshot.active_profiles.join("、")
    };
    checks.push(if public_active {
        check(
            "windows-network-profile",
            DiagnosticStatus::Warning,
            "Windows 网络类型",
            format!("活动网络类型为 {profiles}；公共网络会限制局域网入站连接。"),
            Some("确认当前网络可信后，在 Windows 网络设置中将它改为“专用网络”。"),
            None,
            None,
        )
    } else {
        check(
            "windows-network-profile",
            DiagnosticStatus::Pass,
            "Windows 网络类型",
            format!("活动网络类型：{profiles}。"),
            None,
            None,
            None,
        )
    });

    checks.push(if snapshot.private_firewall_enabled {
        check(
            "windows-firewall-profile",
            DiagnosticStatus::Pass,
            "Windows 专用网络防火墙",
            "专用网络防火墙已启用，将检查 CopyShare 入站规则。",
            None,
            None,
            None,
        )
    } else {
        check(
            "windows-firewall-profile",
            DiagnosticStatus::Warning,
            "Windows 专用网络防火墙",
            "专用网络防火墙当前未启用，端口不会被它阻止。",
            Some("建议启用 Windows 防火墙，并使用下方修复按钮创建最小范围规则。"),
            None,
            None,
        )
    });

    let allowed = [
        snapshot.sync_allowed,
        snapshot.discovery_allowed,
        snapshot.mobile_allowed,
    ];
    for ((id, title, protocol, port), allowed) in
        firewall_targets(sync_port).into_iter().zip(allowed)
    {
        checks.push(if allowed || !snapshot.private_firewall_enabled {
            check(
                id,
                DiagnosticStatus::Pass,
                title,
                if snapshot.private_firewall_enabled {
                    format!("专用网络已允许 {protocol} {port} 入站访问。")
                } else {
                    format!("防火墙未启用，{protocol} {port} 当前不会被阻止。")
                },
                None,
                Some(protocol),
                Some(port),
            )
        } else {
            check(
                id,
                DiagnosticStatus::Error,
                title,
                format!("没有找到覆盖 {protocol} {port} 的专用网络入站允许规则。"),
                Some("点击“修复防火墙”，通过管理员授权创建 CopyShare 规则。"),
                Some(protocol),
                Some(port),
            )
        });
    }
}

fn firewall_targets(sync_port: u16) -> [(&'static str, &'static str, &'static str, u16); 3] {
    [
        ("firewall-sync", "同步端口防火墙", "TCP", sync_port),
        (
            "firewall-discovery",
            "自动发现防火墙",
            "UDP",
            DISCOVERY_PORT,
        ),
        ("firewall-mobile", "手机连接防火墙", "TCP", MOBILE_HTTP_PORT),
    ]
}

fn firewall_rule_name(base: &str, protocol: &str, port: u16) -> String {
    format!("{base} ({protocol} {port})")
}

#[cfg(target_os = "windows")]
fn windows_firewall_snapshot(sync_port: u16) -> AppResult<WindowsFirewallSnapshot> {
    let script = r#"$ErrorActionPreference = 'Stop'
$privateProfile = Get-NetFirewallProfile -Name Private
$connections = @(Get-NetConnectionProfile -ErrorAction SilentlyContinue | Where-Object { $_.IPv4Connectivity.ToString() -ne 'Disconnected' })
$physicalConnections = @($connections | Where-Object { $_.InterfaceAlias -notmatch '(?i)(tun|tap|vpn|tailscale|zerotier|vethernet|hyper-v|vmware|virtualbox|docker|wsl|loopback)' })
if ($physicalConnections.Count -gt 0) { $connections = $physicalConnections }
$activeProfiles = @($connections | ForEach-Object { $_.NetworkCategory.ToString() } | Select-Object -Unique)
[pscustomobject]@{
  privateFirewallEnabled = [bool]$privateProfile.Enabled
  activeProfiles = @($activeProfiles)
} | ConvertTo-Json -Depth 4 -Compress
"#;
    let output = run_powershell(&script)?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Tauri(if detail.is_empty() {
            "Windows 防火墙查询失败".to_string()
        } else {
            format!("Windows 防火墙查询失败：{detail}")
        }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut snapshot: WindowsFirewallSnapshot = serde_json::from_str(stdout.trim())?;
    if snapshot.private_firewall_enabled {
        snapshot.sync_allowed = windows_firewall_rule_exists(&firewall_rule_name(
            FIREWALL_RULE_SYNC,
            "TCP",
            sync_port,
        ))?;
        snapshot.discovery_allowed = windows_firewall_rule_exists(&firewall_rule_name(
            FIREWALL_RULE_DISCOVERY,
            "UDP",
            DISCOVERY_PORT,
        ))?;
        snapshot.mobile_allowed = windows_firewall_rule_exists(&firewall_rule_name(
            FIREWALL_RULE_MOBILE,
            "TCP",
            MOBILE_HTTP_PORT,
        ))?;
    } else {
        snapshot.sync_allowed = true;
        snapshot.discovery_allowed = true;
        snapshot.mobile_allowed = true;
    }
    Ok(snapshot)
}

#[cfg(target_os = "windows")]
fn windows_firewall_rule_exists(display_name: &str) -> AppResult<bool> {
    let mut command = Command::new("netsh");
    let output = command
        .args([
            "advfirewall",
            "firewall",
            "show",
            "rule",
            &format!("name={display_name}"),
            "verbose",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(AppError::Tauri(format!(
            "Windows 防火墙规则查询失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))),
    }
}

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> AppResult<std::process::Output> {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encode_powershell(script),
        ])
        .creation_flags(CREATE_NO_WINDOW);
    Ok(command.output()?)
}

#[cfg(target_os = "windows")]
fn encode_powershell(script: &str) -> String {
    let bytes = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    BASE64.encode(bytes)
}

#[cfg(target_os = "windows")]
fn escape_powershell_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn check(
    id: impl Into<String>,
    status: DiagnosticStatus,
    title: impl Into<String>,
    detail: impl Into<String>,
    recommendation: Option<&str>,
    protocol: Option<&str>,
    port: Option<u16>,
) -> NetworkDiagnosticCheck {
    NetworkDiagnosticCheck {
        id: id.into(),
        status,
        title: title.into(),
        detail: detail.into(),
        recommendation: recommendation.map(ToOwned::to_owned),
        protocol: protocol.map(ToOwned::to_owned),
        port,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_sync_reports_the_configured_listener() {
        let result = sync_listener_check(9876, true);

        assert_eq!(result.status, DiagnosticStatus::Pass);
        assert_eq!(result.protocol.as_deref(), Some("TCP"));
        assert_eq!(result.port, Some(9876));
        assert!(result.detail.contains("9876"));
    }

    #[test]
    fn firewall_targets_cover_all_lan_entry_points() {
        assert_eq!(
            firewall_targets(9123),
            [
                ("firewall-sync", "同步端口防火墙", "TCP", 9123),
                ("firewall-discovery", "自动发现防火墙", "UDP", 8764),
                ("firewall-mobile", "手机连接防火墙", "TCP", 8766),
            ]
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn powershell_encoding_uses_utf16_little_endian() {
        let encoded = encode_powershell("Write-Output '测试'");
        let bytes = BASE64.decode(encoded).unwrap();
        let units = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();

        assert_eq!(String::from_utf16(&units).unwrap(), "Write-Output '测试'");
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires the local Windows NetSecurity service"]
    fn windows_firewall_snapshot_is_valid_json() {
        let snapshot = windows_firewall_snapshot(8765).unwrap();

        assert!(snapshot.active_profiles.iter().all(|profile| [
            "Public",
            "Private",
            "DomainAuthenticated"
        ]
        .contains(&profile.as_str())));
    }
}
