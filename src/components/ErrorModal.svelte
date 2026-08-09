<script lang="ts">
  let { title, message, details, onClose }: {
    title: string;
    message: string;
    details?: string;
    onClose: () => void;
  } = $props();
</script>

<div class="modal-overlay" onclick={onClose} role="presentation">
  <div class="modal" onclick={(e: MouseEvent) => e.stopPropagation()} role="dialog">
    <div class="modal-header">
      <h2>{title}</h2>
      <button class="secondary" onclick={onClose}>✕</button>
    </div>
    <p class="message">{message}</p>
    {#if details}
      <pre class="details">{details}</pre>
    {/if}
    <button onclick={onClose} style="margin-top:16px">确定</button>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    max-width: 500px;
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 12px;
  }
  h2 { font-size: 16px; }
  .message { color: var(--text-primary); margin-bottom: 8px; }
  .details {
    background: var(--bg-secondary);
    padding: 10px;
    border-radius: var(--radius);
    font-size: 11px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 200px;
    overflow-y: auto;
  }
</style>
