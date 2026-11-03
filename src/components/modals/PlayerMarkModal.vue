<script setup lang="ts">
import {
  ROLE_GOOD,
  ROLE_SPY,
  ROLE_ANGEL,
  ROLE_BLANK,
  Player,
} from "../../types/game";

const props = defineProps<{
  isOpen: boolean;
  targetPlayer: Player | null;
  currentMark: string;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
  (e: "mark", role: string): void;
}>();

function close() {
  emit("update:isOpen", false);
}

function selectMark(role: string) {
  emit("mark", role);
  close();
}

function clearMark() {
  emit("mark", "");
  close();
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div v-if="targetPlayer" class="modal-box max-w-sm text-center border border-base-300 shadow-2xl">
      <div class="flex items-center justify-between pb-3 border-b border-base-200 mb-4">
        <h3 class="font-bold text-sm flex items-center gap-1.5 text-primary">
          <span>📝</span> 特工身份推测标记
        </h3>
        <button class="btn btn-xs btn-circle btn-ghost" @click="close">✕</button>
      </div>

      <!-- 目标特工信息卡 -->
      <div class="flex items-center justify-center gap-3 p-3 bg-base-200 rounded-xl mb-4">
        <div class="avatar">
          <div
            class="w-12 h-12 rounded-full border shadow-sm bg-base-100 overflow-hidden"
            :class="targetPlayer.is_alive ? 'border-primary/50' : 'border-error filter grayscale opacity-60'"
          >
            <img :src="targetPlayer.avatar" :alt="targetPlayer.name" class="w-full h-full object-cover" />
          </div>
        </div>
        <div class="text-left">
          <div class="font-bold text-sm flex items-center gap-1.5">
            <span>{{ targetPlayer.name }}</span>
            <span v-if="!targetPlayer.is_alive" class="badge badge-xs badge-error font-bold">OUT</span>
          </div>
          <div class="text-xs text-base-content/60 mt-0.5">
            当前标记：
            <span v-if="currentMark === ROLE_GOOD" class="badge badge-xs badge-primary font-bold">🛡️ 好人</span>
            <span v-else-if="currentMark === ROLE_SPY" class="badge badge-xs badge-error font-bold">🕵️ 卧底</span>
            <span v-else-if="currentMark === ROLE_ANGEL" class="badge badge-xs badge-info font-bold">👼 天使</span>
            <span v-else-if="currentMark === ROLE_BLANK" class="badge badge-xs badge-secondary font-bold">🎭 白板</span>
            <span v-else class="text-base-content/40 italic">未标记</span>
          </div>
        </div>
      </div>

      <!-- 四大身份选择按钮网格 -->
      <p class="text-xs text-base-content/70 mb-3 text-left">
        选择您对该特工的怀疑/判断：
      </p>

      <div class="grid grid-cols-2 gap-2 mb-4">
        <button
          class="btn btn-sm"
          :class="currentMark === ROLE_GOOD ? 'btn-primary' : 'btn-outline btn-primary'"
          @click="selectMark(ROLE_GOOD)"
        >
          <span>🛡️</span>
          <span>好人 (Good)</span>
        </button>

        <button
          class="btn btn-sm"
          :class="currentMark === ROLE_SPY ? 'btn-error' : 'btn-outline btn-error'"
          @click="selectMark(ROLE_SPY)"
        >
          <span>🕵️</span>
          <span>卧底 (Spy)</span>
        </button>

        <button
          class="btn btn-sm"
          :class="currentMark === ROLE_ANGEL ? 'btn-info' : 'btn-outline btn-info'"
          @click="selectMark(ROLE_ANGEL)"
        >
          <span>👼</span>
          <span>天使 (Angel)</span>
        </button>

        <button
          class="btn btn-sm"
          :class="currentMark === ROLE_BLANK ? 'btn-secondary' : 'btn-outline btn-secondary'"
          @click="selectMark(ROLE_BLANK)"
        >
          <span>🎭</span>
          <span>白板 (Blank)</span>
        </button>
      </div>

      <!-- 底部清空与说明 -->
      <div class="space-y-3">
        <button
          v-if="currentMark"
          class="btn btn-xs btn-ghost text-error w-full"
          @click="clearMark"
        >
          ✕ 清除对此特工的标记
        </button>

        <p class="text-[11px] text-base-content/50 leading-relaxed bg-base-200/60 p-2.5 rounded-lg text-left">
          💡 <strong>本地私密便签</strong>：此标记仅保存在您的个人客户端，其他特工完全不可见，可随时点击头像修改，本局游戏结束后自动清空。
        </p>
      </div>
    </div>
  </dialog>
</template>

