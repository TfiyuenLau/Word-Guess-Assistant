use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub const DISCOVERY_PORT: u16 = 38888;
pub const DISCOVERY_MULTICAST_IPV4: Ipv4Addr = Ipv4Addr::new(239, 255, 42, 99);
pub const PROTOCOL_MAGIC: &str = "WGA-SSDP";

/// 探针与广播报文载荷定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPacket {
    pub protocol: String,
    pub action: String, // "DISCOVER" | "ANNOUNCE" | "BYE"
    #[serde(default)]
    pub room_id: String,
    #[serde(default)]
    pub host_name: String,
    #[serde(default)]
    pub host_avatar: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub player_count: usize,
    #[serde(default)]
    pub max_players: usize,
    #[serde(default)]
    pub phase: i32,
    #[serde(default)]
    pub phase_title: String,
}

/// 供前端展示与一键直连的局域网房间结构体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredRoom {
    pub room_id: String,
    pub host_name: String,
    pub host_avatar: String,
    pub ip: String,
    pub port: u16,
    pub player_count: usize,
    pub max_players: usize,
    pub phase: i32,
    pub phase_title: String,
}

/// 创建具备端口复用与组播/广播权限的高并发 UDP Socket
pub fn create_discovery_socket(port: u16) -> std::io::Result<tokio::net::UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    let _ = socket.set_reuse_port(true);
    let _ = socket.set_broadcast(true);
    socket.set_nonblocking(true)?;

    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    socket.bind(&bind_addr.into())?;

    let std_socket: std::net::UdpSocket = socket.into();
    let tokio_socket = tokio::net::UdpSocket::from_std(std_socket)?;
    let _ = tokio_socket.join_multicast_v4(DISCOVERY_MULTICAST_IPV4, Ipv4Addr::UNSPECIFIED);

    Ok(tokio_socket)
}

/// 房主端：启动后台房间广播与探针响应服务（Advertiser）
pub fn start_room_advertiser(
    tcp_port: u16,
    host_name: String,
    host_avatar: String,
    network_state: crate::network::NetworkState,
    game_state: Option<crate::game_engine::GameEngineState>,
    cancellation_token: CancellationToken,
) {
    tokio::spawn(async move {
        // 尝试绑定 DISCOVERY_PORT；若端口冲突则降级为 0（仍支持广播与组播发送）
        let socket = match create_discovery_socket(DISCOVERY_PORT) {
            Ok(s) => s,
            Err(e) => {
                warn!("绑定固定探针端口 {} 失败 ({})，降级为随机端口广播", DISCOVERY_PORT, e);
                match create_discovery_socket(0) {
                    Ok(s) => s,
                    Err(e2) => {
                        error!("创建房间广播服务 Socket 失败: {}", e2);
                        return;
                    }
                }
            }
        };

        info!("房主在线探针广播服务已启动，通告 TCP 游戏端口: {}", tcp_port);

        let broadcast_target = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, DISCOVERY_PORT));
        let multicast_target = SocketAddr::V4(SocketAddrV4::new(DISCOVERY_MULTICAST_IPV4, DISCOVERY_PORT));

        let mut ticker = tokio::time::interval(Duration::from_millis(2000));
        let mut buf = vec![0u8; 2048];

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("房主探针服务收到销毁指令，发送离开公告并退出");
                    let bye_packet = DiscoveryPacket {
                        protocol: PROTOCOL_MAGIC.to_string(),
                        action: "BYE".to_string(),
                        room_id: format!("{}:{}", host_name, tcp_port),
                        host_name: host_name.clone(),
                        host_avatar: host_avatar.clone(),
                        port: tcp_port,
                        player_count: 0,
                        max_players: 12,
                        phase: 0,
                        phase_title: "房间已解散".to_string(),
                    };
                    if let Ok(bytes) = serde_json::to_vec(&bye_packet) {
                        let _ = socket.send_to(&bytes, broadcast_target).await;
                        let _ = socket.send_to(&bytes, multicast_target).await;
                    }
                    break;
                }

                // 1. 定期主动广播/组播心跳包
                _ = ticker.tick() => {
                    let player_count = network_state.player_profiles.lock().await.len();
                    let (phase, phase_title) = if let Some(ref g_state) = game_state {
                        let g = g_state.inner.lock().await;
                        let title = match g.phase {
                            0 => "准备中",
                            1 => "描述阶段",
                            2 => "讨论与投票",
                            3 => "黑夜暗杀",
                            4 => "终局结算",
                            5 => "白板猜词",
                            _ => "对局中",
                        }.to_string();
                        (g.phase, title)
                    } else {
                        (0, "准备中".to_string())
                    };

                    let packet = DiscoveryPacket {
                        protocol: PROTOCOL_MAGIC.to_string(),
                        action: "ANNOUNCE".to_string(),
                        room_id: format!("{}:{}", host_name, tcp_port),
                        host_name: host_name.clone(),
                        host_avatar: host_avatar.clone(),
                        port: tcp_port,
                        player_count,
                        max_players: 12,
                        phase,
                        phase_title,
                    };

                    if let Ok(bytes) = serde_json::to_vec(&packet) {
                        let _ = socket.send_to(&bytes, broadcast_target).await;
                        let _ = socket.send_to(&bytes, multicast_target).await;
                    }
                }

                // 2. 接收来自客户端的 DISCOVER 探针并即刻单播应答
                recv_res = socket.recv_from(&mut buf) => {
                    if let Ok((len, src_addr)) = recv_res {
                        if let Ok(req) = serde_json::from_slice::<DiscoveryPacket>(&buf[..len]) {
                            if req.protocol == PROTOCOL_MAGIC && req.action == "DISCOVER" {
                                debug!("收到来自 {} 的房间探针请求，即刻回传房间状态", src_addr);
                                let player_count = network_state.player_profiles.lock().await.len();
                                let (phase, phase_title) = if let Some(ref g_state) = game_state {
                                    let g = g_state.inner.lock().await;
                                    let title = match g.phase {
                                        0 => "准备中",
                                        1 => "描述阶段",
                                        2 => "讨论与投票",
                                        3 => "黑夜暗杀",
                                        4 => "终局结算",
                                        5 => "白板猜词",
                                        _ => "对局中",
                                    }.to_string();
                                    (g.phase, title)
                                } else {
                                    (0, "准备中".to_string())
                                };

                                let reply = DiscoveryPacket {
                                    protocol: PROTOCOL_MAGIC.to_string(),
                                    action: "ANNOUNCE".to_string(),
                                    room_id: format!("{}:{}", host_name, tcp_port),
                                    host_name: host_name.clone(),
                                    host_avatar: host_avatar.clone(),
                                    port: tcp_port,
                                    player_count,
                                    max_players: 12,
                                    phase,
                                    phase_title,
                                };

                                if let Ok(bytes) = serde_json::to_vec(&reply) {
                                    let _ = socket.send_to(&bytes, src_addr).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

/// 客户端端：扫描发现局域网房间 Command
#[tauri::command]
pub async fn discover_rooms_command(
    app_handle: AppHandle,
    timeout_ms: Option<u64>,
) -> Result<Vec<DiscoveredRoom>, String> {
    let scan_timeout = Duration::from_millis(timeout_ms.unwrap_or(1200));

    // 创建客户端专用的探测 Socket（绑定 0 获取空闲端口）
    let socket = create_discovery_socket(0)
        .map_err(|e| format!("创建探针 Socket 失败: {}", e))?;

    let broadcast_target = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, DISCOVERY_PORT));
    let multicast_target = SocketAddr::V4(SocketAddrV4::new(DISCOVERY_MULTICAST_IPV4, DISCOVERY_PORT));

    // 发送主动探测广播包
    let probe_packet = DiscoveryPacket {
        protocol: PROTOCOL_MAGIC.to_string(),
        action: "DISCOVER".to_string(),
        room_id: String::new(),
        host_name: String::new(),
        host_avatar: String::new(),
        port: 0,
        player_count: 0,
        max_players: 0,
        phase: 0,
        phase_title: String::new(),
    };

    let probe_bytes = serde_json::to_vec(&probe_packet)
        .map_err(|e| format!("序列化探测包失败: {}", e))?;

    let _ = socket.send_to(&probe_bytes, broadcast_target).await;
    let _ = socket.send_to(&probe_bytes, multicast_target).await;

    let mut discovered: HashMap<String, DiscoveredRoom> = HashMap::new();
    let mut buf = vec![0u8; 2048];
    let deadline = tokio::time::Instant::now() + scan_timeout;

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, src_addr))) => {
                if let Ok(packet) = serde_json::from_slice::<DiscoveryPacket>(&buf[..len]) {
                    if packet.protocol == PROTOCOL_MAGIC && packet.action == "ANNOUNCE" {
                        let sender_ip = match src_addr.ip() {
                            std::net::IpAddr::V4(v4) => {
                                if v4.is_loopback() || v4.is_unspecified() {
                                    "127.0.0.1".to_string()
                                } else {
                                    v4.to_string()
                                }
                            }
                            std::net::IpAddr::V6(v6) => {
                                if v6.is_loopback() {
                                    "127.0.0.1".to_string()
                                } else {
                                    v6.to_string()
                                }
                            }
                        };

                        let room_key = format!("{}:{}", sender_ip, packet.port);
                        let room = DiscoveredRoom {
                            room_id: packet.room_id,
                            host_name: packet.host_name,
                            host_avatar: packet.host_avatar,
                            ip: sender_ip,
                            port: packet.port,
                            player_count: packet.player_count,
                            max_players: packet.max_players,
                            phase: packet.phase,
                            phase_title: packet.phase_title,
                        };

                        let _ = app_handle.emit("lan_room_discovered", &room);
                        discovered.insert(room_key, room);
                    }
                }
            }
            _ => break,
        }
    }

    Ok(discovered.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_discovery_socket() {
        let sock = create_discovery_socket(0);
        assert!(sock.is_ok());
    }

    #[tokio::test]
    async fn test_discovery_packet_serialization() {
        let p = DiscoveryPacket {
            protocol: PROTOCOL_MAGIC.to_string(),
            action: "ANNOUNCE".to_string(),
            room_id: "test_room".to_string(),
            host_name: "房主".to_string(),
            host_avatar: "avatar.svg".to_string(),
            port: 8888,
            player_count: 3,
            max_players: 12,
            phase: 0,
            phase_title: "准备中".to_string(),
        };

        let json = serde_json::to_string(&p).unwrap();
        let decoded: DiscoveryPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.protocol, PROTOCOL_MAGIC);
        assert_eq!(decoded.port, 8888);
        assert_eq!(decoded.player_count, 3);
    }

    #[tokio::test]
    async fn test_advertiser_and_discovery_interaction() {
        let cancel_token = CancellationToken::new();
        let net_state = crate::network::NetworkState::new();

        // 模拟房主启动探针广播
        start_room_advertiser(
            9999,
            "测试房主".to_string(),
            "avatar.svg".to_string(),
            net_state,
            None,
            cancel_token.clone(),
        );

        // 稍等 100ms 待 Socket 启动完成
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 客户端创建探针接收并发送 DISCOVER
        let client_sock = create_discovery_socket(0).unwrap();
        let probe = DiscoveryPacket {
            protocol: PROTOCOL_MAGIC.to_string(),
            action: "DISCOVER".to_string(),
            room_id: String::new(),
            host_name: String::new(),
            host_avatar: String::new(),
            port: 0,
            player_count: 0,
            max_players: 0,
            phase: 0,
            phase_title: String::new(),
        };
        let probe_bytes = serde_json::to_vec(&probe).unwrap();
        let target: SocketAddr = format!("127.0.0.1:{}", DISCOVERY_PORT).parse().unwrap();
        let _ = client_sock.send_to(&probe_bytes, target).await;

        let mut buf = vec![0u8; 1024];
        let recv_res = tokio::time::timeout(Duration::from_millis(2500), client_sock.recv_from(&mut buf)).await;

        cancel_token.cancel();

        assert!(recv_res.is_ok());
        if let Ok(Ok((len, _))) = recv_res {
            let res: DiscoveryPacket = serde_json::from_slice(&buf[..len]).unwrap();
            assert_eq!(res.protocol, PROTOCOL_MAGIC);
            assert_eq!(res.action, "ANNOUNCE");
            assert_eq!(res.port, 9999);
            assert_eq!(res.host_name, "测试房主");
        }
    }
}
