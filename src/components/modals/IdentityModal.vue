<script setup lang="ts">
import { computed } from "vue";
import { useGame } from "../../composables/useGame";
import {
  ROLE_ANGEL,
  ROLE_BLANK,
  ROLE_SPECTATOR,
} from "../../types/game";

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
}>();

const { myIdentity } = useGame();

function close() {
  emit("update:isOpen", false);
}

// 天使双词解析计算属性
const angelWords = computed(() => {
  if (!myIdentity.value?.word) return ["-", "-"];
  const parts = myIdentity.value.word.split(" / ");
  return parts.length === 2 ? parts : [myIdentity.value.word, ""];
});
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-sm text-center">
      <h3 class="font-bold text-lg mb-4 text-primary flex items-center justify-center gap-2">
        <span>📜</span> 我的身份信息
      </h3>

      <div v-if="myIdentity" class="space-y-4 my-2">
        <!-- 身份角标（除天使与白板外，普通特工发词阶段不显示 GOOD/SPY，需自辨多数派/少数派） -->
        <div>
          <span class="text-xs text-base-content/60 block mb-1">分配角色</span>
          <span
            class="badge badge-lg font-bold px-4 py-3"
            :class="{
              'badge-info': myIdentity.role === ROLE_ANGEL,
              'badge-secondary': myIdentity.role === ROLE_BLANK,
              'badge-neutral': myIdentity.role === ROLE_SPECTATOR,
              'badge-primary': myIdentity.role !== ROLE_ANGEL && myIdentity.role !== ROLE_BLANK && myIdentity.role !== ROLE_SPECTATOR,
            }"
          >
            {{
              myIdentity.role === ROLE_ANGEL ? '天使阵营 (Angel)' :
              myIdentity.role === ROLE_BLANK ? '白板阵营 (Blank)' :
              myIdentity.role === ROLE_SPECTATOR ? '观战者 (Spectator)' : '特工 (Agent)'
            }}
          </span>
        </div>

        <!-- 0. 观战者卡片 -->
        <div v-if="myIdentity.role === ROLE_SPECTATOR" class="bg-base-200 p-4 rounded-xl border border-base-300 space-y-2">
          <span class="text-xs text-base-content/60 block mb-1">👀 观战模式</span>
          <p class="text-xs text-base-content/80 text-center leading-relaxed">
            您在中途加入或已处于观战席。本局不可发言与操作，但可实时观看讨论与终局结算。
          </p>
        </div>

        <!-- 1. 天使双词卡片（同时获知好词和坏词，未知好坏） -->
        <div v-if="myIdentity.role === ROLE_ANGEL" class="bg-base-200 p-4 rounded-xl border border-base-300 space-y-2">
          <span class="text-xs text-base-content/60 block mb-2">天使双词（好词与坏词，顺序随机未知好坏）</span>
          <div class="flex items-center justify-center gap-3">
            <span class="badge badge-lg badge-info font-black text-base px-3 py-4 shadow-sm">
              {{ angelWords[0] }}
            </span>
            <span class="text-xs font-bold text-base-content/40">与</span>
            <span class="badge badge-lg badge-info font-black text-base px-3 py-4 shadow-sm">
              {{ angelWords[1] }}
            </span>
          </div>
          <p class="text-[11px] text-base-content/60 text-center mt-2 leading-relaxed">
            您已同时获知好词与坏词，但未知词的好坏，请结合全员发言逻辑进行推导判断。
          </p>
        </div>

        <!-- 2. 普通特工单词卡片（多数派好人 / 少数派卧底待自辨） -->
        <div v-else-if="myIdentity.role !== ROLE_BLANK && myIdentity.word" class="bg-base-200 p-4 rounded-xl border border-base-300">
          <span class="text-xs text-base-content/60 block mb-1">本局词语</span>
          <span class="text-2xl font-black text-primary tracking-widest block my-1">
            {{ myIdentity.word }}
          </span>
          <p class="text-[11px] text-base-content/60 text-center mt-2 leading-relaxed">
            阵营待自辨：您已获得本局词语，但未知属于多数派（好词）还是少数派（坏词）。请在游玩与发言过程中自行判断阵营归属！
          </p>
        </div>

        <!-- 3. 白板无词提示 -->
        <div v-else-if="myIdentity.role === ROLE_BLANK" class="alert alert-info text-xs text-left">
          <span>⚠️ 您的角色为白板，没有获得任何词语，请根据下方提示发言伪装！</span>
        </div>

        <!-- 4. 提示词（白板可见） -->
        <div v-if="myIdentity.role === ROLE_BLANK && myIdentity.hint" class="bg-warning/10 p-3 rounded-xl border border-warning/30 text-left">
          <span class="text-xs text-warning font-semibold block mb-1">💡 相似方向提示 (Hint)</span>
          <p class="text-xs leading-relaxed text-base-content/80">
            {{ myIdentity.hint }}
          </p>
        </div>
      </div>

      <div v-else class="py-6 text-sm text-base-content/60">
        游戏尚未开始，请等待房主配置出题并启动。
      </div>

      <div class="modal-action justify-center">
        <button class="btn btn-sm btn-neutral w-full" @click="close">
          关闭
        </button>
      </div>
    </div>
  </dialog>
</template>

