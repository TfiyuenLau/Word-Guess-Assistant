use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter, State};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::codec::{
    game_message::Data, ChatMessage, GameMessage, JoinRequest, LeaveRequest, ProtobufCodec,
};

/// 全局网络与房间状态管理
#[derive(Clone)]
pub struct NetworkState {
    /// 房主维护的所有玩家发送信箱：player_id -> Sender<GameMessage>
    pub peers: Arc<Mutex<HashMap<String, mpsc::Sender<GameMessage>>>>,
    /// 客户端（及房主自身）发往房主逻辑中心的信箱
    pub host_sender: Arc<Mutex<Option<mpsc::Sender<GameMessage>>>>,
    /// 用于优雅销毁房间监听循环及所有连接子任务的令牌
    pub cancellation_token: Arc<Mutex<Option<CancellationToken>>>,
    /// 房间内所有玩家的基础信息缓存（用于新玩家加入时全量同步大厅列表）
    pub player_profiles: Arc<Mutex<HashMap<String, JoinRequest>>>,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkState {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            host_sender: Arc::new(Mutex::new(None)),
            cancellation_token: Arc::new(Mutex::new(None)),
            player_profiles: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// 统一派发游戏消息至前端 Vue 监听器
pub fn dispatch_message_to_frontend(app_handle: &AppHandle, msg: &GameMessage) {
    if let Err(e) = app_handle.emit("game_message", msg) {
        warn!("发送全局 game_message 事件失败: {}", e);
    }

    if let Some(ref data) = msg.data {
        match data {
            Data::JoinReq(req) => {
                let _ = app_handle.emit("join_req", req);
            }
            Data::LeaveReq(req) => {
                let _ = app_handle.emit("leave_req", req);
            }
            Data::Identity(identity) => {
                let _ = app_handle.emit("identity", identity);
            }
            Data::Chat(chat) => {
                let _ = app_handle.emit("chat", chat);
            }
            Data::Vote(vote) => {
                let _ = app_handle.emit("vote", vote);
            }
            Data::Night(night) => {
                let _ = app_handle.emit("night", night);
            }
            Data::SyncPhase(phase) => {
                let _ = app_handle.emit("sync_phase", phase);
            }
            Data::Heartbeat(hb) => {
                let _ = app_handle.emit("heartbeat", hb);
            }
            Data::BlankGuess(guess) => {
                let _ = app_handle.emit("blank_guess", guess);
            }
        }
    }
}

/// 广播消息给所有连接中的玩家（包括房主自己的虚拟信箱）
pub async fn broadcast_message(
    peers: &Arc<Mutex<HashMap<String, mpsc::Sender<GameMessage>>>>,
    msg: &GameMessage,
) {
    let peers_guard = peers.lock().await;
    for (player_id, tx) in peers_guard.iter() {
        if let Err(e) = tx.send(msg.clone()).await {
            warn!("向玩家 {} 广播消息失败: {}", player_id, e);
        }
    }
}

/// 向指定玩家单播消息（用于下发天使/卧底/好人/白板个性化身份词语等）
pub async fn send_to_player(
    peers: &Arc<Mutex<HashMap<String, mpsc::Sender<GameMessage>>>>,
    player_id: &str,
    msg: GameMessage,
) -> Result<(), String> {
    let peers_guard = peers.lock().await;
    if let Some(tx) = peers_guard.get(player_id) {
        tx.send(msg).await.map_err(|e| format!("发送给玩家 {} 失败: {}", player_id, e))
    } else {
        Err(format!("玩家 {} 未连接", player_id))
    }
}

/// 优雅清理网络状态及后台所有运行任务
pub async fn close_internal(state: &NetworkState) {
    if let Some(token) = state.cancellation_token.lock().await.take() {
        token.cancel();
    }
    state.peers.lock().await.clear();
    *state.host_sender.lock().await = None;
    state.player_profiles.lock().await.clear();
}

/// 处理房主中央逻辑中心收到的消息（路由与状态同步）
async fn handle_host_logic_message(state: &NetworkState, app_handle: &AppHandle, msg: GameMessage) {
    if let Some(ref data) = msg.data {
        match data {
            Data::JoinReq(req) => {
                info!("中央逻辑接收到玩家加入: {} ({})", req.player_name, req.player_id);
                // 1. 缓存新玩家信息
                state.player_profiles.lock().await.insert(req.player_id.clone(), req.clone());

                // 2. 先给新加入的玩家单向同步当前房间内已有所有玩家的信息（全量同步）
                {
                    let profiles = state.player_profiles.lock().await;
                    let peers_guard = state.peers.lock().await;
                    if let Some(new_peer_tx) = peers_guard.get(&req.player_id) {
                        for (pid, existing_req) in profiles.iter() {
                            if pid != &req.player_id {
                                let sync_msg = GameMessage {
                                    data: Some(Data::JoinReq(existing_req.clone())),
                                };
                                let _ = new_peer_tx.send(sync_msg).await;
                            }
                        }
                    }
                }

                // 3. 将新玩家的加入广播给房间所有人（包括房主和其他玩家）
                broadcast_message(&state.peers, &msg).await;

                // 4. 若对局已在进行中（非 PREPARING 准备阶段），执行【断线重连】或【观战模式】状态同步
                {
                    use tauri::Manager;
                    if let Some(engine_state) = app_handle.try_state::<crate::game_engine::GameEngineState>() {
                        let game = engine_state.inner.lock().await;
                        if game.phase != crate::codec::GamePhase::Preparing as i32 {
                            let peers_guard = state.peers.lock().await;
                            if let Some(new_peer_tx) = peers_guard.get(&req.player_id) {
                                if let Some(existing_p) = game.players.get(&req.player_id) {
                                    // A. 既有玩家断线重连：恢复其身份包与当前对局阶段
                                    let id_msg = GameMessage {
                                        data: Some(Data::Identity(crate::codec::IdentityAssignment {
                                            role: existing_p.role.clone(),
                                            word: existing_p.word.clone(),
                                            hint: existing_p.hint.clone(),
                                        })),
                                    };
                                    let _ = new_peer_tx.send(id_msg).await;

                                    let phase_msg = GameMessage {
                                        data: Some(Data::SyncPhase(crate::codec::SyncPhase {
                                            new_phase: game.phase,
                                            extra_info: format!("重连成功：当前为第 {} 轮对局进行中。", game.round),
                                        })),
                                    };
                                    let _ = new_peer_tx.send(phase_msg).await;

                                    let reconnect_chat = GameMessage {
                                        data: Some(Data::Chat(crate::codec::ChatMessage {
                                            sender_id: "SYSTEM".to_string(),
                                            text: format!("【重连通知】特工【{}】已重连回到对局。", req.player_name),
                                            is_system: true,
                                        })),
                                    };
                                    broadcast_message(&state.peers, &reconnect_chat).await;
                                } else {
                                    // B. 陌生玩家中途加入观战：分发观战者身份与当前阶段
                                    let spectator_id_msg = GameMessage {
                                        data: Some(Data::Identity(crate::codec::IdentityAssignment {
                                            role: "SPECTATOR".to_string(),
                                            word: String::new(),
                                            hint: "您已中途进入房间，当前处于观战模式。".to_string(),
                                        })),
                                    };
                                    let _ = new_peer_tx.send(spectator_id_msg).await;

                                    let phase_msg = GameMessage {
                                        data: Some(Data::SyncPhase(crate::codec::SyncPhase {
                                            new_phase: game.phase,
                                            extra_info: format!("观战就绪：当前为第 {} 轮对局进行中。", game.round),
                                        })),
                                    };
                                    let _ = new_peer_tx.send(phase_msg).await;

                                    let spectator_chat = GameMessage {
                                        data: Some(Data::Chat(crate::codec::ChatMessage {
                                            sender_id: "SYSTEM".to_string(),
                                            text: format!("【观战通知】特工【{}】中途加入房间观战。", req.player_name),
                                            is_system: true,
                                        })),
                                    };
                                    broadcast_message(&state.peers, &spectator_chat).await;
                                }
                            }
                        }
                    }
                }
            }
            Data::LeaveReq(req) => {
                info!("中央逻辑接收到玩家离开: {}", req.player_id);
                state.player_profiles.lock().await.remove(&req.player_id);
                state.peers.lock().await.remove(&req.player_id);
                broadcast_message(&state.peers, &msg).await;
            }
            Data::Vote(vote) => {
                // 收到玩家公投票选：调用游戏引擎秘密登记并判定是否全员投票完毕自动流转，绝不广播谁投了谁！
                use tauri::Manager;
                let game_state = app_handle.state::<crate::game_engine::GameEngineState>();
                let mut game = game_state.inner.lock().await;
                if let Err(e) = crate::game_engine::record_vote_and_maybe_advance(
                    &mut game,
                    state,
                    app_handle,
                    &vote.voter_id,
                    &vote.target_id,
                ).await {
                    warn!("处理玩家 {} 投票失败: {}", vote.voter_id, e);
                }
            }
            Data::Night(night) => {
                // 夜间行动裁决：静默登记伤亡，待天亮统一客观播报
                use tauri::Manager;
                let game_state = app_handle.state::<crate::game_engine::GameEngineState>();
                let mut game = game_state.inner.lock().await;
                if game.phase == crate::codec::GamePhase::NightAction as i32 {
                    let actor_alive_role = game.players.get(&night.actor_id).map(|p| (p.is_alive, p.role.clone()));
                    if let Some((true, role)) = actor_alive_role {
                        if role == crate::game_engine::ROLE_SPY {
                            if let Some(target) = game.players.get_mut(&night.target_id) {
                                if target.is_alive {
                                    target.is_alive = false;
                                    if !game.night_casualties.contains(&night.target_id) {
                                        game.night_casualties.push(night.target_id.clone());
                                    }
                                }
                            }
                        } else {
                            if let Some(actor) = game.players.get_mut(&night.actor_id) {
                                actor.is_alive = false;
                                if !game.night_casualties.contains(&night.actor_id) {
                                    game.night_casualties.push(night.actor_id.clone());
                                }
                            }
                        }
                    }
                }
            }
            Data::Chat(_) | Data::SyncPhase(_) => {
                // 聊天与阶段同步广播给全员
                broadcast_message(&state.peers, &msg).await;
            }
            Data::Identity(_) => {
                // 身份包广播或按需处理
                broadcast_message(&state.peers, &msg).await;
            }
            Data::BlankGuess(guess) => {
                // 收到客户端发送的白板猜词
                use tauri::Manager;
                let game_state = app_handle.state::<crate::game_engine::GameEngineState>();
                if let Err(e) = crate::game_engine::process_blank_guess_message(
                    &game_state,
                    state,
                    app_handle,
                    &guess.player_id,
                    &guess.word_a,
                    &guess.word_b,
                ).await {
                    warn!("处理白板猜词失败: {}", e);
                }
            }
            Data::Heartbeat(_) => {
                // 心跳包处理
            }
        }
    }
}

/// 处理单个远端客户端的连接 IO（读写拆分与优雅取消）
pub async fn handle_connection(
    _app_handle: AppHandle,
    state: NetworkState,
    stream: TcpStream,
    addr: SocketAddr,
    token: CancellationToken,
) {
    info!("建立与客户端 {} 的 TCP 会话", addr);
    let (read_half, write_half) = stream.into_split();
    let mut framed_read = FramedRead::new(read_half, ProtobufCodec::new());
    let mut framed_write = FramedWrite::new(write_half, ProtobufCodec::new());

    // 为该客户端创建专属的发送信箱
    let (client_tx, mut client_rx) = mpsc::channel::<GameMessage>(64);
    let mut connected_player_id: Option<String> = None;

    // 启动负责向该客户端写入数据的异步子任务
    let writer_token = token.clone();
    let writer_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = writer_token.cancelled() => {
                    break;
                }
                msg = client_rx.recv() => {
                    match msg {
                        Some(m) => {
                            if let Err(e) = framed_write.send(m).await {
                                warn!("向客户端 {} 发送数据包失败: {}", addr, e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // 负责从该客户端读取数据的异步循环
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                info!("客户端 {} 收到服务端关闭令牌，断开会话", addr);
                break;
            }
            item = framed_read.next() => {
                match item {
                    Some(Ok(msg)) => {
                        if let Some(ref data) = msg.data {
                            match data {
                                Data::JoinReq(req) => {
                                    connected_player_id = Some(req.player_id.clone());
                                    // 登记到全局 peers 信箱
                                    state.peers.lock().await.insert(req.player_id.clone(), client_tx.clone());
                                    // 转发给房主中央逻辑处理
                                    if let Some(host_tx) = state.host_sender.lock().await.as_ref() {
                                        let _ = host_tx.send(msg).await;
                                    }
                                }
                                Data::LeaveReq(req) => {
                                    info!("客户端 {} 发起主动离房请求: {}", addr, req.player_id);
                                    if let Some(host_tx) = state.host_sender.lock().await.as_ref() {
                                        let _ = host_tx.send(msg).await;
                                    }
                                    break;
                                }
                                _ => {
                                    if let Some(host_tx) = state.host_sender.lock().await.as_ref() {
                                        let _ = host_tx.send(msg).await;
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!("从客户端 {} 读取数据解码失败: {}", addr, e);
                        break;
                    }
                    None => {
                        info!("客户端 {} 连接已挂断", addr);
                        break;
                    }
                }
            }
        }
    }

    // 清理子任务与玩家状态
    writer_task.abort();
    if let Some(pid) = connected_player_id {
        info!("连接挂断，清理玩家状态: {}", pid);
        state.peers.lock().await.remove(&pid);
        state.player_profiles.lock().await.remove(&pid);

        let leave_msg = GameMessage {
            data: Some(Data::LeaveReq(LeaveRequest {
                player_id: pid.clone(),
            })),
        };
        if let Some(host_tx) = state.host_sender.lock().await.as_ref() {
            let _ = host_tx.send(leave_msg).await;
        }
    }
}

// ==========================================
// Tauri Commands (IPC 接口)
// ==========================================

/// 创建游戏房间 Command（房主端）
#[tauri::command]
pub async fn create_game_room_command(
    app_handle: AppHandle,
    state: State<'_, NetworkState>,
    port: Option<u16>,
    host_player_id: String,
    host_player_name: String,
    host_avatar: String,
) -> Result<(), String> {
    // 规避 E0521：将 state 复制后再传入异步块
    let state_clone = state.inner().clone();
    let port = port.unwrap_or(8888);

    // 1. 清理已有会话与重置游戏引擎状态
    close_internal(&state_clone).await;
    {
        use tauri::Manager;
        if let Some(engine_state) = app_handle.try_state::<crate::game_engine::GameEngineState>() {
            let mut game = engine_state.inner.lock().await;
            game.reset();
        }
    }

    // 2. 绑定 TCP 端口
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| format!("绑定端口 {} 失败: {}", bind_addr, e))?;
    info!("房主成功启动房间，监听端口: {}", bind_addr);

    // 3. 创建房间全局取消令牌
    let cancellation_token = CancellationToken::new();
    *state_clone.cancellation_token.lock().await = Some(cancellation_token.clone());

    // 启动局域网在线探针广播与组播通告服务 (mDNS / SSDP Advertiser)
    {
        use tauri::Manager;
        let engine_state = app_handle.try_state::<crate::game_engine::GameEngineState>().map(|s| s.inner().clone());
        crate::discovery::start_room_advertiser(
            port,
            host_player_name.clone(),
            host_avatar.clone(),
            state_clone.clone(),
            engine_state,
            cancellation_token.clone(),
        );
    }

    // 4. 房主“虚拟加入”闭环：
    // (1) 创建房主专属虚拟信箱存入 peers
    let (host_peer_tx, mut host_peer_rx) = mpsc::channel::<GameMessage>(64);
    state_clone
        .peers
        .lock()
        .await
        .insert(host_player_id.clone(), host_peer_tx);

    let host_profile = JoinRequest {
        player_id: host_player_id.clone(),
        player_name: host_player_name.clone(),
        avatar: host_avatar.clone(),
    };
    state_clone
        .player_profiles
        .lock()
        .await
        .insert(host_player_id.clone(), host_profile.clone());

    // (2) 启动后台任务：消费房主虚拟信箱接收的广播，派发给前端 Vue
    let host_app = app_handle.clone();
    let host_peer_token = cancellation_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = host_peer_token.cancelled() => {
                    info!("房主虚拟信箱监听已销毁");
                    break;
                }
                msg = host_peer_rx.recv() => {
                    match msg {
                        Some(m) => {
                            dispatch_message_to_frontend(&host_app, &m);
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // (3) 创建房主逻辑中心信箱 host_sender
    let (host_logic_tx, mut host_logic_rx) = mpsc::channel::<GameMessage>(64);
    *state_clone.host_sender.lock().await = Some(host_logic_tx);

    // (4) 启动中央逻辑调度路由后台任务
    let logic_state = state_clone.clone();
    let logic_token = cancellation_token.clone();
    let logic_app = app_handle.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = logic_token.cancelled() => {
                    info!("房主中央逻辑处理循环已销毁");
                    break;
                }
                msg = host_logic_rx.recv() => {
                    match msg {
                        Some(m) => {
                            handle_host_logic_message(&logic_state, &logic_app, m).await;
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // (5) 将房主自己的加入请求派发给自身前端
    let host_join_msg = GameMessage {
        data: Some(Data::JoinReq(host_profile)),
    };
    dispatch_message_to_frontend(&app_handle, &host_join_msg);

    // 5. 启动 accept 循环，必须放入 tokio::spawn 后台运行以防阻塞 Command
    let accept_token = cancellation_token.clone();
    let accept_state = state_clone.clone();
    let accept_app = app_handle.clone();
    tokio::spawn(async move {
        info!("房主连接接收服务开始运行");
        loop {
            tokio::select! {
                _ = accept_token.cancelled() => {
                    info!("TcpListener accept 循环退出");
                    break;
                }
                res = listener.accept() => {
                    match res {
                        Ok((socket, addr)) => {
                            let client_token = accept_token.clone();
                            let client_state = accept_state.clone();
                            let client_app = accept_app.clone();
                            tokio::spawn(async move {
                                handle_connection(client_app, client_state, socket, addr, client_token).await;
                            });
                        }
                        Err(e) => {
                            error!("Accept 连接出错: {}", e);
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

/// 关闭游戏房间 Command（房主端）
#[tauri::command]
pub async fn close_game_room_command(
    app_handle: AppHandle,
    state: State<'_, NetworkState>,
) -> Result<(), String> {
    let state_clone = state.inner().clone();
    close_internal(&state_clone).await;
    {
        use tauri::Manager;
        if let Some(engine_state) = app_handle.try_state::<crate::game_engine::GameEngineState>() {
            let mut game = engine_state.inner.lock().await;
            game.reset();
        }
    }
    let _ = app_handle.emit("room_closed", ());
    info!("房间已完全关闭，所有后台连接已释放，游戏引擎状态已重置");
    Ok(())
}

/// 加入游戏房间 Command（普通客户端）
#[tauri::command]
pub async fn join_game_room_command(
    app_handle: AppHandle,
    state: State<'_, NetworkState>,
    host_address: String,
    player_id: String,
    player_name: String,
    avatar: String,
) -> Result<(), String> {
    let state_clone = state.inner().clone();

    // 1. 清理已有连接并重置本地引擎状态
    close_internal(&state_clone).await;
    {
        use tauri::Manager;
        if let Some(engine_state) = app_handle.try_state::<crate::game_engine::GameEngineState>() {
            let mut game = engine_state.inner.lock().await;
            game.reset();
        }
    }

    // 2. 连接至房主
    info!("正在连接至房主: {}", host_address);
    let stream = TcpStream::connect(&host_address)
        .await
        .map_err(|e| format!("连接房主 {} 失败: {}", host_address, e))?;
    info!("成功连接至房主: {}", host_address);

    let (read_half, write_half) = stream.into_split();
    let mut framed_read = FramedRead::new(read_half, ProtobufCodec::new());
    let mut framed_write = FramedWrite::new(write_half, ProtobufCodec::new());

    // 3. 创建取消令牌
    let cancellation_token = CancellationToken::new();
    *state_clone.cancellation_token.lock().await = Some(cancellation_token.clone());

    // 4. 创建发往房主的信箱
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<GameMessage>(64);
    *state_clone.host_sender.lock().await = Some(outbound_tx);

    // 5. 立即向房主发送 JoinRequest
    let join_req = GameMessage {
        data: Some(Data::JoinReq(JoinRequest {
            player_id: player_id.clone(),
            player_name: player_name.clone(),
            avatar: avatar.clone(),
        })),
    };
    framed_write
        .send(join_req)
        .await
        .map_err(|e| format!("向房主发送入房请求失败: {}", e))?;

    // 6. 启动向房主发送数据的写入子任务
    let writer_token = cancellation_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = writer_token.cancelled() => {
                    break;
                }
                msg = outbound_rx.recv() => {
                    match msg {
                        Some(m) => {
                            if let Err(e) = framed_write.send(m).await {
                                warn!("向房主发送数据包错误: {}", e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // 7. 启动接收房主广播并派发至 Vue 前端的读取子任务
    let reader_app = app_handle.clone();
    let reader_token = cancellation_token.clone();
    let reader_state = state_clone.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = reader_token.cancelled() => {
                    info!("客户端连接循环已主动注销");
                    break;
                }
                item = framed_read.next() => {
                    match item {
                        Some(Ok(msg)) => {
                            dispatch_message_to_frontend(&reader_app, &msg);
                        }
                        Some(Err(e)) => {
                            warn!("读取房主数据包失败: {}", e);
                            break;
                        }
                        None => {
                            info!("房主已关闭连接");
                            let _ = reader_app.emit("room_closed", ());
                            break;
                        }
                    }
                }
            }
        }
        *reader_state.host_sender.lock().await = None;
    });

    Ok(())
}

/// 离开游戏房间 Command（客户端）
#[tauri::command]
pub async fn leave_game_room_command(
    app_handle: AppHandle,
    state: State<'_, NetworkState>,
    player_id: String,
) -> Result<(), String> {
    let state_clone = state.inner().clone();

    // 如果客户端处于已连接状态，向房主通知离开
    let leave_msg = GameMessage {
        data: Some(Data::LeaveReq(LeaveRequest {
            player_id: player_id.clone(),
        })),
    };

    if let Some(sender) = state_clone.host_sender.lock().await.as_ref() {
        let _ = sender.send(leave_msg).await;
    }

    close_internal(&state_clone).await;
    {
        use tauri::Manager;
        if let Some(engine_state) = app_handle.try_state::<crate::game_engine::GameEngineState>() {
            let mut game = engine_state.inner.lock().await;
            game.reset();
        }
    }
    let _ = app_handle.emit("player_left_self", player_id);
    Ok(())
}

/// 发送聊天发言 Command
#[tauri::command]
pub async fn send_chat_command(
    state: State<'_, NetworkState>,
    sender_id: String,
    text: String,
    is_system: bool,
) -> Result<(), String> {
    let state_clone = state.inner().clone();
    let chat_msg = GameMessage {
        data: Some(Data::Chat(ChatMessage {
            sender_id,
            text,
            is_system,
        })),
    };

    let sender = state_clone.host_sender.lock().await.clone();
    if let Some(tx) = sender {
        tx.send(chat_msg)
            .await
            .map_err(|e| format!("发送发言失败: {}", e))?;
        Ok(())
    } else {
        Err("当前未加入任何游戏房间".to_string())
    }
}

/// 获取房间当前所有玩家列表 Command
#[tauri::command]
pub async fn get_room_players_command(
    state: State<'_, NetworkState>,
) -> Result<Vec<JoinRequest>, String> {
    let profiles = state.player_profiles.lock().await;
    Ok(profiles.values().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_network_state_initialization() {
        let state = NetworkState::new();
        assert!(state.peers.lock().await.is_empty());
        assert!(state.host_sender.lock().await.is_none());
        assert!(state.cancellation_token.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_broadcast_and_unicast() {
        let state = NetworkState::new();
        let (tx1, mut rx1) = mpsc::channel::<GameMessage>(10);
        let (tx2, mut rx2) = mpsc::channel::<GameMessage>(10);

        state.peers.lock().await.insert("p1".to_string(), tx1);
        state.peers.lock().await.insert("p2".to_string(), tx2);

        // 广播测试
        let broadcast_msg = GameMessage {
            data: Some(Data::Chat(ChatMessage {
                sender_id: "host".to_string(),
                text: "广播通知".to_string(),
                is_system: true,
            })),
        };
        broadcast_message(&state.peers, &broadcast_msg).await;

        let res1 = rx1.recv().await.unwrap();
        let res2 = rx2.recv().await.unwrap();
        assert_eq!(res1, broadcast_msg);
        assert_eq!(res2, broadcast_msg);

        // 单播测试
        let unicast_msg = GameMessage {
            data: Some(Data::Chat(ChatMessage {
                sender_id: "host".to_string(),
                text: "私信".to_string(),
                is_system: false,
            })),
        };
        send_to_player(&state.peers, "p1", unicast_msg.clone()).await.unwrap();
        let res_p1 = rx1.recv().await.unwrap();
        assert_eq!(res_p1, unicast_msg);

        // 验证 p2 没有收到单播消息
        tokio::select! {
            _ = rx2.recv() => panic!("p2 不应该收到单播给 p1 的消息"),
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }

    #[tokio::test]
    async fn test_cancellation_cleanup() {
        let state = NetworkState::new();
        let token = CancellationToken::new();
        *state.cancellation_token.lock().await = Some(token.clone());

        let (tx, _rx) = mpsc::channel::<GameMessage>(10);
        state.peers.lock().await.insert("p1".to_string(), tx);

        assert!(!token.is_cancelled());
        close_internal(&state).await;

        assert!(token.is_cancelled());
        assert!(state.peers.lock().await.is_empty());
        assert!(state.host_sender.lock().await.is_none());
        assert!(state.cancellation_token.lock().await.is_none());
    }
}

