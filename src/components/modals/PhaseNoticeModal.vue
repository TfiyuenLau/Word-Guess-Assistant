<script setup lang="ts">
import { computed } from "vue";
import { GamePhase } from "../../types/game";

const props = defineProps<{
  isOpen: boolean;
  phase: GamePhase;
  round: number;
  extraInfo: string;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
  (e: "confirm"): void;
}>();

function close() {
  emit("update:isOpen", false);
  emit("confirm");
}

const phaseConfig = computed(() => {
  switch (props.phase) {
    case GamePhase.DESCRIPTION:
      return {
        icon: "☀️",
        title: props.round === 1 ? "一句话描述阶段" : `第 ${props.round} 轮白天`,
        badgeClass: "badge-info",
        description:
          props.round === 1
            ? "第 1 轮白天开始，请所有存活玩家用一句话描述自己的词语（本局仅第一日发言）。"
            : `第 ${props.round} 轮白天开始。每轮白天无需重复发言，过往发言已整理在聊天区底部供回忆与推导。`,
      };
    case GamePhase.VOTING:
      return {
        icon: "🗳️",
        title: "讨论与公投阶段",
        badgeClass: "badge-warning",
        description: "公投阶段已开启，请各位特工仔细比对线索，进行投票表决（单选且提交后锁定）。",
      };
    case GamePhase.NIGHT_ACTION:
      return {
        icon: "🌙",
        title: "黑夜行动阶段",
        badgeClass: "badge-error",
        description: "黑夜阶段降临，卧底特工可执行暗杀申请；非卧底擅自行动将导致自身出局。",
      };
    case GamePhase.BLANK_GUESS:
      return {
        icon: "✨",
        title: "白板猜词特别阶段",
        badgeClass: "badge-warning",
        description: "白板特工被公投出局，触发特别猜词阶段！若猜得的两方词完全正确，游戏立即结束并由白板获胜；若猜错则继续进入黑夜。",
      };
    case GamePhase.SETTLEMENT:
      return {
        icon: "🏆",
        title: "终局结算阶段",
        badgeClass: "badge-success",
        description: "本局游戏已分出胜负，所有玩家真实身份与机密词汇即将揭晓！",
      };
    default:
      return {
        icon: "🔄",
        title: "阶段已更新",
        badgeClass: "badge-neutral",
        description: props.extraInfo || "游戏状态已流转，请知悉。",
      };
  }
});
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-sm text-center border border-base-300 shadow-2xl">
      <div class="text-4xl mb-3 animate-bounce">
        {{ phaseConfig.icon }}
      </div>
      <h3 class="font-bold text-lg mb-1 flex items-center justify-center gap-2">
        <span class="badge badge-sm font-semibold" :class="phaseConfig.badgeClass">
          第 {{ round }} 轮
        </span>
        <span>{{ phaseConfig.title }}</span>
      </h3>
      <p class="text-xs text-base-content/70 mt-3 mb-6 leading-relaxed bg-base-200 p-3 rounded-xl border border-base-300 text-left">
        {{ phaseConfig.description }}
      </p>
      <div class="modal-action justify-center">
        <button class="btn btn-primary btn-sm w-full font-bold shadow-md" @click="close">
          确认知悉
        </button>
      </div>
    </div>
  </dialog>
</template>

