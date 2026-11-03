<script setup lang="ts">
defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "update:isOpen", value: boolean): void;
}>();

function close() {
  emit("update:isOpen", false);
}
</script>

<template>
  <dialog class="modal modal-bottom sm:modal-middle" :class="{ 'modal-open': isOpen }">
    <div class="modal-box max-w-md p-4 bg-base-100 border border-base-300 shadow-2xl space-y-3">
      <!-- 弹窗顶栏 -->
      <div class="flex items-center justify-between border-b border-base-200 pb-2">
        <h3 class="font-bold text-sm sm:text-base flex items-center gap-1.5 text-primary">
          <span>📖</span>
          <span>游戏规则与身份指引</span>
        </h3>
        <button class="btn btn-xs btn-circle btn-ghost" @click="close">✕</button>
      </div>

      <!-- 滚动内容区域 -->
      <div class="max-h-[70vh] overflow-y-auto space-y-3 pr-1 text-xs">
        <!-- 1. 终局胜负判定 -->
        <section class="bg-base-200/80 rounded-xl p-3 border border-base-300">
          <h4 class="font-bold text-xs text-primary mb-2 flex items-center gap-1">
            <span>🏆</span> 终局胜负条件
          </h4>
          <ul class="space-y-1.5 leading-relaxed text-base-content/80">
            <li class="flex items-start gap-1.5">
              <span class="badge badge-primary badge-xs mt-0.5 shrink-0">好人胜</span>
              <span>所有卧底出局，<strong>且白板被找出并猜词失败</strong>（若卧底全灭但白板仍存活潜伏，好人无法直接获胜，需继续公投寻找白板）。</span>
            </li>
            <li class="flex items-start gap-1.5">
              <span class="badge badge-error badge-xs mt-0.5 shrink-0">卧底胜</span>
              <span>存活卧底人数<strong>大于或等于</strong>存活好人人数（平民好人 + 天使）。</span>
            </li>
            <li class="flex items-start gap-1.5">
              <span class="badge badge-secondary badge-xs mt-0.5 shrink-0">白板胜</span>
              <span>白板白天被公投出局后，在专属猜词阶段<strong>同时准确猜中双方词汇</strong>，直接独立逆袭获胜！</span>
            </li>
          </ul>
        </section>

        <!-- 2. 四大秘境身份 -->
        <section class="bg-base-200/80 rounded-xl p-3 border border-base-300">
          <h4 class="font-bold text-xs text-secondary mb-2 flex items-center gap-1">
            <span>🎭</span> 四大秘境身份
          </h4>
          <div class="grid grid-cols-1 gap-2">
            <!-- 好人 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="flex items-center gap-1.5 font-bold text-primary mb-1">
                <span>🛡️</span>
                <span>平民好人 (Good)</span>
              </div>
              <p class="text-base-content/70 leading-normal">
                获知多数派词（好词）。发词阶段统称「特工」，需在发言中自辨多数派/少数派；白天参与公投，夜间禁止动刀（盲动将导致自身自爆）。
              </p>
            </div>

            <!-- 卧底 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="flex items-center gap-1.5 font-bold text-error mb-1">
                <span>🕵️</span>
                <span>卧底 (Spy)</span>
              </div>
              <p class="text-base-content/70 leading-normal">
                获知少数派词（坏词）。发词阶段统称「特工」自辨阵营；白天伪装发言引导好人互踩，夜间拥有暗杀特权（可刺杀一名特工或静默）。
              </p>
            </div>

            <!-- 天使 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="flex items-center gap-1.5 font-bold text-info mb-1">
                <span>👼</span>
                <span>天使 (Angel)</span>
              </div>
              <p class="text-base-content/70 leading-normal">
                归属好人阵营。<strong>同时获知好词与坏词（顺序随机未知好坏）</strong>；需通过众人发言辨析真假词，在隐蔽自身的前提下暗中引导好人公投卧底。
              </p>
            </div>

            <!-- 白板 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="flex items-center gap-1.5 font-bold text-secondary mb-1">
                <span>🎭</span>
                <span>白板 (Blank)</span>
              </div>
              <p class="text-base-content/70 leading-normal">
                无词（仅有模糊方向提示）。夜间被杀正常出局（不触发猜词）；白天若被公投出局，触发专属双词逆袭特别阶段，猜中好词与坏词即可直接获胜！
              </p>
            </div>
          </div>
        </section>

        <!-- 3. 核心流程与博弈技巧 -->
        <section class="bg-base-200/80 rounded-xl p-3 border border-base-300">
          <h4 class="font-bold text-xs text-warning mb-2 flex items-center gap-1">
            <span>💡</span> 核心流程与实用技巧
          </h4>
          <ul class="space-y-1.5 text-base-content/70 leading-relaxed list-disc list-inside">
            <li><strong>首日一句话发言</strong>：每人仅在第 1 轮白天描述一次，后续轮次无需重复发言，历史发言将在白天自动整理到聊天流底部供随时翻阅比对。</li>
            <li><strong>私密单选投票</strong>：公投全程保密，仅显示本人所投对象，公屏仅播报匿名投票进度；提交后不可更改。</li>
            <li><strong>特工推测便签</strong>：点击聊天区其他玩家头像，可私密标记身份推测（好人/卧底/天使/白板），彩色徽章仅自己可见，终局自动清空。</li>
            <li><strong>断线无感重连</strong>：网络波动或意外刷新时，重新加入自动恢复原有身份与对局状态；中途加入的陌生访客将转入观战席。</li>
          </ul>
        </section>
      </div>

      <!-- 底部关闭按钮 -->
      <div class="modal-action mt-2 pt-2 border-t border-base-200">
        <button class="btn btn-sm btn-primary w-full" @click="close">
          好的，已知悉规则
        </button>
      </div>
    </div>

    <!-- 背景遮罩点击关闭 -->
    <form method="dialog" class="modal-backdrop">
      <button @click="close">关闭</button>
    </form>
  </dialog>
</template>

