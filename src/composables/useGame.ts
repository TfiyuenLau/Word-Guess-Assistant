import { ref, reactive } from "vue";
import { safeInvoke as invoke } from "../utils/tauriBridge";
import { isTauri, isWeb, getPlatformType } from "../utils/platform";
import {
  GamePhase,
  IdentityAssignment,
  ChatMessage,
  Player,
  FullGameState,
  SettlementDetails,
  DiscoveredRoom,
} from "../types/game";
import { globalP2PNetwork, P2PMessage } from "../utils/p2pNetwork";
import { P2PGameEngine } from "../utils/p2pGameEngine";

// 默认头像候选池
const defaultAvatars = [
  "/avatar/characters/detective.svg",
  "/avatar/characters/ninja.svg",
  "/avatar/cyber/agent.svg",
  "/avatar/animals/cat.svg",
  "/avatar/animals/fox.svg",
];

// 生成默认唯一的玩家 ID
function getOrCreatePlayerId(): string {
  const key = "wga_player_id";
  let id = sessionStorage.getItem(key);
  if (!id) {
    id = "user_" + Math.random().toString(36).substring(2, 9);
    sessionStorage.setItem(key, id);
  }
  return id;
}

function getInitialPlayerName(): string {
  const sessionName = sessionStorage.getItem("wga_player_name");
  if (sessionName) return sessionName;
  const localName = localStorage.getItem("wga_player_name");
  if (localName) return localName;
  const randomSuffix = Math.random().toString(36).substring(2, 6);
  const name = `特工_${randomSuffix}`;
  sessionStorage.setItem("wga_player_name", name);
  return name;
}

function getInitialAvatar(): string {
  const sessionAvatar = sessionStorage.getItem("wga_player_avatar");
  if (sessionAvatar) return sessionAvatar;
  const localAvatar = localStorage.getItem("wga_player_avatar");
  if (localAvatar) return localAvatar;
  const randomAvatar = defaultAvatars[Math.floor(Math.random() * defaultAvatars.length)];
  sessionStorage.setItem("wga_player_avatar", randomAvatar);
  return randomAvatar;
}

// 全局单例响应式状态
const selfPlayer = reactive<Player>({
  id: getOrCreatePlayerId(),
  name: getInitialPlayerName(),
  avatar: getInitialAvatar(),
  is_alive: true,
});

const networkMode = ref<"p2p" | "tcp">("p2p");
const roomCode = ref("");
const isHost = ref(false);
const isInRoom = ref(false);
const roomAddress = ref("127.0.0.1:8888");
const roomPort = ref(8888);

const currentTheme = ref(localStorage.getItem("wga_theme") || "night");

const currentPhase = ref<GamePhase>(GamePhase.PREPARING);
const phaseExtraInfo = ref("等待房主配置题目并启动游戏");
const myIdentity = ref<IdentityAssignment | null>(null);
const players = ref<Player[]>([]);
const chatMessages = ref<ChatMessage[]>([]);
const currentRound = ref(1);
const winner = ref<string | null>(null);
const settlementDetails = ref<SettlementDetails | null>(null);
const playerMarks = ref<Record<string, string>>({}); // 玩家本地推测标记
const discoveredRooms = ref<DiscoveredRoom[]>([]);
const isDiscovering = ref(false);

let p2pEngine: P2PGameEngine | null = null;

export function useGame() {
  // 设置并持久化 DaisyUI 主题
  function setTheme(themeName: string) {
    currentTheme.value = themeName;
    document.documentElement.setAttribute("data-theme", themeName);
    localStorage.setItem("wga_theme", themeName);
  }

  // 更新并持久化个人资料
  function updateProfile(name: string, avatar: string) {
    selfPlayer.name = name;
    selfPlayer.avatar = avatar;
    sessionStorage.setItem("wga_player_name", name);
    sessionStorage.setItem("wga_player_avatar", avatar);
    localStorage.setItem("wga_player_name", name);
    localStorage.setItem("wga_player_avatar", avatar);
  }

  // 完全重置本地游戏状态
  function resetLocalGameState() {
    isInRoom.value = false;
    isHost.value = false;
    currentPhase.value = GamePhase.PREPARING;
    phaseExtraInfo.value = "等待房主配置题目并启动游戏";
    myIdentity.value = null;
    players.value = [];
    chatMessages.value = [];
    currentRound.value = 1;
    winner.value = null;
    settlementDetails.value = null;
    playerMarks.value = {};
  }

  function setPlayerMark(playerId: string, role: string) {
    if (!role) {
      delete playerMarks.value[playerId];
    } else {
      playerMarks.value[playerId] = role;
    }
  }

  function clearPlayerMarks() {
    playerMarks.value = {};
  }

  // -------------------------------------------------------------
  // WebRTC P2P 模式：基于 4~6 位数字房间号秒连（全平台与网页通用）
  // -------------------------------------------------------------

  async function createP2PRoom(code?: string) {
    try {
      resetLocalGameState();
      networkMode.value = "p2p";
      const targetCode = code?.trim() || Math.floor(1000 + Math.random() * 9000).toString();
      roomCode.value = targetCode;

      p2pEngine = new P2PGameEngine();
      p2pEngine.addPlayer({ ...selfPlayer });

      isHost.value = true;
      isInRoom.value = true;
      players.value = [{ ...selfPlayer }];
      chatMessages.value = [
        {
          sender_id: "SYSTEM",
          text: `P2P 房间创建成功！房间号：【${targetCode}】。网页或客户端玩家均可直接输入房间号秒连。同 Wi-Fi 下将自动建立局域网直连。`,
          is_system: true,
          timestamp: Date.now(),
        },
      ];

      // 绑定 P2P 网络事件
      globalP2PNetwork.onPeerJoin((peer) => {
        if (!isHost.value || !p2pEngine) return;
        p2pEngine.addPlayer(peer);
        syncP2PPlayers();

        const joinNotice: ChatMessage = {
          sender_id: "SYSTEM",
          text: `特工【${peer.name}】已成功进入房间。`,
          is_system: true,
          timestamp: Date.now(),
        };
        chatMessages.value.push(joinNotice);
        globalP2PNetwork.broadcast({ type: "CHAT_MESSAGE", senderId: "SYSTEM", payload: joinNotice });

        // 单播同步当前房间状态
        globalP2PNetwork.sendTo(peer.id, {
          type: "SYNC_GAME_STATE",
          senderId: "HOST",
          payload: p2pEngine.getSnapshot(),
        });
      });

      globalP2PNetwork.onPeerLeave((peerId) => {
        if (!isHost.value || !p2pEngine) return;
        const p = p2pEngine.players.get(peerId);
        const name = p?.name || "特工";
        p2pEngine.removePlayer(peerId);
        syncP2PPlayers();

        const leaveNotice: ChatMessage = {
          sender_id: "SYSTEM",
          text: `特工【${name}】已离开房间。`,
          is_system: true,
          timestamp: Date.now(),
        };
        chatMessages.value.push(leaveNotice);
        globalP2PNetwork.broadcast({ type: "CHAT_MESSAGE", senderId: "SYSTEM", payload: leaveNotice });
      });

      globalP2PNetwork.onMessage((msg: P2PMessage, senderId: string) => {
        if (!isHost.value || !p2pEngine) return;
        handleHostReceivedMessage(msg, senderId);
      });

      await globalP2PNetwork.createRoom(targetCode, { ...selfPlayer });
    } catch (err: any) {
      resetLocalGameState();
      throw new Error(err?.message || err?.toString() || "创建 P2P 房间失败");
    }
  }

  async function joinP2PRoom(code: string) {
    if (!code || !code.trim()) {
      throw new Error("请输入有效的房间号");
    }
    try {
      resetLocalGameState();
      networkMode.value = "p2p";
      const targetCode = code.trim();
      roomCode.value = targetCode;
      isHost.value = false;
      isInRoom.value = true;

      chatMessages.value = [
        {
          sender_id: "SYSTEM",
          text: `正在连接房间【${targetCode}】...`,
          is_system: true,
          timestamp: Date.now(),
        },
      ];

      globalP2PNetwork.onPeerLeave((peerId) => {
        if (peerId === "HOST") {
          alert("房主已离开房间，当前对局已解散。");
          leaveRoom();
        }
      });

      globalP2PNetwork.onMessage((msg: P2PMessage) => {
        handleClientReceivedMessage(msg);
      });

      await globalP2PNetwork.joinRoom(targetCode, { ...selfPlayer });
    } catch (err: any) {
      resetLocalGameState();
      throw new Error(err?.message || err?.toString() || "加入 P2P 房间失败");
    }
  }

  function syncP2PPlayers() {
    if (!p2pEngine) return;
    const list = Array.from(p2pEngine.players.values()).map((p) => ({
      id: p.id,
      name: p.name,
      avatar: p.avatar,
      is_alive: p.isAlive,
    }));
    players.value = list;
    globalP2PNetwork.broadcast({
      type: "SYNC_PLAYERS",
      senderId: "HOST",
      payload: list,
    });
  }

  function handleHostReceivedMessage(msg: P2PMessage, senderId: string) {
    if (!p2pEngine) return;

    switch (msg.type) {
      case "CHAT_MESSAGE": {
        chatMessages.value.push(msg.payload);
        globalP2PNetwork.broadcast({
          type: "CHAT_MESSAGE",
          senderId,
          payload: msg.payload,
        });
        break;
      }
      case "SUBMIT_VOTE": {
        p2pEngine.submitVote(senderId, msg.payload.targetId);
        break;
      }
      case "SUBMIT_NIGHT_ACTION": {
        p2pEngine.submitNightAction(senderId, msg.payload.targetId);
        break;
      }
      case "SUBMIT_BLANK_GUESS": {
        const ok = p2pEngine.submitBlankGuess(senderId, msg.payload.wordA, msg.payload.wordB);
        if (ok) {
          applyP2PSnapshot(p2pEngine.getSnapshot());
          globalP2PNetwork.broadcast({
            type: "SYNC_GAME_STATE",
            senderId: "HOST",
            payload: p2pEngine.getSnapshot(),
          });
        }
        break;
      }
    }
  }

  function handleClientReceivedMessage(msg: P2PMessage) {
    switch (msg.type) {
      case "SYNC_PLAYERS": {
        players.value = msg.payload;
        break;
      }
      case "SYNC_GAME_STATE": {
        applyP2PSnapshot(msg.payload);
        break;
      }
      case "IDENTITY_ASSIGNMENT": {
        myIdentity.value = msg.payload;
        break;
      }
      case "CHAT_MESSAGE": {
        chatMessages.value.push(msg.payload);
        break;
      }
      case "ROOM_DISBANDED": {
        alert("房主已解散房间。");
        leaveRoom();
        break;
      }
    }
  }

  function applyP2PSnapshot(state: FullGameState) {
    currentPhase.value = state.phase;
    currentRound.value = state.round;
    winner.value = state.winner || null;
    settlementDetails.value = state.settlement_details || null;
    if (state.players) {
      players.value = Object.values(state.players);
    }
  }

  // -------------------------------------------------------------
  // 局域网 TCP/IP 模式（原生 Listen-Server 离线模式）
  // -------------------------------------------------------------

  async function createTcpRoom(port = 8888) {
    try {
      resetLocalGameState();
      networkMode.value = "tcp";
      roomPort.value = port;
      await invoke("create_game_room_command", {
        port,
        hostPlayerId: selfPlayer.id,
        hostPlayerName: selfPlayer.name,
        hostAvatar: selfPlayer.avatar,
      });
      isHost.value = true;
      isInRoom.value = true;
      currentPhase.value = GamePhase.PREPARING;
      chatMessages.value = [
        {
          sender_id: "SYSTEM",
          text: `房间创建成功！监听端口：${port}。其他特工可通过局域网 IP 直连。`,
          is_system: true,
          timestamp: Date.now(),
        },
      ];
      players.value = [{ ...selfPlayer }];
    } catch (err: any) {
      throw new Error(err?.toString() || "创建房间失败");
    }
  }

  async function joinTcpRoom(address: string) {
    try {
      resetLocalGameState();
      networkMode.value = "tcp";
      roomAddress.value = address;
      await invoke("join_game_room_command", {
        hostAddress: address,
        playerId: selfPlayer.id,
        playerName: selfPlayer.name,
        avatar: selfPlayer.avatar,
      });
      isHost.value = false;
      isInRoom.value = true;
      currentPhase.value = GamePhase.PREPARING;
      chatMessages.value = [
        {
          sender_id: "SYSTEM",
          text: `已连接至房主：${address}`,
          is_system: true,
          timestamp: Date.now(),
        },
      ];
    } catch (err: any) {
      throw new Error(err?.toString() || "加入房间失败");
    }
  }

  // 退出房间（自动适配 P2P 与 TCP）
  async function leaveRoom() {
    try {
      if (networkMode.value === "p2p") {
        if (isHost.value) {
          globalP2PNetwork.broadcast({ type: "ROOM_DISBANDED", senderId: "HOST", payload: null });
        }
        globalP2PNetwork.close();
        p2pEngine = null;
      } else {
        if (isHost.value) {
          await invoke("close_game_room_command");
        } else {
          await invoke("leave_game_room_command", { playerId: selfPlayer.id });
        }
      }
    } catch (e) {
      console.warn("退出房间返回:", e);
    } finally {
      resetLocalGameState();
    }
  }

  // 发送聊天发言
  async function sendChat(text: string) {
    if (!text.trim()) return;

    if (networkMode.value === "p2p") {
      const msg: ChatMessage = {
        sender_id: selfPlayer.id,
        text: text.trim(),
        is_system: false,
        timestamp: Date.now(),
      };
      if (isHost.value) {
        chatMessages.value.push(msg);
        globalP2PNetwork.broadcast({ type: "CHAT_MESSAGE", senderId: selfPlayer.id, payload: msg });
      } else {
        chatMessages.value.push(msg);
        globalP2PNetwork.sendToHost({ type: "CHAT_MESSAGE", senderId: selfPlayer.id, payload: msg });
      }
    } else {
      await invoke("send_chat_command", {
        senderId: selfPlayer.id,
        text: text.trim(),
        isSystem: false,
      });
    }
  }

  // 房主启动游戏
  async function startGame(theme?: string, similarity?: string) {
    if (networkMode.value === "p2p") {
      if (!isHost.value || !p2pEngine) return;
      const assignments = p2pEngine.startGame(theme, similarity);

      // 提取自身身份
      myIdentity.value = assignments.get(selfPlayer.id) || null;

      // 单播分别下发专属身份
      assignments.forEach((assignment, targetId) => {
        if (targetId !== selfPlayer.id) {
          globalP2PNetwork.sendTo(targetId, {
            type: "IDENTITY_ASSIGNMENT",
            senderId: "HOST",
            payload: assignment,
          });
        }
      });

      // 广播全局快照
      applyP2PSnapshot(p2pEngine.getSnapshot());
      globalP2PNetwork.broadcast({
        type: "SYNC_GAME_STATE",
        senderId: "HOST",
        payload: p2pEngine.getSnapshot(),
      });

      const startMsg: ChatMessage = {
        sender_id: "SYSTEM",
        text: "游戏已启动！发词完成，请各位特工仔细阅读专属线索与词汇，注意隐藏身份。",
        is_system: true,
        timestamp: Date.now(),
      };
      chatMessages.value.push(startMsg);
      globalP2PNetwork.broadcast({ type: "CHAT_MESSAGE", senderId: "SYSTEM", payload: startMsg });
    } else {
      await invoke("start_game_command", {
        theme: theme || null,
        similarity: similarity || "极为接近",
      });
    }
  }

  // 房主推进阶段
  async function nextPhase() {
    if (networkMode.value === "p2p") {
      if (!isHost.value || !p2pEngine) return;
      const result = p2pEngine.advancePhase();

      applyP2PSnapshot(p2pEngine.getSnapshot());
      globalP2PNetwork.broadcast({
        type: "SYNC_GAME_STATE",
        senderId: "HOST",
        payload: p2pEngine.getSnapshot(),
      });

      if (result.announcement) {
        const announceMsg: ChatMessage = {
          sender_id: "SYSTEM",
          text: result.announcement,
          is_system: true,
          timestamp: Date.now(),
        };
        chatMessages.value.push(announceMsg);
        globalP2PNetwork.broadcast({ type: "CHAT_MESSAGE", senderId: "SYSTEM", payload: announceMsg });
      }
    } else {
      await invoke("next_phase_command");
    }
  }

  // 投票
  async function submitVote(targetId: string) {
    if (networkMode.value === "p2p") {
      if (isHost.value && p2pEngine) {
        p2pEngine.submitVote(selfPlayer.id, targetId);
      } else {
        globalP2PNetwork.sendToHost({
          type: "SUBMIT_VOTE",
          senderId: selfPlayer.id,
          payload: { targetId },
        });
      }
    } else {
      await invoke("submit_vote_command", {
        voterId: selfPlayer.id,
        targetId,
      });
    }
  }

  // 夜晚暗杀
  async function submitNightAction(targetId: string): Promise<string> {
    if (networkMode.value === "p2p") {
      if (isHost.value && p2pEngine) {
        p2pEngine.submitNightAction(selfPlayer.id, targetId);
      } else {
        globalP2PNetwork.sendToHost({
          type: "SUBMIT_NIGHT_ACTION",
          senderId: selfPlayer.id,
          payload: { targetId },
        });
      }
      return "暗夜行动已提交";
    } else {
      return await invoke("submit_night_action_command", {
        actorId: selfPlayer.id,
        targetId,
      });
    }
  }

  // 白板两方词终极猜词
  async function submitBlankGuess(wordA: string, wordB: string): Promise<boolean> {
    if (networkMode.value === "p2p") {
      if (isHost.value && p2pEngine) {
        const ok = p2pEngine.submitBlankGuess(selfPlayer.id, wordA, wordB);
        if (ok) {
          applyP2PSnapshot(p2pEngine.getSnapshot());
          globalP2PNetwork.broadcast({
            type: "SYNC_GAME_STATE",
            senderId: "HOST",
            payload: p2pEngine.getSnapshot(),
          });
        }
        return ok;
      } else {
        globalP2PNetwork.sendToHost({
          type: "SUBMIT_BLANK_GUESS",
          senderId: selfPlayer.id,
          payload: { wordA, wordB },
        });
        return true;
      }
    } else {
      return await invoke("submit_blank_guess_command", {
        playerId: selfPlayer.id,
        wordA,
        wordB,
      });
    }
  }

  // 局域网探针发现
  async function discoverRooms(timeoutMs: number = 1200): Promise<DiscoveredRoom[]> {
    isDiscovering.value = true;
    try {
      const rooms = await invoke<DiscoveredRoom[]>("discover_rooms_command", { timeoutMs });
      if (rooms) discoveredRooms.value = rooms;
      return rooms || [];
    } catch (e) {
      console.warn("探测局域网房间失败:", e);
      return [];
    } finally {
      isDiscovering.value = false;
    }
  }

  // 重新开始游戏
  async function restartGame() {
    if (networkMode.value === "p2p") {
      if (!isHost.value || !p2pEngine) return;
      p2pEngine.reset();
      applyP2PSnapshot(p2pEngine.getSnapshot());
      globalP2PNetwork.broadcast({
        type: "SYNC_GAME_STATE",
        senderId: "HOST",
        payload: p2pEngine.getSnapshot(),
      });
    } else {
      await invoke("restart_game_command");
    }
    currentPhase.value = GamePhase.PREPARING;
    currentRound.value = 1;
    winner.value = null;
    settlementDetails.value = null;
    myIdentity.value = null;
    phaseExtraInfo.value = "游戏已重置，等待重新配置。";
  }

  // 查询完整游戏状态快照 (仅 TCP 模式轮询或初始同步)
  async function refreshGameState() {
    if (networkMode.value === "p2p") return;
    try {
      const state = await invoke<FullGameState>("get_game_state_command");
      if (state) {
        currentPhase.value = state.phase;
        currentRound.value = state.round;
        winner.value = state.winner || null;
        settlementDetails.value = state.settlement_details || null;
        if (state.players && Object.keys(state.players).length > 0) {
          players.value = Object.values(state.players);
          const self = state.players[selfPlayer.id];
          if (self && self.role) {
            myIdentity.value = {
              role: self.role,
              word: self.word || "",
              hint: self.hint || "",
            };
          }
        }
      }
    } catch (e) {
      console.warn("获取游戏状态失败:", e);
    }
  }

  return {
    selfPlayer,
    networkMode,
    roomCode,
    isHost,
    isInRoom,
    roomAddress,
    roomPort,
    currentTheme,
    currentPhase,
    phaseExtraInfo,
    myIdentity,
    players,
    chatMessages,
    currentRound,
    winner,
    settlementDetails,
    playerMarks,
    setPlayerMark,
    clearPlayerMarks,
    setTheme,
    updateProfile,
    createP2PRoom,
    joinP2PRoom,
    createTcpRoom,
    joinTcpRoom,
    leaveRoom,
    sendChat,
    startGame,
    nextPhase,
    submitVote,
    submitNightAction,
    submitBlankGuess,
    discoveredRooms,
    isDiscovering,
    discoverRooms,
    restartGame,
    refreshGameState,
    isTauri: isTauri(),
    isWeb: isWeb(),
    platform: getPlatformType(),
  };
}
