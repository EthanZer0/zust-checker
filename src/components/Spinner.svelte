<script lang="ts">
  let { size = 20, stroke = 2, color = 'var(--accent)' }:
    { size?: number; stroke?: number; color?: string } = $props();
</script>

<span
  class="spinner"
  style="width:{size}px;height:{size}px;border-width:{stroke}px;border-top-color:{color}"
  role="status"
  aria-label="加载中"
></span>

<style>
  .spinner {
    display: inline-block;
    border-style: solid;
    border-color: var(--border);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
