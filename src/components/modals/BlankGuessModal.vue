<script setup lang="ts">
import { ref } from "vue";
import { useGame } from "../../composables/useGame";

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
}>();

const { submitBlankGuess } = useGame();

const guessWordA = ref("");
const guessWordB = ref("");
const actionLoading = ref(false);

function close() {
  emit("update:isOpen", false);
}

// 提交两方词猜词
async function handleConfirmBlankGuess() {
  const w1 = guessWordA.value.trim();
  const w2 = guessWordB.value.trim();
  if (!w1 || !w2) return;

  actionLoading.value = true;
  try {
    await submitBlankGuess(w1, w2);
    guessWordA.value = "";
    guessWordB.value = "";
    close();
  } catch (err: any) {
    alert(err?.message || err?.toString() || "提交猜词失败");
  } finally {
    actionLoading.value = false;
  }
}

// 放弃猜词
async function handleGiveUp() {
  if (actionLoading.value) return;
  actionLoading.value = true;
  try {
    await submitBlankGuess("", "");
    guessWordA.value = "";
    guessWordB.value = "";
    close();
  } catch (err: any) {
    console.warn("放弃猜词错误:", err);
    close();
  } finally {
    actionLoading.value = false;
  }
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-sm border border-warning/40 shadow-2xl">
      <h3 class="font-bold text-lg mb-2 text-warning flex items-center gap-2">
        <span>✨</span> 白板猜词特别阶段
      </h3>
      <p class="text-xs text-base-content/70 mb-4 leading-relaxed">
        您已被公投出局！请猜出本局的两方词汇（好人词与卧底词）。若猜得的两方词完全正确，游戏立即结束并由白板获胜！
      </p>

      <div class="space-y-3 mb-4">
        <div class="form-control">
          <label class="label py-1">
            <span class="label-text text-xs font-semibold">词语一：</span>
          </label>
          <input
            v-model="guessWordA"
            type="text"
            placeholder="输入您猜测的第一个词语..."
            class="input input-bordered input-sm w-full text-sm"
            maxlength="20"
          />
        </div>
        <div class="form-control">
          <label class="label py-1">
            <span class="label-text text-xs font-semibold">词语二：</span>
          </label>
          <input
            v-model="guessWordB"
            type="text"
            placeholder="输入您猜测的第二个词语..."
            class="input input-bordered input-sm w-full text-sm"
            maxlength="20"
          />
        </div>
      </div>

      <div class="modal-action">
        <button
          class="btn btn-sm btn-ghost text-xs"
          :disabled="actionLoading"
          @click="handleGiveUp"
        >
          放弃猜词
        </button>
        <button
          class="btn btn-sm btn-warning font-bold text-xs"
          :disabled="!guessWordA.trim() || !guessWordB.trim() || actionLoading"
          @click="handleConfirmBlankGuess"
        >
          <span v-if="actionLoading" class="loading loading-spinner loading-xs"></span>
          提交双词猜词
        </button>
      </div>
    </div>
  </dialog>
</template>
