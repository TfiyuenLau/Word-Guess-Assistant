<script setup lang="ts">
import { ref, computed } from "vue";
import { useGame } from "../../composables/useGame";

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
  (e: "feedback", text: string): void;
}>();

const { selfPlayer, players, submitNightAction } = useGame();

const selectedNightTarget = ref("");
const actionLoading = ref(false);

const aliveOtherPlayers = computed(() => {
  return players.value.filter((p) => p.is_alive && p.id !== selfPlayer.id);
});

function close() {
  emit("update:isOpen", false);
}

async function handleConfirmNightAction() {
  if (!selectedNightTarget.value) return;
  actionLoading.value = true;
  try {
    const res = await submitNightAction(selectedNightTarget.value);
    emit("feedback", res);
    selectedNightTarget.value = "";
    close();
  } catch (err: any) {
    alert(err?.message || err?.toString() || "行动失败");
  } finally {
    actionLoading.value = false;
  }
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-sm border border-error/30">
      <h3 class="font-bold text-lg mb-2 text-error flex items-center gap-2">
        <span>🗡️</span> 夜间暗杀
      </h3>
      <div class="alert alert-error text-xs mb-4">
        <span>规则提示：若行动者非卧底身份，行动将导致自身出局。</span>
      </div>

      <div class="space-y-2 max-h-60 overflow-y-auto">
        <label
          v-for="p in aliveOtherPlayers"
          :key="p.id"
          class="flex items-center gap-3 p-3 bg-base-200 rounded-xl cursor-pointer hover:bg-base-300 transition-colors"
        >
          <input
            v-model="selectedNightTarget"
            type="radio"
            name="night-target"
            :value="p.id"
            class="radio radio-error radio-sm"
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
          class="btn btn-sm btn-error"
          :disabled="!selectedNightTarget || actionLoading"
          @click="handleConfirmNightAction"
        >
          <span v-if="actionLoading" class="loading loading-spinner loading-xs"></span>
          确认行动
        </button>
      </div>
    </div>
  </dialog>
</template>

