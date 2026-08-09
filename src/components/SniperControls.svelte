<script lang="ts">
  import type { SniperTarget, SniperTickResult } from '../lib/types';

  let { targets, removeTarget, running, sniperInterval, sniperStagger = 80, onStart, onStop, sniperLog, sniperSuccess, sniperSessionExpired = false, sniperReconnecting = false, onIntervalChange, onStaggerChange }:
    {
      targets: SniperTarget[]; removeTarget: (i: number) => void;
      running: boolean; sniperInterval: number; sniperStagger?: number;
      onStart: () => void; onStop: () => void;
      sniperLog: SniperTickResult[]; sniperSuccess: boolean;
      sniperSessionExpired?: boolean;
      sniperReconnecting?: boolean;
      onIntervalChange: (v: number) => void;
      onStaggerChange?: (v: number) => void;
    } = $props();
</script>

<div class="sniper-panel">
  <h3>🎯 抢课目标 ({targets.length})</h3>

  <div class="targets">
    {#each targets as t, i}
      <div class="target-chip">
        <span>{t.kch_id}</span>
        <button class="secondary" style="padding:1px 6px;font-size:10px" onclick={() => removeTarget(i)}>✕</button>
      </div>
    {/each}
  </div>

  <div class="controls">
    <label>
      间隔(ms): <input type="number" bind:value={sniperInterval} oninput={(e: any) => onIntervalChange(parseInt(e.target.value) || 900)} style="width:80px" min={200} max={10000} disabled={running} />
    </label>
    <label>
      错开(ms): <input type="number" bind:value={sniperStagger} oninput={(e: any) => onStaggerChange?.(parseInt(e.target.value) || 0)} style="width:75px" min={0} max={500} disabled={running} />
    </label>

    {#if running}
      <button onclick={onStop} class="stop-btn">停止抢课</button>
    {:else}
      <button onclick={onStart} disabled={targets.length === 0}>开始抢课</button>
    {/if}
  </div>

  {#if sniperSuccess}
    <div class="success-msg">✅ 抢课成功！</div>
  {/if}

  {#if sniperSessionExpired}
    <div class="session-expired-msg">⚠️ Session 已失效，请重新登录后再次启动抢课</div>
  {/if}

  {#if sniperReconnecting}
    <div class="reconnecting-msg">🔄 Session 过期，正在自动重登...</div>
  {/if}

  {#if sniperLog.length > 0}
    <div class="log">
      <h4>日志 (最近 20 条)</h4>
      {#each sniperLog.slice(0, 20) as entry}
        <div class="log-entry">
          <span class="log-time">{entry.elapsed_secs.toFixed(1)}s</span>
          <span class="log-attempts">{entry.total_attempts} 次</span>
          {#each entry.targets as t}
            <span class="log-status" class:success={t.status === 'success'}
              class:conflict={t.status === 'conflict'} class:unavailable={t.status === 'unavailable'}
              class:rate-limited={t.status === 'rate_limited'}>
              [{t.status}] {t.detail.slice(0, 60)}
            </span>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sniper-panel {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px;
    margin: 12px 0;
  }
  h3 { font-size: 13px; margin-bottom: 8px; color: var(--accent); }
  h4 { font-size: 11px; color: var(--text-secondary); margin-bottom: 6px; }
  .targets { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 10px; }
  .target-chip {
    display: flex; align-items: center; gap: 4px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    padding: 2px 10px;
    font-size: 11px;
  }
  .controls {
    display: flex; align-items: center; gap: 12px;
  }
  .controls label { font-size: 12px; color: var(--text-secondary); }
  .stop-btn { background: var(--error) !important; color: var(--bg-primary) !important; }
  .success-msg {
    background: rgba(74, 222, 128, 0.12); color: var(--success);
    padding: 10px; border-radius: var(--radius);
    margin-top: 10px; font-weight: 600;
  }
  .session-expired-msg {
    background: rgba(251, 191, 36, 0.12); color: var(--warning);
    padding: 10px; border-radius: var(--radius);
    margin-top: 10px; font-weight: 600;
  }
  .reconnecting-msg {
    background: rgba(129, 140, 248, 0.12); color: var(--accent);
    padding: 10px; border-radius: var(--radius);
    margin-top: 10px; font-weight: 600;
  }
  .log {
    margin-top: 10px;
    max-height: 200px;
    overflow-y: auto;
    font-size: 11px;
    background: var(--bg-secondary);
    border-radius: var(--radius);
    padding: 8px;
  }
  .log-entry {
    padding: 3px 0;
    border-bottom: 1px solid var(--border);
  }
  .log-time { color: var(--text-secondary); margin-right: 8px; }
  .log-attempts { margin-right: 8px; }
  .log-status.success { color: var(--success); }
  .log-status.conflict { color: var(--warning); }
  .log-status.unavailable { color: var(--error); }
  .log-status.rate-limited { color: var(--warning); }
</style>
