<script setup lang="ts">
import { ref, computed } from "vue";
import { useGame } from "../../composables/useGame";
import { GamePhase } from "../../types/game";

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
}>();

const {
  currentPhase,
  currentRound,
  players,
  startGame,
  nextPhase,
  restartGame,
} = useGame();

const similaritySetting = ref("极为接近");
const themeSetting = ref("");
const actionLoading = ref(false);

const aliveCount = computed(() => players.value.filter((p) => p.is_alive).length);

const phaseTitleMap: Record<number, string> = {
  [GamePhase.PREPARING]: "准备中",
  [GamePhase.DESCRIPTION]: "一句话描述",
  [GamePhase.VOTING]: "讨论与投票",
  [GamePhase.NIGHT_ACTION]: "黑夜暗杀",
  [GamePhase.SETTLEMENT]: "终局结算",
  [GamePhase.BLANK_GUESS]: "白板猜词",
};

function close() {
  emit("update:isOpen", false);
}

// 房主启动游戏
async function handleHostStartGame() {
  actionLoading.value = true;
  try {
    await startGame(themeSetting.value.trim(), similaritySetting.value);
    close();
  } catch (err: any) {
    alert(err?.message || err?.toString() || "启动游戏失败");
  } finally {
    actionLoading.value = false;
  }
}

// 房主推进阶段
async function handleHostNextPhase() {
  actionLoading.value = true;
  try {
    await nextPhase();
    close();
  } catch (err: any) {
    alert(err?.message || err?.toString() || "推进阶段失败");
  } finally {
    actionLoading.value = false;
  }
}

// 房主重置游戏
async function handleHostRestart() {
  actionLoading.value = true;
  try {
    await restartGame();
    close();
  } catch (err: any) {
    alert(err?.message || err?.toString() || "重置游戏失败");
  } finally {
    actionLoading.value = false;
  }
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-bold text-lg mb-4 text-primary flex items-center justify-between">
        <span class="flex items-center gap-2"><span>👑</span> 房主控制中心</span>
        <span class="badge badge-sm badge-neutral">{{ phaseTitleMap[currentPhase] }}</span>
      </h3>

      <!-- (1) 准备阶段配置 -->
      <div v-if="currentPhase === GamePhase.PREPARING" class="space-y-4">
        <div class="form-control">
          <label class="label">
            <span class="label-text font-semibold">词汇相似度 (LLM / 词库)</span>
          </label>
          <div class="join w-full">
            <button
              type="button"
              class="btn join-item flex-1"
              :class="similaritySetting === '极为接近' ? 'btn-primary' : 'btn-outline'"
              @click="similaritySetting = '极为接近'"
            >
              极为接近
            </button>
            <button
              type="button"
              class="btn join-item flex-1"
              :class="similaritySetting === '较为接近' ? 'btn-primary' : 'btn-outline'"
              @click="similaritySetting = '较为接近'"
            >
              较为接近
            </button>
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text font-semibold">出题主题 / 领域 (选填)</span>
          </label>
          <input
            v-model="themeSetting"
            type="text"
            placeholder="例如：科技、美食、生活、自然..."
            class="input input-bordered w-full text-sm"
          />
        </div>

        <div class="bg-base-200 p-3 rounded-lg text-xs space-y-1">
          <div class="flex justify-between items-center">
            <span>当前房间玩家：</span>
            <span class="font-bold text-primary text-sm">{{ players.length }} 人</span>
          </div>
          <p class="text-base-content/60 text-[11px]">
            支持 4~12 名玩家同时在线，系统将智能平衡好人、卧底、天使与白板比例。
          </p>
        </div>

        <button
          class="btn btn-primary w-full shadow-lg"
          :disabled="actionLoading || players.length < 4 || players.length > 12"
          @click="handleHostStartGame"
        >
          <span v-if="actionLoading" class="loading loading-spinner loading-sm"></span>
          <span v-if="players.length < 4">等待玩家加入 (需 4~12 人，当前 {{ players.length }} 人)</span>
          <span v-else-if="players.length > 12">人数超出上限 (最多 12 人)</span>
          <span v-else>分发身份并启动游戏 ({{ players.length }} 人局)</span>
        </button>
      </div>

      <!-- (2) 游戏中各阶段流转控制 -->
      <div v-else class="space-y-4">
        <div class="stats bg-base-200 w-full shadow text-center py-2">
          <div class="stat p-2">
            <div class="stat-title text-xs">当前轮次</div>
            <div class="stat-value text-xl text-primary">{{ currentRound }}</div>
          </div>
          <div class="stat p-2">
            <div class="stat-title text-xs">存活人数</div>
            <div class="stat-value text-xl text-secondary">{{ aliveCount }}/{{ players.length }}</div>
          </div>
        </div>

        <p class="text-xs text-base-content/70">
          点击「推进阶段 (NEXT)」将根据游戏规则结算当前阶段并自动流转至下一状态。
        </p>

        <div class="flex flex-col gap-2 pt-2">
          <button
            class="btn btn-primary w-full"
            :disabled="actionLoading"
            @click="handleHostNextPhase"
          >
            <span v-if="actionLoading" class="loading loading-spinner loading-sm"></span>
            <span>推进阶段 (NEXT ⏭️)</span>
          </button>
          <button
            class="btn btn-outline btn-error w-full btn-sm"
            :disabled="actionLoading"
            @click="handleHostRestart"
          >
            重新开始游戏
          </button>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-sm btn-ghost" @click="close">
          关闭
        </button>
      </div>
    </div>
  </dialog>
</template>

