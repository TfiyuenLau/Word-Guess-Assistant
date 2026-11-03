<script setup lang="ts">
import { useGame } from "../../composables/useGame";
import {
  ROLE_GOOD,
  ROLE_SPY,
  ROLE_ANGEL,
  ROLE_BLANK,
} from "../../types/game";

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
}>();

const { winner, settlementDetails } = useGame();

function close() {
  emit("update:isOpen", false);
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-lg">
      <h3 class="font-bold text-xl text-center mb-1 text-primary">
        游戏对局战报
      </h3>
      <p class="text-center text-xs text-base-content/60 mb-4">
        获胜方：<span class="badge badge-primary font-bold">{{ winner || '好人阵营' }}</span>
      </p>

      <!-- 词汇与提示公开卡片 -->
      <div v-if="settlementDetails" class="bg-base-200 p-4 rounded-xl mb-4 space-y-2 border border-base-300">
        <div class="flex justify-between items-center text-sm">
          <span class="text-base-content/70">好人词：</span>
          <span class="font-black text-primary text-base">{{ settlementDetails.good_word }}</span>
        </div>
        <div class="flex justify-between items-center text-sm">
          <span class="text-base-content/70">卧底词：</span>
          <span class="font-black text-error text-base">{{ settlementDetails.spy_word }}</span>
        </div>
        <div class="text-xs pt-1 border-t border-base-300 text-base-content/80">
          <span class="font-semibold text-warning">语义方向提示：</span>
          {{ settlementDetails.hint }}
        </div>
      </div>

      <!-- 全员身份揭秘列表 -->
      <h4 class="text-xs font-semibold text-base-content/60 mb-2">全员身份公示</h4>
      <div class="space-y-2 max-h-60 overflow-y-auto pr-1">
        <div
          v-for="p in settlementDetails?.player_reveals || []"
          :key="p.id"
          class="flex items-center justify-between p-2.5 bg-base-100 rounded-lg border border-base-200"
        >
          <div class="flex items-center gap-2">
            <div class="avatar">
              <div class="w-8 h-8 rounded-full">
                <img :src="p.avatar" :alt="p.name" />
              </div>
            </div>
            <div>
              <span class="text-sm font-bold block">{{ p.name }}</span>
              <span class="text-xs text-base-content/50">词语: {{ p.word || '（无词）' }}</span>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <span
              class="badge badge-sm"
              :class="{
                'badge-primary': p.role === ROLE_GOOD,
                'badge-error': p.role === ROLE_SPY,
                'badge-info': p.role === ROLE_ANGEL,
                'badge-secondary': p.role === ROLE_BLANK,
              }"
            >
              {{ p.role }}
            </span>
            <span
              class="badge badge-xs font-bold"
              :class="p.is_alive ? 'badge-success' : 'badge-error'"
            >
              {{ p.is_alive ? '存活' : 'OUT' }}
            </span>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-sm btn-primary w-full" @click="close">
          好的，已知悉
        </button>
      </div>
    </div>
  </dialog>
</template>

