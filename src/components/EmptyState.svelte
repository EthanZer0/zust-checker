<script lang="ts">
  let { icon = '📭', text = '暂无数据', hint = '' }:
    { icon?: string; text?: string; hint?: string } = $props();
</script>

<div class="empty-state">
  <span class="es-icon">{icon}</span>
  <span class="es-text">{text}</span>
  {#if hint}<span class="es-hint">{hint}</span>{/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 48px 20px;
    color: var(--text-secondary);
    text-align: center;
  }
  .es-icon {
    font-size: 36px;
    opacity: 0.6;
    line-height: 1;
  }
  .es-text {
    font-size: 14px;
  }
  .es-hint {
    font-size: 12px;
    opacity: 0.7;
  }
</style>
