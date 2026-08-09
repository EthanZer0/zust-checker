<script lang="ts">
  import type { CourseTab } from '../lib/types';

  let { tabs = [], activeTabIdx = 0, onSelect }: {
    tabs?: CourseTab[];
    activeTabIdx?: number;
    onSelect?: (idx: number) => void;
  } = $props();
</script>

{#if tabs.length === 0}
  <div class="status-msg">暂无选课板块（可能不在选课周期）</div>
{:else}
  <div class="tab-selector">
    {#each tabs as t, i}
      <button
        class="tab-btn"
        class:active={i === activeTabIdx}
        onclick={() => onSelect?.(i)}
      >
        {t.name}
      </button>
    {/each}
  </div>
{/if}

<style>
  .tab-selector {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 12px;
  }
  .tab-btn {
    padding: 6px 14px;
    font-size: 12px;
    border-radius: var(--radius-pill);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .tab-btn:hover {
    border-color: var(--accent);
    color: var(--text-primary);
    transform: none;
  }
  .tab-btn.active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .status-msg { color: var(--text-secondary); padding: 16px; text-align: center; }
</style>
