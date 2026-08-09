<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  let { data, kchId, onClose, onAddTarget }: {
    data: any;
    kchId: string;
    onClose: () => void;
    onAddTarget: (jxb: any) => void;
  } = $props();

  let items = $derived(() => {
    if (Array.isArray(data)) return data;
    return data?.items || data?.tmpList || [];
  });

  function clean(s: string): string {
    return (s || '')
      .replace(/<br[^>]*>/gi, ' / ')
      .replace(/\{[^}]*\}/g, '')
      .replace(/<[^>]+>/g, '')
      .trim();
  }

  function teacherName(s: string): string {
    if (!s) return '';
    const parts = s.split('/');
    return parts[1] || parts[0] || '';
  }

  function courseName(): string {
    // data might have course name at top level or inside first item
    if (typeof data?.kcmc === 'string') return data.kcmc;
    if (typeof data?.kch === 'string') return data.kch;
    const first = items()[0];
    if (first) {
      return first.kcmc || first.kch || '';
    }
    return kchId;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="modal-overlay" onclick={onClose} role="presentation" transition:fade={{ duration: 150 }}>
  <div
    class="modal"
    onclick={(e: MouseEvent) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    transition:scale={{ start: 0.96, duration: 200, easing: cubicOut }}
  >
    <div class="modal-header">
      <h2>教学班详情 — {courseName()}</h2>
      <button class="secondary" onclick={onClose}>✕</button>
    </div>

    {#if items().length === 0}
      <div class="status-msg">暂无教学班数据</div>
    {:else}
      <table>
        <thead>
          <tr>
            <th style="width:40px">#</th>
            <th>教师</th>
            <th>时间</th>
            <th>地点</th>
            <th style="width:80px">已选/上限</th>
            <th style="width:40px"></th>
          </tr>
        </thead>
        <tbody>
          {#each items() as d, i}
            <tr>
              <td>{i + 1}</td>
              <td>{teacherName(d.jsxx || d.jsxm || '')}</td>
              <td>{clean(d.sksj || d.sj || '')}</td>
              <td>{clean(d.jxdd || d.cdmc || '')}</td>
              <td>
                {#if d.yxzrs && d.jxbrl && parseInt(d.yxzrs) >= parseInt(d.jxbrl)}
                  <span style="color:var(--error);font-weight:600" title="已选满">已满</span>
                {:else}
                  {d.yxzrs || d.blyxrs || '-'} / {d.jxbrl || '-'}
                {/if}
              </td>
              <td>
                <button
                  class="secondary add-btn"
                  onclick={() => onAddTarget(d)}
                  title="添加到抢课列表"
                >+</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
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
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    padding: 24px;
    min-width: 650px;
    max-width: 850px;
    max-height: 80vh;
    overflow-y: auto;
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 16px;
  }
  h2 { font-size: 16px; color: var(--accent); }
  .status-msg { color: var(--text-secondary); text-align: center; padding: 20px; }
  .add-btn {
    padding: 2px 8px;
    font-size: 14px;
    font-weight: bold;
    color: var(--accent);
    border-color: var(--accent);
    cursor: pointer;
  }
  .add-btn:hover {
    background: var(--accent-soft);
  }
</style>
