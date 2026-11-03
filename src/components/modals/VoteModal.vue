<script setup lang="ts">
import { ref, computed } from "vue";
import { useGame } from "../../composables/useGame";

const props = defineProps<{
  isOpen: boolean;
  hasVoted: boolean;
  votedTargetName: string;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
  (e: "voted", target: { id: string; name: string }): void;
}>();

const { selfPlayer, players, submitVote } = useGame();

const selectedVoteTarget = ref("");
const actionLoading = ref(false);

const aliveOtherPlayers = computed(() => {
  return players.value.filter((p) => p.is_alive && p.id !== selfPlayer.id);
});

function close() {
  emit("update:isOpen", false);
}

async function handleConfirmVote() {
  if (!selectedVoteTarget.value || props.hasVoted) return;
  actionLoading.value = true;
  try {
    const targetPlayer = players.value.find((p) => p.id === selectedVoteTarget.value);
    const targetName = targetPlayer ? targetPlayer.name : "未知特工";
    await submitVote(selectedVoteTarget.value);
    emit("voted", { id: selectedVoteTarget.value, name: targetName });
    close();
  } catch (err: any) {
    alert(err?.message || err?.toString() || "投票失败");
  } finally {
    actionLoading.value = false;
  }
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-sm">
      <h3 class="font-bold text-lg mb-2 text-warning flex items-center gap-2">
        <span>🗳️</span> 公投表决
      </h3>

      <!-- 已完成投票状态（锁定不可更改） -->
      <div v-if="hasVoted" class="py-4 space-y-3">
        <div class="alert alert-warning text-xs text-left leading-relaxed">
          <span>🔒 您已投票给【<strong>{{ votedTargetName }}</strong>】。<br />投票仅可选择一次，提交后不可更改。</span>
        </div>
        <div class="modal-action justify-center">
          <button class="btn btn-sm btn-neutral w-full" @click="close">
            关闭
          </button>
        </div>
      </div>

      <!-- 尚未投票状态 -->
      <div v-else>
        <p class="text-xs text-base-content/60 mb-4">
          请选择投票目标（仅可选择一次，提交后不可更改）：
        </p>

        <div class="space-y-2 max-h-60 overflow-y-auto">
          <label
            v-for="p in aliveOtherPlayers"
            :key="p.id"
            class="flex items-center gap-3 p-3 bg-base-200 rounded-xl cursor-pointer hover:bg-base-300 transition-colors"
          >
            <input
              v-model="selectedVoteTarget"
              type="radio"
              name="vote-target"
              :value="p.id"
              class="radio radio-warning radio-sm"
            />
            <div class="avatar">
              <div class="w-8 h-8 rounded-full">
                <img :src="p.avatar" :alt="p.name" />
              </div>
            </div>
            <span class="text-sm font-semibold flex-1">{{ p.name }}</span>
          </label>
        </div>

        <div class="modal-action">
          <button class="btn btn-sm btn-ghost" @click="close">取消</button>
          <button
            class="btn btn-sm btn-warning"
            :disabled="!selectedVoteTarget || actionLoading"
            @click="handleConfirmVote"
          >
            <span v-if="actionLoading" class="loading loading-spinner loading-xs"></span>
            确认投票
          </button>
        </div>
      </div>
    </div>
  </dialog>
</template>

