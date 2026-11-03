<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useGame } from "./composables/useGame";
import { DiscoveredRoom } from "./types/game";
import Game from "./components/Game.vue";
import RuleModal from "./components/modals/RuleModal.vue";
import {
  getCustomSignalingUrl,
  setCustomSignalingUrl,
  DEFAULT_SIGNALING_URL,
} from "./utils/p2pSignaling";

const {
  selfPlayer,
  isInRoom,
  currentTheme,
  setTheme,
  updateProfile,
  createP2PRoom,
  joinP2PRoom,
  createTcpRoom,
  joinTcpRoom,
  discoveredRooms,
  isDiscovering,
  discoverRooms,
} = useGame();

// P2P 房间号状态
const p2pRoomCodeInput = ref("");

// 传统 TCP/IP 离线输入临时状态
const playerNameInput = ref(selfPlayer.name);
const createPortInput = ref(8888);
const joinIpInput = ref("127.0.0.1");
const joinPortInput = ref(8888);

const errorMessage = ref("");
const isSubmitting = ref(false);

// 模态框状态
const isAvatarModalOpen = ref(false);
const isRuleModalOpen = ref(false);
const isSettingsModalOpen = ref(false);
const selectedCategory = ref("characters");
const customSignalingInput = ref(getCustomSignalingUrl());

// DaisyUI 精选主题列表
const themes = [
  { id: "light", name: "清爽浅色 (Light)" },
  { id: "dark", name: "深邃暗黑 (Dark)" },
];

// 分类头像元数据（3 大阵营 15 款矢量 SVG）
const avatarCategories = [
  {
    name: "characters",
    title: "Characters",
    avatars: [
      { id: "detective", name: "Detective", path: "/avatar/characters/detective.svg" },
      { id: "ninja", name: "Ninja", path: "/avatar/characters/ninja.svg" },
      { id: "wizard", name: "Wizard", path: "/avatar/characters/wizard.svg" },
      { id: "robot", name: "Robot", path: "/avatar/characters/robot.svg" },
      { id: "hero", name: "Hero", path: "/avatar/characters/hero.svg" },
    ],
  },
  {
    name: "cyber",
    title: "Cyber",
    avatars: [
      { id: "agent", name: "Agent", path: "/avatar/cyber/agent.svg" },
      { id: "hacker", name: "Hacker", path: "/avatar/cyber/hacker.svg" },
      { id: "cyborg", name: "Cyborg", path: "/avatar/cyber/cyborg.svg" },
      { id: "neon", name: "Neon", path: "/avatar/cyber/neon.svg" },
      { id: "glitch", name: "Glitch", path: "/avatar/cyber/glitch.svg" },
    ],
  },
  {
    name: "animals",
    title: "Animals",
    avatars: [
      { id: "cat", name: "Cat", path: "/avatar/animals/cat.svg" },
      { id: "fox", name: "Fox", path: "/avatar/animals/fox.svg" },
      { id: "panda", name: "Panda", path: "/avatar/animals/panda.svg" },
      { id: "bear", name: "Bear", path: "/avatar/animals/bear.svg" },
      { id: "rabbit", name: "Rabbit", path: "/avatar/animals/rabbit.svg" },
    ],
  },
];

let unlistenDiscover: UnlistenFn | null = null;

onMounted(async () => {
  // 1. 初始化全局主题
  const validTheme = themes.some((t) => t.id === currentTheme.value)
    ? currentTheme.value
    : "night";
  setTheme(validTheme);

  // 2. 检测 URL 是否携带 #room=XXXX 邀请参数
  if (window.location.hash) {
    const match = window.location.hash.match(/room=(\w+)/i);
    if (match && match[1]) {
      p2pRoomCodeInput.value = match[1];
    }
  }

  // 3. 注册局域网房间探针实时发现监听器
  try {
    unlistenDiscover = await listen<DiscoveredRoom>("lan_room_discovered", (event) => {
      const room = event.payload;
      const idx = discoveredRooms.value.findIndex(
        (r) => r.ip === room.ip && r.port === room.port
      );
      if (idx >= 0) {
        discoveredRooms.value[idx] = room;
      } else {
        discoveredRooms.value.push(room);
      }
    });
  } catch (err) {
    console.warn("注册局域网发现监听失败:", err);
  }

  // 4. 自动发射探针扫描局域网在线房间
  discoverRooms(1500);
});

onUnmounted(() => {
  if (unlistenDiscover) {
    unlistenDiscover();
    unlistenDiscover = null;
  }
});

// 手动触发扫描局域网房间
async function handleDiscoverRooms() {
  errorMessage.value = "";
  try {
    await discoverRooms(1500);
  } catch (err: any) {
    console.warn("扫描房间出错:", err);
  }
}

// 点击局域网卡片填充 IP 与端口
function quickSelectRoom(room: DiscoveredRoom) {
  joinIpInput.value = room.ip;
  joinPortInput.value = room.port;
}

// 一键直连加入发现的房间
async function quickJoinRoom(room: DiscoveredRoom) {
  joinIpInput.value = room.ip;
  joinPortInput.value = room.port;
  await handleJoinTcpRoom();
}

// 选择头像
function selectAvatar(path: string) {
  updateProfile(playerNameInput.value.trim() || selfPlayer.name, path);
}

// 确认昵称变动
function onNameChange() {
  const name = playerNameInput.value.trim();
  if (name) {
    updateProfile(name, selfPlayer.avatar);
  } else {
    playerNameInput.value = selfPlayer.name;
  }
}

// --- P2P 模式操作 ---

async function handleCreateP2PRoom() {
  errorMessage.value = "";
  onNameChange();
  isSubmitting.value = true;
  try {
    await createP2PRoom();
  } catch (err: any) {
    errorMessage.value = err.message || "创建 P2P 房间失败";
  } finally {
    isSubmitting.value = false;
  }
}

async function handleJoinP2PRoom() {
  errorMessage.value = "";
  onNameChange();
  const code = p2pRoomCodeInput.value.trim();
  if (!code) {
    errorMessage.value = "请输入 4 位数字房间号";
    return;
  }
  isSubmitting.value = true;
  try {
    await joinP2PRoom(code);
  } catch (err: any) {
    errorMessage.value = err.message || "加入 P2P 房间失败";
  } finally {
    isSubmitting.value = false;
  }
}

// --- TCP/IP 离线模式操作 ---

async function handleCreateTcpRoom() {
  errorMessage.value = "";
  onNameChange();
  isSubmitting.value = true;
  try {
    await createTcpRoom(createPortInput.value || 8888);
  } catch (err: any) {
    errorMessage.value = err.message || "创建房间失败";
  } finally {
    isSubmitting.value = false;
  }
}

async function handleJoinTcpRoom() {
  errorMessage.value = "";
  onNameChange();

  let ip = joinIpInput.value.trim();
  let port = joinPortInput.value;

  if (!ip) {
    errorMessage.value = "请输入房主 IP 地址";
    return;
  }

  if (ip.includes(":")) {
    const parts = ip.split(":");
    ip = parts[0].trim();
    if (parts[1]) {
      const parsedPort = parseInt(parts[1].trim(), 10);
      if (!isNaN(parsedPort)) {
        port = parsedPort;
        joinPortInput.value = parsedPort;
      }
    }
  }

  if (!port || port < 1 || port > 65535) {
    errorMessage.value = "请输入合法的端口号 (1-65535)";
    return;
  }

  const address = `${ip}:${port}`;
  isSubmitting.value = true;
  try {
    await joinTcpRoom(address);
  } catch (err: any) {
    errorMessage.value = err.message || "加入房间失败";
  } finally {
    isSubmitting.value = false;
  }
}

function handleSaveSettings() {
  if (customSignalingInput.value.trim()) {
    setCustomSignalingUrl(customSignalingInput.value.trim());
  }
  isSettingsModalOpen.value = false;
}

function handleResetSettings() {
  customSignalingInput.value = DEFAULT_SIGNALING_URL;
  setCustomSignalingUrl(DEFAULT_SIGNALING_URL);
}
</script>

<template>
  <!-- 如果已在房间内，展示游戏主界面 -->
  <Game v-if="isInRoom" />

  <!-- 否则展示大厅 / 个人配置 / 创房与入房入口 -->
  <div v-else class="h-screen flex flex-col overflow-hidden bg-base-200">
    <!-- 顶部导航栏 -->
    <header class="navbar bg-base-100 shadow-md px-3 sm:px-4 flex justify-between items-center z-10 flex-none min-h-[50px] border-b border-base-300">
      <div class="flex items-center gap-1.5 min-w-0">
        <span class="text-xl">🕵️</span>
        <div class="flex flex-col min-w-0">
          <h1 class="text-base font-black tracking-wide bg-gradient-to-r from-primary to-secondary bg-clip-text text-transparent truncate">
            卧底猜词助手
          </h1>
          <span class="text-[9px] text-base-content/60 font-mono tracking-tighter leading-none hidden sm:block">WORD GUESS ASSISTANT</span>
        </div>
        <span class="badge badge-primary text-[10px] hidden xs:inline-flex ml-1 font-bold">P2P & LAN</span>
      </div>

      <div class="flex items-center gap-1.5 shrink-0">
        <!-- 设置按钮 -->
        <button
          class="btn btn-xs sm:btn-sm btn-ghost btn-circle"
          title="网络与信令设置"
          @click="isSettingsModalOpen = true"
        >
          ⚙️
        </button>

        <!-- 主题切换下拉菜单 -->
        <div class="dropdown dropdown-end shrink-0">
          <div tabindex="0" role="button" class="btn btn-xs sm:btn-sm btn-outline gap-1">
            <span>🎨</span>
            <span class="capitalize text-xs">{{ currentTheme }}</span>
            <span class="text-[10px]">▼</span>
          </div>
          <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-50 w-44 p-2 shadow-xl border border-base-300">
            <li v-for="t in themes" :key="t.id">
              <button
                :class="{ 'active': currentTheme === t.id }"
                @click="setTheme(t.id)"
              >
                {{ t.name }}
              </button>
            </li>
          </ul>
        </div>
      </div>
    </header>

    <!-- 主体可滚动区域 -->
    <main class="flex-1 overflow-y-auto px-4 py-4 space-y-4 max-w-md mx-auto w-full">
      <!-- 错误提示 Toast -->
      <div v-if="errorMessage" class="alert alert-error shadow-lg py-2.5 px-4 text-xs transition-all">
        <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-5 w-5" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span class="flex-1">{{ errorMessage }}</span>
        <button class="btn btn-xs btn-ghost" @click="errorMessage = ''">✕</button>
      </div>

      <!-- 第一部分：玩家个人名片 -->
      <section class="card bg-base-100 shadow-md border border-base-300 p-4">
        <div class="flex items-center gap-3.5">
          <!-- 当前头像预览与更换入口 -->
          <div
            class="relative group cursor-pointer shrink-0"
            title="点击更换形象"
            @click="isAvatarModalOpen = true"
          >
            <div class="avatar">
              <div class="w-16 h-16 rounded-full ring-2 ring-primary ring-offset-base-100 ring-offset-2 overflow-hidden shadow bg-base-200">
                <img :src="selfPlayer.avatar" alt="Current Avatar" class="w-full h-full object-cover" />
              </div>
            </div>
            <span class="badge badge-primary badge-xs absolute -bottom-1 -right-1 shadow font-bold">
              更换
            </span>
          </div>

          <!-- 昵称编辑输入框 -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between mb-1">
              <label class="text-xs font-bold text-base-content/80">特工代号</label>
              <button
                class="text-[11px] text-primary hover:underline font-semibold flex items-center gap-0.5"
                @click="isAvatarModalOpen = true"
              >
                <span>更换头像</span>
                <span>›</span>
              </button>
            </div>
            <div class="join w-full">
              <input
                v-model="playerNameInput"
                type="text"
                maxlength="16"
                placeholder="请输入玩家代号..."
                class="input input-sm input-bordered join-item w-full text-xs font-medium"
                @blur="onNameChange"
                @keyup.enter="onNameChange"
              />
              <button class="btn btn-sm btn-neutral join-item text-xs px-3" @click="onNameChange">
                保存
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- 游戏规则说明长方形按钮 -->
      <button
        class="btn btn-outline btn-primary w-full shadow-sm flex items-center justify-between px-4 h-11 transition-all hover:scale-[1.01]"
        title="点击查看游戏规则与身份说明"
        @click="isRuleModalOpen = true"
      >
        <div class="flex items-center gap-2">
          <span class="text-base sm:text-lg">📖</span>
          <span class="font-bold text-xs sm:text-sm">游戏规则与身份说明</span>
        </div>
        <span class="text-xs text-primary/80 font-medium flex items-center gap-0.5">
          <span>查看说明</span>
          <span>›</span>
        </span>
      </button>

      <!-- 第二部分：核心推荐模式：WebRTC P2P 全平台房间号联机 -->
      <section class="card bg-base-100 shadow-md border-2 border-primary/30 hover:border-primary/60 transition-all p-4 space-y-3.5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5">
            <span class="text-lg">🌐</span>
            <h3 class="font-bold text-sm text-primary">WebRTC P2P 房间号快速联机</h3>
          </div>
          <span class="badge badge-primary badge-xs font-bold">推荐全平台</span>
        </div>

        <p class="text-xs text-base-content/70 leading-relaxed bg-base-200/50 p-2.5 rounded-lg border border-base-200">
          ✨ 纯网页、安卓手机与电脑客户端零门槛互联。同 Wi-Fi 局域网下自动切换内网直连（0 流量延迟）。
        </p>

        <!-- 创建房间与加入房间卡片 -->
        <div class="space-y-3">
          <!-- 房主一键创建房间 -->
          <button
            class="btn btn-sm btn-primary w-full shadow-md shadow-primary/20 flex items-center justify-center gap-1.5"
            :disabled="isSubmitting"
            @click="handleCreateP2PRoom"
          >
            <span v-if="isSubmitting" class="loading loading-spinner loading-xs"></span>
            <span>🎲 创建新房间 (生成专属房间号)</span>
          </button>

          <!-- 分隔线 -->
          <div class="divider my-1 text-[11px] text-base-content/40">或者输入房间号加入</div>

          <!-- 玩家输入房间号加入 -->
          <div class="join w-full">
            <input
              v-model="p2pRoomCodeInput"
              type="text"
              maxlength="8"
              class="input input-sm input-bordered join-item w-full font-mono text-xs tracking-wider"
              placeholder="输入 4 位房间号 (如 8821)..."
              @keyup.enter="handleJoinP2PRoom"
            />
            <button
              class="btn btn-sm btn-secondary join-item px-4 text-xs font-bold"
              :disabled="isSubmitting"
              @click="handleJoinP2PRoom"
            >
              <span v-if="isSubmitting" class="loading loading-spinner loading-xs"></span>
              <span>🚀 快速加入</span>
            </button>
          </div>
        </div>
      </section>

      <!-- 第三部分：折叠高级模式：局域网 IP 直连 (传统离线 Listen-Server) -->
      <details class="collapse collapse-arrow bg-base-100 border border-base-300 shadow-sm rounded-box">
        <summary class="collapse-title text-xs font-bold text-base-content/80 flex items-center gap-1.5 py-3">
          <span>📶</span>
          <span>局域网 IP 直连 (高级离线模式)</span>
          <span class="badge badge-ghost badge-xs text-[10px]">纯本地断网专用</span>
        </summary>
        <div class="collapse-content space-y-4 pt-1">
          <p class="text-[11px] text-base-content/60 leading-relaxed">
            在无外网接入的极端离线环境（如无网热点/露营），可通过原生底层 TCP 端口与局域网 IP 直连对决。
          </p>

          <!-- 房主模式：创建 TCP 房间 -->
          <div class="p-3 bg-base-200/50 rounded-lg border border-base-200 space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-xs font-bold text-base-content/80">房主建房 (TCP)</span>
              <span class="badge badge-outline badge-xs">端口监听</span>
            </div>
            <div class="join w-full">
              <input
                v-model.number="createPortInput"
                type="number"
                min="1024"
                max="65535"
                class="input input-sm input-bordered join-item w-full font-mono text-xs"
                placeholder="8888"
              />
              <button
                class="btn btn-sm btn-neutral join-item text-xs"
                :disabled="isSubmitting"
                @click="handleCreateTcpRoom"
              >
                开启监听
              </button>
            </div>
          </div>

          <!-- 玩家模式：直连 IP 加入 -->
          <div class="p-3 bg-base-200/50 rounded-lg border border-base-200 space-y-2">
            <span class="text-xs font-bold text-base-content/80">玩家直连 IP 加入</span>
            <div class="grid grid-cols-3 gap-2">
              <div class="form-control col-span-2">
                <input
                  v-model="joinIpInput"
                  type="text"
                  class="input input-sm input-bordered w-full font-mono text-xs"
                  placeholder="192.168.1.100"
                />
              </div>
              <div class="form-control col-span-1">
                <input
                  v-model.number="joinPortInput"
                  type="number"
                  min="1"
                  max="65535"
                  class="input input-sm input-bordered w-full font-mono text-xs"
                  placeholder="8888"
                />
              </div>
            </div>
            <button
              class="btn btn-sm btn-outline btn-secondary w-full text-xs"
              :disabled="isSubmitting"
              @click="handleJoinTcpRoom"
            >
              直连 IP 进入对决
            </button>
          </div>

          <!-- 局域网在线房间探测列表 -->
          <div class="pt-2 border-t border-base-200 space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-xs font-bold text-base-content/80 flex items-center gap-1">
                <span>📡</span>
                <span>局域网探针自动发现</span>
                <span v-if="discoveredRooms.length > 0" class="badge badge-secondary badge-xs font-mono font-bold">
                  {{ discoveredRooms.length }}
                </span>
              </span>
              <button
                class="btn btn-ghost btn-xs text-[11px] text-secondary gap-1 px-1.5"
                :disabled="isDiscovering"
                @click="handleDiscoverRooms"
              >
                <span :class="{ 'animate-spin': isDiscovering }">🔄</span>
                <span>刷新</span>
              </button>
            </div>

            <div v-if="discoveredRooms.length === 0" class="py-2 text-center text-[11px] text-base-content/50">
              未检测到活跃广播。确保房主在同 Wi-Fi 开启了 TCP 房间。
            </div>

            <div v-else class="space-y-1.5 max-h-36 overflow-y-auto">
              <div
                v-for="room in discoveredRooms"
                :key="`${room.ip}:${room.port}`"
                class="flex items-center justify-between p-2 bg-base-200 rounded-lg text-xs"
                @click="quickSelectRoom(room)"
              >
                <div class="truncate">
                  <span class="font-bold">{{ room.host_name }}</span>
                  <span class="text-base-content/50 text-[10px] ml-1.5">{{ room.ip }}:{{ room.port }}</span>
                </div>
                <button class="btn btn-xs btn-secondary" @click.stop="quickJoinRoom(room)">加入</button>
              </div>
            </div>
          </div>
        </div>
      </details>
    </main>

    <!-- 底部版权信息 -->
    <footer class="footer footer-center py-2 px-4 bg-base-100 text-base-content/40 border-t border-base-300 text-[10px] flex-none">
      <p>Word Guess Assistant © 2026 — WebRTC P2P & Serverless LAN 猜词博弈引擎</p>
    </footer>

    <!-- 网络信令设置模态框 (Settings Modal) -->
    <dialog class="modal modal-bottom sm:modal-middle" :class="{ 'modal-open': isSettingsModalOpen }">
      <div class="modal-box max-w-sm p-4 bg-base-100 border border-base-300 shadow-2xl space-y-3">
        <div class="flex items-center justify-between border-b border-base-200 pb-2">
          <h3 class="font-bold text-sm flex items-center gap-1.5 text-primary">
            <span>⚙️</span> 网络与信令配置
          </h3>
          <button class="btn btn-xs btn-circle btn-ghost" @click="isSettingsModalOpen = false">✕</button>
        </div>

        <div class="form-control space-y-1">
          <label class="label py-0.5">
            <span class="label-text text-xs font-semibold">P2P 握手信令节点 (WSS / MQTT)</span>
          </label>
          <input
            v-model="customSignalingInput"
            type="text"
            class="input input-sm input-bordered w-full font-mono text-xs"
            placeholder="wss://broker.emqx.io:8084/mqtt"
          />
          <span class="text-[10px] text-base-content/60 leading-normal">
            国内推荐默认节点，或可自行配置国内私有 WSS 信令网关。仅在握手瞬间发送一次，后续全为 P2P 直连。
          </span>
        </div>

        <div class="p-2.5 bg-base-200 rounded-lg text-[10px] text-base-content/70 space-y-1">
          <div class="font-bold text-base-content/80">国内高可用 STUN 穿透矩阵已启用：</div>
          <div>• 腾讯云 STUN: stun.qq.com:3478</div>
          <div>• 小米 STUN: stun.miwifi.com:3478</div>
          <div>• 哔哩哔哩 STUN: stun.chat.bilibili.com:3478</div>
        </div>

        <div class="modal-action pt-2 border-t border-base-200 flex justify-between">
          <button class="btn btn-xs btn-ghost text-xs" @click="handleResetSettings">恢复默认</button>
          <div class="space-x-2">
            <button class="btn btn-xs btn-outline" @click="isSettingsModalOpen = false">取消</button>
            <button class="btn btn-xs btn-primary px-3" @click="handleSaveSettings">保存配置</button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="isSettingsModalOpen = false">关闭</button>
      </form>
    </dialog>

    <!-- 头像选择悬浮模态框 (Avatar Selection Modal) -->
    <dialog class="modal modal-bottom sm:modal-middle" :class="{ 'modal-open': isAvatarModalOpen }">
      <div class="modal-box max-w-sm p-4 bg-base-100 border border-base-300 shadow-2xl">
        <div class="flex items-center justify-between mb-3 border-b border-base-200 pb-2">
          <h3 class="font-bold text-sm flex items-center gap-1.5 text-primary">
            <span>🎭</span> 选择头像
          </h3>
          <button class="btn btn-xs btn-circle btn-ghost" @click="isAvatarModalOpen = false">✕</button>
        </div>

        <!-- 分类 Tab 切换 -->
        <div class="tabs tabs-boxed mb-3 p-1 bg-base-200 flex justify-center">
          <a
            v-for="cat in avatarCategories"
            :key="cat.name"
            class="tab tab-xs flex-1"
            :class="{ 'tab-active font-bold': selectedCategory === cat.name }"
            @click="selectedCategory = cat.name"
          >
            {{ cat.title }}
          </a>
        </div>

        <!-- 头像网格 -->
        <div class="py-1">
          <div
            v-for="cat in avatarCategories"
            v-show="selectedCategory === cat.name"
            :key="cat.name"
            class="grid grid-cols-5 gap-2.5 justify-items-center"
          >
            <div
              v-for="av in cat.avatars"
              :key="av.id"
              class="flex flex-col items-center cursor-pointer group"
              @click="selectAvatar(av.path)"
            >
              <div
                class="avatar rounded-full p-0.5 transition-all duration-150 group-hover:scale-110"
                :class="selfPlayer.avatar === av.path ? 'ring-2 ring-primary scale-105 bg-primary/20 shadow-md shadow-primary/30' : 'ring-1 ring-base-300 hover:ring-primary/60'"
              >
                <div class="w-11 h-11 rounded-full overflow-hidden bg-base-200">
                  <img :src="av.path" :alt="av.name" class="w-full h-full object-cover" />
                </div>
              </div>
              <span
                class="text-[10px] mt-1 truncate max-w-full text-center transition-colors"
                :class="selfPlayer.avatar === av.path ? 'text-primary font-bold' : 'text-base-content/70'"
              >
                {{ av.name }}
              </span>
            </div>
          </div>
        </div>

        <div class="modal-action mt-3 pt-2 border-t border-base-200 flex justify-end">
          <button class="btn btn-sm btn-primary px-5" @click="isAvatarModalOpen = false">
            完成
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="isAvatarModalOpen = false">关闭</button>
      </form>
    </dialog>

    <!-- 游戏规则与身份指引悬浮模态框 (Rule Modal) -->
    <RuleModal v-model:is-open="isRuleModalOpen" />
  </div>
</template>
