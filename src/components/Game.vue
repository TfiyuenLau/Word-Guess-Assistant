<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useGame } from "../composables/useGame";
import {
  GamePhase,
  ROLE_GOOD,
  ROLE_SPY,
  ROLE_ANGEL,
  ROLE_BLANK,
  ROLE_SPECTATOR,
  Player,
} from "../types/game";

// 引入模块化弹窗子组件
import IdentityModal from "./modals/IdentityModal.vue";
import HostConsoleModal from "./modals/HostConsoleModal.vue";
import VoteModal from "./modals/VoteModal.vue";
import NightActionModal from "./modals/NightActionModal.vue";
import BlankGuessModal from "./modals/BlankGuessModal.vue";
import SettlementModal from "./modals/SettlementModal.vue";
import PhaseNoticeModal from "./modals/PhaseNoticeModal.vue";
import PlayerMarkModal from "./modals/PlayerMarkModal.vue";

const {
  selfPlayer,
  networkMode,
  roomCode,
  isHost,
  roomPort,
  currentPhase,
  phaseExtraInfo,
  myIdentity,
  players,
  chatMessages,
  currentRound,
  playerMarks,
  setPlayerMark,
  clearPlayerMarks,
  leaveRoom,
  sendChat,
  refreshGameState,
} = useGame();

// 界面临时状态
const inputText = ref("");
const chatScrollContainer = ref<HTMLElement | null>(null);

// 模态弹窗显隐控制
const isIdentityModalOpen = ref(false);
const isHostModalOpen = ref(false);
const isVoteModalOpen = ref(false);
const isNightModalOpen = ref(false);
const isBlankGuessModalOpen = ref(false);
const isSettlementModalOpen = ref(false);
const isPhaseNoticeOpen = ref(false);
const isPlayerMarkModalOpen = ref(false);
const markedTargetPlayer = ref<Player | null>(null);

// 投票状态与私密显示
const hasVoted = ref(false);
const votedTargetName = ref("");

// 监听器销毁函数集（幂等管理）
let unlistenFns: UnlistenFn[] = [];

// 阶段展示映射
const phaseTitleMap: Record<number, string> = {
  [GamePhase.PREPARING]: "准备中",
  [GamePhase.DESCRIPTION]: "一句话描述",
  [GamePhase.VOTING]: "讨论与投票",
  [GamePhase.NIGHT_ACTION]: "黑夜暗杀",
  [GamePhase.SETTLEMENT]: "终局结算",
  [GamePhase.BLANK_GUESS]: "白板猜词",
};

const phaseBadgeClassMap: Record<number, string> = {
  [GamePhase.PREPARING]: "badge-neutral",
  [GamePhase.DESCRIPTION]: "badge-info",
  [GamePhase.VOTING]: "badge-warning",
  [GamePhase.NIGHT_ACTION]: "badge-error",
  [GamePhase.SETTLEMENT]: "badge-success",
  [GamePhase.BLANK_GUESS]: "badge-warning",
};

// 当前玩家存活状态
const isSelfAlive = computed(() => {
  if (myIdentity.value?.role === ROLE_SPECTATOR) return false;
  const me = players.value.find((p) => p.id === selfPlayer.id);
  return me ? me.is_alive : true;
});

// 输入框禁用规则：
// 1. 玩家出局则禁止发言；
// 2. 准备阶段（PREPARING）自由聊天，不禁用；
// 3. 第一轮描述阶段（DESCRIPTION 且 round===1）允许发言；
// 4. 投票、夜晚及第二轮次及以后的所有阶段，全员统一禁用输入框，直到游戏重置回准备阶段。
const isInputDisabled = computed(() => {
  if (!isSelfAlive.value) return true;
  if (currentPhase.value === GamePhase.PREPARING) return false;
  if (currentPhase.value === GamePhase.DESCRIPTION && currentRound.value === 1) {
    return false;
  }
  return true;
});

// 输入框占位符动态展示
const inputPlaceholder = computed(() => {
  if (myIdentity.value?.role === ROLE_SPECTATOR) {
    return "您当前处于观战模式，禁止发言";
  }
  if (!isSelfAlive.value) {
    return "您已出局 (OUT)，禁止发言";
  }
  if (currentPhase.value === GamePhase.PREPARING) {
    return "房间准备中，可自由交流讨论...";
  }
  if (currentPhase.value === GamePhase.DESCRIPTION && currentRound.value === 1) {
    return "第 1 轮一句话描述（本局仅第一日发言）...";
  }
  return "每轮白天无需重复发言，当前阶段禁止输入";
});

// 确认知悉阶段转换后的回调处理
function handlePhaseNoticeConfirmed() {
  if (currentPhase.value === GamePhase.SETTLEMENT) {
    isSettlementModalOpen.value = true;
  }
}

// 存活玩家统计
const aliveCount = computed(() => players.value.filter((p) => p.is_alive).length);

// 滚动到底部辅助函数
function scrollToBottom() {
  nextTick(() => {
    if (chatScrollContainer.value) {
      chatScrollContainer.value.scrollTop = chatScrollContainer.value.scrollHeight;
    }
  });
}

// 清理所有 Tauri 事件监听器
async function cleanupListeners() {
  for (const unlisten of unlistenFns) {
    try {
      unlisten();
    } catch (e) {
      console.warn("清理监听器错误:", e);
    }
  }
  unlistenFns = [];
}

// 建立 Tauri 事件监听网络
async function setupGameListeners() {
  await cleanupListeners();

  // 1. 玩家加入事件
  unlistenFns.push(
    await listen<any>("join_req", (event) => {
      const { player_id: playerId, player_name: playerName, avatar } = event.payload;
      const exists = players.value.find((p) => p.id === playerId);
      if (!exists) {
        players.value.push({
          id: playerId,
          name: playerName,
          avatar: avatar || "/avatar/characters/detective.svg",
          is_alive: true,
        });
        chatMessages.value.push({
          sender_id: "SYSTEM",
          text: `特工【${playerName}】加入房间。`,
          is_system: true,
          timestamp: Date.now(),
        });
        scrollToBottom();
      }
    })
  );

  // 2. 玩家离开事件
  unlistenFns.push(
    await listen<any>("leave_req", (event) => {
      const { player_id: playerId } = event.payload;
      const leaving = players.value.find((p) => p.id === playerId);
      if (leaving) {
        players.value = players.value.filter((p) => p.id !== playerId);
        chatMessages.value.push({
          sender_id: "SYSTEM",
          text: `特工【${leaving.name}】离开房间。`,
          is_system: true,
          timestamp: Date.now(),
        });
        scrollToBottom();
      }
    })
  );

  // 3. 聊天消息事件
  unlistenFns.push(
    await listen<any>("chat", (event) => {
      const { sender_id: senderId, text, is_system: isSystem } = event.payload;

      chatMessages.value.push({
        sender_id: senderId,
        text,
        is_system: isSystem ?? (senderId === "SYSTEM"),
        timestamp: Date.now(),
      });

      scrollToBottom();
    })
  );

  // 4. 阶段流转同步事件
  unlistenFns.push(
    await listen<any>("sync_phase", async (event) => {
      const { new_phase: newPhase, extra_info: extraInfo } = event.payload;
      currentPhase.value = newPhase;
      phaseExtraInfo.value = extraInfo;

      chatMessages.value.push({
        sender_id: "SYSTEM",
        text: `【阶段更新】${extraInfo}`,
        is_system: true,
        timestamp: Date.now(),
      });

      // 刷新游戏引擎全局状态（同步最新轮次与存活人员）
      await refreshGameState();

      // 每次阶段转换，弹窗明确通知所有人阶段已转换
      isPhaseNoticeOpen.value = true;

      // 进入投票阶段时重置投票锁定状态
      if (newPhase === GamePhase.VOTING) {
        hasVoted.value = false;
        votedTargetName.value = "";
      } else {
        isVoteModalOpen.value = false;
      }

      // 若进入白板特别猜词阶段，白板特工自动弹出两方词猜词对话框
      if (newPhase === GamePhase.BLANK_GUESS) {
        if (myIdentity.value?.role === ROLE_BLANK) {
          isBlankGuessModalOpen.value = true;
        }
      }

      // 游戏进入终局结算时，清空所有本地身份推测标记
      if (newPhase === GamePhase.SETTLEMENT) {
        clearPlayerMarks();
      }

      // 从第二轮次白天开始，每次白天将所有人过往发言（包括玩家本人发言）重新移动到聊天栏底部方便所有人查看和回忆
      if (newPhase === GamePhase.DESCRIPTION && currentRound.value >= 2) {
        const playerMsgs = chatMessages.value.filter((m) => !m.is_system);
        if (playerMsgs.length > 0) {
          const systemMsgs = chatMessages.value.filter(
            (m) => m.is_system && !m.text.startsWith("【历史回忆】")
          );
          systemMsgs.push({
            sender_id: "SYSTEM",
            text: `【历史回忆】以下为第 1 轮全员线索发言回顾（每轮白天无需重复发言）：`,
            is_system: true,
            timestamp: Date.now(),
          });
          chatMessages.value = [...systemMsgs, ...playerMsgs];
        }
      }

      scrollToBottom();
    })
  );

  // 5. 身份派发事件
  unlistenFns.push(
    await listen<any>("identity", (event) => {
      const { role, word, hint } = event.payload;
      myIdentity.value = { role, word, hint };
      isIdentityModalOpen.value = true;
    })
  );

  // 6. 房间关闭事件
  unlistenFns.push(
    await listen("room_closed", () => {
      alert("房主已解散或断开房间");
      leaveRoom();
    })
  );
}

onMounted(async () => {
  await setupGameListeners();
  await refreshGameState();
  scrollToBottom();
});

onUnmounted(async () => {
  await cleanupListeners();
});

// 发送聊天输入
async function handleSend() {
  if (isInputDisabled.value) return;
  const msg = inputText.value.trim();
  if (!msg) return;
  inputText.value = "";
  try {
    await sendChat(msg);
  } catch (err: any) {
    console.error("发言发送失败:", err);
  }
}

// 处理投票成功事件（仅本人可见的系统播报）
function handleVoted(target: { id: string; name: string }) {
  hasVoted.value = true;
  votedTargetName.value = target.name;
  chatMessages.value.push({
    sender_id: "SYSTEM",
    text: `【公投选择】您已投票给【${target.name}】（已锁定不可更改，仅自己可见）`,
    is_system: true,
    timestamp: Date.now(),
  });
  scrollToBottom();
}

// 获取玩家存活状态辅助
function isPlayerAlive(senderId: string): boolean {
  const p = players.value.find((item) => item.id === senderId);
  return p ? p.is_alive : true;
}

// 获取玩家头像辅助
function getPlayerAvatar(senderId: string): string {
  const p = players.value.find((item) => item.id === senderId);
  return p ? p.avatar : "/avatar/characters/detective.svg";
}

// 获取玩家昵称辅助
function getPlayerName(senderId: string): string {
  if (senderId === selfPlayer.id) return "我";
  const p = players.value.find((item) => item.id === senderId);
  return p ? p.name : senderId;
}

// 点击玩家头像触发身份推测标记弹窗（点击自己头像触发查看身份）
function handleAvatarClick(playerId: string) {
  if (playerId === selfPlayer.id) {
    isIdentityModalOpen.value = true;
    return;
  }
  const p = players.value.find((item) => item.id === playerId);
  if (p) {
    markedTargetPlayer.value = p;
    isPlayerMarkModalOpen.value = true;
  }
}

// 处理身份标记选择
function handleMarkSelected(role: string) {
  if (markedTargetPlayer.value) {
    setPlayerMark(markedTargetPlayer.value.id, role);
  }
}

// 身份标记样式映射辅助
function getMarkBadgeClass(role: string): string {
  switch (role) {
    case ROLE_GOOD: return "badge-primary";
    case ROLE_SPY: return "badge-error";
    case ROLE_ANGEL: return "badge-info";
    case ROLE_BLANK: return "badge-secondary";
    default: return "badge-neutral";
  }
}

// 身份标记文案映射辅助
function getMarkLabel(role: string): string {
  switch (role) {
    case ROLE_GOOD: return "🛡️好人?";
    case ROLE_SPY: return "🕵️卧底?";
    case ROLE_ANGEL: return "👼天使?";
    case ROLE_BLANK: return "🎭白板?";
    default: return "";
  }
}
</script>

<template>
  <div class="h-screen bg-base-200 flex flex-col overflow-hidden relative">
    <!-- 1. 顶部 Header 导航栏 -->
    <header class="navbar bg-base-100 shadow-md border-b border-base-300 px-2 sm:px-4 flex justify-between items-center z-20 flex-none gap-1 min-h-[48px]">
      <!-- 左侧：返回大厅按钮 -->
      <div class="flex items-center gap-1 sm:gap-2 shrink-0">
        <button
          class="btn btn-xs sm:btn-sm btn-ghost gap-1 px-1 sm:px-2.5"
          title="返回大厅"
          @click="leaveRoom"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M15 19l-7-7 7-7" />
          </svg>
          <span class="text-xs">返回</span>
        </button>
        <span v-if="networkMode === 'p2p'" class="badge badge-xs sm:badge-sm badge-primary font-mono font-bold">
          房间: {{ roomCode }}
        </span>
        <span v-else class="badge badge-xs sm:badge-sm badge-neutral hidden md:inline-flex">
          端口: {{ roomPort }}
        </span>
      </div>

      <!-- 中间：当前阶段与轮次显示 -->
      <div class="flex flex-col items-center min-w-0 flex-1 px-1">
        <div class="flex items-center gap-1.5 flex-wrap justify-center">
          <span class="badge badge-xs sm:badge-sm" :class="phaseBadgeClassMap[currentPhase] || 'badge-neutral'">
            {{ phaseTitleMap[currentPhase] || '未知阶段' }}
          </span>
          <span class="text-xs font-semibold text-base-content/80">
            第 {{ currentRound }} 轮
          </span>
          <span class="text-xs text-base-content/60 hidden sm:inline">
            (存活 {{ aliveCount }}/{{ players.length }} 人)
          </span>
          <span v-if="myIdentity?.role === ROLE_SPECTATOR" class="badge badge-neutral badge-xs sm:badge-sm font-semibold gap-1">
            <span>👀</span>
            <span>观战中</span>
          </span>
          <span v-else-if="!isSelfAlive" class="badge badge-error badge-xs sm:badge-sm font-bold gap-1 animate-pulse">
            <span>💀</span>
            <span>OUT</span>
          </span>
        </div>
        <span class="text-[11px] text-base-content/60 truncate max-w-[180px] sm:max-w-xs md:max-w-md mt-0.5">
          {{ phaseExtraInfo }}
        </span>
      </div>

      <!-- 右侧：身份卡与大厅列表查看 -->
      <div class="flex items-center gap-1 sm:gap-2 shrink-0">
        <!-- 身份按钮 -->
        <button
          class="btn btn-xs sm:btn-sm btn-outline btn-primary gap-1 px-2 sm:px-3"
          @click="isIdentityModalOpen = true"
        >
          <span>📜</span>
          <span>我的身份</span>
        </button>

        <!-- 终局结算战报按钮 (仅结算期可见) -->
        <button
          v-if="currentPhase === GamePhase.SETTLEMENT"
          class="btn btn-xs sm:btn-sm btn-success gap-1 px-2 sm:px-3"
          @click="isSettlementModalOpen = true"
        >
          <span>🏆</span>
          <span>战报</span>
        </button>
      </div>
    </header>

    <!-- 2. 中间主区域：自适应高度独立滚动的 DaisyUI 聊天气泡流 -->
    <main
      ref="chatScrollContainer"
      class="flex-1 overflow-y-auto p-4 md:p-6 space-y-4"
    >
      <!-- 房间初始引导卡片 -->
      <div v-if="chatMessages.length === 0" class="text-center py-12 text-base-content/50 text-sm">
        房间通信已接通，等待玩家就绪...
      </div>

      <!-- 聊天消息气泡流 -->
      <div
        v-for="(msg, idx) in chatMessages"
        :key="idx"
        class="transition-all"
      >
        <!-- 系统公告消息：居中 warning 徽章/卡片 -->
        <div v-if="msg.is_system" class="flex justify-center my-2">
          <div class="alert alert-warning shadow-sm py-2 px-4 max-w-lg text-xs md:text-sm font-medium flex items-center justify-center gap-2">
            <span>📢</span>
            <span>{{ msg.text }}</span>
          </div>
        </div>

        <!-- 自己的消息：chat-end + chat-bubble-primary -->
        <div
          v-else-if="msg.sender_id === selfPlayer.id"
          class="chat chat-end"
        >
          <div
            class="chat-image avatar cursor-pointer hover:scale-105 transition-transform"
            title="点击查看我的身份"
            @click="isIdentityModalOpen = true"
          >
            <div
              class="w-10 h-10 rounded-full border shadow-sm bg-base-100 overflow-hidden"
              :class="isSelfAlive ? 'border-primary/40' : 'border-error filter grayscale contrast-75 opacity-60'"
            >
              <img :src="selfPlayer.avatar" alt="My avatar" class="w-full h-full object-cover" />
            </div>
          </div>
          <div class="chat-header text-xs opacity-60 mb-1 flex items-center gap-1">
            <span>{{ selfPlayer.name }}</span>
            <span class="badge badge-xs badge-primary">我</span>
            <span v-if="myIdentity?.role === ROLE_SPECTATOR" class="badge badge-xs badge-neutral">观战</span>
            <span v-else-if="!isSelfAlive" class="badge badge-xs badge-error font-bold">OUT</span>
          </div>
          <div class="chat-bubble chat-bubble-primary text-sm shadow">
            {{ msg.text }}
          </div>
        </div>

        <!-- 其他玩家消息：chat-start + chat-bubble-neutral -->
        <div
          v-else
          class="chat chat-start"
        >
          <div
            class="chat-image avatar cursor-pointer hover:ring-2 hover:ring-primary hover:scale-105 transition-all rounded-full"
            title="点击标记/查看此特工身份推测"
            @click="handleAvatarClick(msg.sender_id)"
          >
            <div
              class="w-10 h-10 rounded-full border shadow-sm bg-base-100 overflow-hidden"
              :class="isPlayerAlive(msg.sender_id) ? 'border-base-300' : 'border-error filter grayscale contrast-75 opacity-60'"
            >
              <img :src="getPlayerAvatar(msg.sender_id)" alt="Player avatar" class="w-full h-full object-cover" />
            </div>
          </div>
          <div class="chat-header text-xs opacity-60 mb-1 flex items-center gap-1">
            <span>{{ getPlayerName(msg.sender_id) }}</span>
            <!-- 本地私密推测便签徽章（仅自己可见，点击可快速修改） -->
            <span
              v-if="playerMarks[msg.sender_id]"
              class="badge badge-xs font-semibold cursor-pointer shadow-xs hover:scale-105 transition-transform"
              :class="getMarkBadgeClass(playerMarks[msg.sender_id])"
              title="点击修改推测标记"
              @click.stop="handleAvatarClick(msg.sender_id)"
            >
              {{ getMarkLabel(playerMarks[msg.sender_id]) }}
            </span>
            <span v-if="!isPlayerAlive(msg.sender_id)" class="badge badge-xs badge-error font-bold">OUT</span>
          </div>
          <div class="chat-bubble bg-base-100 text-base-content text-sm shadow border border-base-300">
            {{ msg.text }}
          </div>
        </div>
      </div>
    </main>

    <!-- 3. 操作按钮悬浮栏（阶段专属动作） -->
    <div
      v-if="currentPhase === GamePhase.VOTING && isSelfAlive"
      class="flex-none bg-warning/10 border-t border-warning/30 px-4 py-2 flex items-center justify-between z-10"
    >
      <div class="text-xs font-semibold text-warning-content flex items-center gap-1.5">
        <span>🗳️</span>
        <span v-if="!hasVoted">投票阶段进行中，请选择怀疑对象</span>
        <span v-else class="text-warning-content/90">
          已完成投票：投给了【<span class="font-bold underline">{{ votedTargetName }}</span>】（不可更改）
        </span>
      </div>
      <button
        v-if="!hasVoted"
        class="btn btn-xs btn-warning shadow-sm"
        @click="isVoteModalOpen = true"
      >
        立即投票
      </button>
      <span v-else class="badge badge-warning badge-sm font-semibold shadow-sm">
        ✓ 已提交
      </span>
    </div>

    <div
      v-else-if="currentPhase === GamePhase.NIGHT_ACTION && isSelfAlive"
      class="flex-none bg-error/10 border-t border-error/30 px-4 py-2 flex items-center justify-between z-10"
    >
      <span class="text-xs font-semibold text-error-content flex items-center gap-1">
        <span>🌙</span> 夜间阶段进行中，卧底可执行暗杀
      </span>
      <button class="btn btn-xs btn-error" @click="isNightModalOpen = true">
        申请暗杀
      </button>
    </div>

    <div
      v-else-if="currentPhase === GamePhase.BLANK_GUESS"
      class="flex-none bg-warning/10 border-t border-warning/30 px-4 py-2 flex items-center justify-between z-10"
    >
      <span class="text-xs font-semibold text-warning-content flex items-center gap-1">
        <span>✨</span> 白板猜词特别阶段：白板特工正在进行两方词猜词
      </span>
      <button
        v-if="myIdentity?.role === ROLE_BLANK"
        class="btn btn-xs btn-warning font-bold shadow-sm"
        @click="isBlankGuessModalOpen = true"
      >
        进行猜词
      </button>
    </div>

    <!-- 4. 底部输入栏：始终固定在窗口最下方 (flex-none)，按规则控制禁用 -->
    <footer class="flex-none bg-base-100 border-t border-base-300 p-3 z-10">
      <form class="join w-full" @submit.prevent="handleSend">
        <input
          v-model="inputText"
          type="text"
          :placeholder="inputPlaceholder"
          class="input input-bordered join-item w-full text-sm"
          maxlength="100"
          :disabled="isInputDisabled"
        />
        <button
          type="submit"
          class="btn btn-primary join-item px-6"
          :disabled="isInputDisabled || !inputText.trim()"
        >
          发送
        </button>
      </form>
    </footer>

    <!-- 5. 房主专属悬浮管理按钮 (FAB) - 上移至 bottom-36 像素位，彻底避开投票操作条与底部头像 -->
    <button
      v-if="isHost"
      class="btn btn-circle btn-primary btn-sm sm:btn-md fixed bottom-36 right-4 z-30 shadow-2xl hover:scale-105 transition-transform border border-primary-content/20 opacity-90 hover:opacity-100"
      title="房主管理控制中心"
      @click="isHostModalOpen = true"
    >
      <span class="text-base sm:text-lg">⚙️</span>
    </button>

    <!-- ========================================== -->
    <!-- 拆分重构后的各阶段独立模态弹窗组件           -->
    <!-- ========================================== -->
    <!-- A. 查看身份弹窗 -->
    <IdentityModal v-model:is-open="isIdentityModalOpen" />

    <!-- B. 房主控制台弹窗 -->
    <HostConsoleModal v-model:is-open="isHostModalOpen" />

    <!-- C. 投票选择弹窗 -->
    <VoteModal
      v-model:is-open="isVoteModalOpen"
      :has-voted="hasVoted"
      :voted-target-name="votedTargetName"
      @voted="handleVoted"
    />

    <!-- D. 黑夜暗杀弹窗 -->
    <NightActionModal v-model:is-open="isNightModalOpen" />

    <!-- E. 白板猜词弹窗 -->
    <BlankGuessModal v-model:is-open="isBlankGuessModalOpen" />

    <!-- F. 终局战报大揭秘弹窗 -->
    <SettlementModal v-model:is-open="isSettlementModalOpen" />

    <!-- G. 阶段流转明确确认通知弹窗 -->
    <PhaseNoticeModal
      v-model:is-open="isPhaseNoticeOpen"
      :phase="currentPhase"
      :round="currentRound"
      :extra-info="phaseExtraInfo"
      @confirm="handlePhaseNoticeConfirmed"
    />

    <!-- H. 玩家本地身份推测便签弹窗 -->
    <PlayerMarkModal
      v-model:is-open="isPlayerMarkModalOpen"
      :target-player="markedTargetPlayer"
      :current-mark="markedTargetPlayer ? (playerMarks[markedTargetPlayer.id] || '') : ''"
      @mark="handleMarkSelected"
    />
  </div>
</template>
