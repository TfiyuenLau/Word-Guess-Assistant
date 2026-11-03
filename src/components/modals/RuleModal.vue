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
          <span>游戏规则与基础玩法</span>
        </h3>
        <button class="btn btn-xs btn-circle btn-ghost" @click="close">✕</button>
      </div>

      <!-- 滚动内容区域 -->
      <div class="max-h-[70vh] overflow-y-auto space-y-3 pr-1 text-xs">
        <!-- 1. 玩法简介与胜负判定 -->
        <section class="bg-base-200/80 rounded-xl p-3 border border-base-300 space-y-2">
          <h4 class="font-bold text-xs text-primary flex items-center gap-1">
            <span>🏆</span> 胜负目标
          </h4>
          <div class="space-y-1.5 text-base-content/80 leading-relaxed">
            <div class="flex items-start gap-1.5">
              <span class="badge badge-primary badge-xs mt-0.5 shrink-0">好人胜</span>
              <span>找出所有卧底并投票放逐，<strong>且白板被找出并猜词失败</strong>。</span>
            </div>
            <div class="flex items-start gap-1.5">
              <span class="badge badge-error badge-xs mt-0.5 shrink-0">卧底胜</span>
              <span>存活的卧底人数<strong>大于或等于</strong>存活的好人人数。</span>
            </div>
            <div class="flex items-start gap-1.5">
              <span class="badge badge-secondary badge-xs mt-0.5 shrink-0">白板胜</span>
              <span>白天被投票出局后，<strong>同时准确猜中好人词与卧底词</strong>，直接独立获胜！</span>
            </div>
          </div>
        </section>

        <!-- 2. 四大角色介绍 -->
        <section class="bg-base-200/80 rounded-xl p-3 border border-base-300 space-y-2">
          <h4 class="font-bold text-xs text-secondary flex items-center gap-1">
            <span>🎭</span> 四大角色说明
          </h4>
          <div class="grid grid-cols-1 gap-2">
            <!-- 好人 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="font-bold text-primary flex items-center gap-1 mb-0.5">
                <span>🛡️</span> 平民好人
              </div>
              <p class="text-base-content/70 leading-normal">
                拿到多数派词（好词）。发词时不显示好人角标，需通过大家发言判断自己是否属于多数派；白天参与公投，夜间禁止动刀。
              </p>
            </div>

            <!-- 卧底 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="font-bold text-error flex items-center gap-1 mb-0.5">
                <span>🕵️</span> 卧底
              </div>
              <p class="text-base-content/70 leading-normal">
                拿到少数派词（坏词）。伪装发言隐藏词义差异；夜间拥有暗杀特权（可选择刺杀一名玩家或放弃）。
              </p>
            </div>

            <!-- 天使 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="font-bold text-info flex items-center gap-1 mb-0.5">
                <span>👼</span> 天使 (好人阵营)
              </div>
              <p class="text-base-content/70 leading-normal">
                <strong>发词时同时看到好词与坏词（顺序随机）</strong>。需通过发言辨析真假词，在不暴露自身身份的前提下引导好人公投卧底。
              </p>
            </div>

            <!-- 白板 -->
            <div class="bg-base-100 p-2.5 rounded-lg border border-base-200">
              <div class="font-bold text-secondary flex items-center gap-1 mb-0.5">
                <span>🎭</span> 白板
              </div>
              <p class="text-base-content/70 leading-normal">
                <strong>没有词</strong>（仅有一句模糊方向提示）。夜间被暗杀正常出局；白天若被投票出局，触发专属猜词机会，完全猜对双词直接翻盘获胜！
              </p>
            </div>
          </div>
        </section>

        <!-- 3. 游戏流程与实用功能 -->
        <section class="bg-base-200/80 rounded-xl p-3 border border-base-300 space-y-2">
          <h4 class="font-bold text-xs text-warning flex items-center gap-1">
            <span>⚙️</span> 流程与实用功能
          </h4>
          <ul class="space-y-1.5 text-base-content/70 leading-relaxed">
            <li class="flex items-start gap-1">
              <span class="text-warning font-bold">•</span>
              <span><strong>首轮一句话描述</strong>：每人仅在第 1 轮白天描述发言一次，后续轮次无需重复发言，历史线索会自动汇总在聊天底部供查阅。</span>
            </li>
            <li class="flex items-start gap-1">
              <span class="text-warning font-bold">•</span>
              <span><strong>私密单选投票</strong>：投票全程保密，公屏仅播报匿名进度，提交后立即锁定不可更改。</span>
            </li>
            <li class="flex items-start gap-1">
              <span class="text-warning font-bold">•</span>
              <span><strong>特工推测便签</strong>：点击聊天区任意玩家头像，可私密标记身份推测（好人/卧底/天使/白板），彩色标记仅自己可见。</span>
            </li>
            <li class="flex items-start gap-1">
              <span class="text-warning font-bold">•</span>
              <span><strong>掉线自动重连</strong>：如果不小心刷新页面或切出后台，重新进入房间会自动恢复原有身份与对局状态。</span>
            </li>
          </ul>
        </section>
      </div>

      <!-- 底部关闭按钮 -->
      <div class="modal-action mt-2 pt-2 border-t border-base-200">
        <button class="btn btn-sm btn-primary w-full font-bold" @click="close">
          知道了，开始对决
        </button>
      </div>
    </div>

    <!-- 背景遮罩点击关闭 -->
    <form method="dialog" class="modal-backdrop">
      <button @click="close">关闭</button>
    </form>
  </dialog>
</template>
