<script lang="ts">
  import { getSchedule, currentSemester } from '../lib/api';
  import type { ScheduleEntry } from '../lib/types';
  import TermPicker from './TermPicker.svelte';
  import Spinner from './Spinner.svelte';
  import EmptyState from './EmptyState.svelte';

  let entries = $state<ScheduleEntry[]>([]);
  let loading = $state(true);
  let error = $state('');
  const sem = currentSemester();
  let year = $state(sem.year);
  let term = $state(sem.term);
  let viewMode = $state<'list' | 'grid'>('grid');

  async function load() {
    loading = true;
    error = '';
    try {
      entries = await getSchedule(year, term);
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { load(); });

  // ── Grid view helpers ──
  const DAYS = ['星期一', '星期二', '星期三', '星期四', '星期五', '星期六', '星期日'];
  // Individual periods 1–12 (common university range)
  const PERIODS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

  // Warm, muted tints — distinguishable yet cohesive on both light (cream)
  // and dark backdrops. Returned value is used directly as cell background.
  const COURSE_TINTS = [
    'rgba(194, 112, 61, 0.16)',  // terracotta
    'rgba(201, 154, 46, 0.16)',  // goldenrod
    'rgba(122, 154, 109, 0.16)', // sage
    'rgba(79, 154, 148, 0.15)',  // dusty teal
    'rgba(165, 106, 138, 0.15)', // mauve
    'rgba(126, 132, 156, 0.16)', // muted slate
  ];
  function courseColor(name: string): string {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
    return COURSE_TINTS[h % COURSE_TINTS.length];
  }

  // Expand each entry into individual-period cells, matching by span
  function expandEntries(): { day: string; period: number; session: string; entry: ScheduleEntry | null }[] {
    const map = new Map<string, { day: string; period: number; session: string; entry: ScheduleEntry | null }>();
    for (const e of entries) {
      const [start, end] = e.sessions.split('-').map(Number);
      if (!start || !end) continue;
      for (let p = start; p <= end; p++) {
        const key = `${e.day}|${p}`;
        // Only first period of a span gets the full entry; others get a "continuation" marker
        if (!map.has(key)) {
          map.set(key, { day: e.day, period: p, session: e.sessions, entry: e });
        }
      }
    }
    return Array.from(map.values());
  }

  // Derived: grid lookup — recalculates when entries change
  let gridLookup = $derived(
    new Map(
      expandEntries().map(x => [`${x.day}|${x.period}`, x.entry] as [string, ScheduleEntry | null])
    )
  );

  function gridEntry(day: string, period: number): ScheduleEntry | undefined {
    const e = gridLookup.get(`${day}|${period}`);
    return e ?? undefined;
  }

  function isSpanStart(day: string, period: number): boolean {
    const e = gridLookup.get(`${day}|${period}`);
    if (!e) return false;
    const prev = gridLookup.get(`${day}|${period - 1}`);
    return prev !== e; // new entry starts here (not continued from above)
  }

  function rowspan(day: string, period: number): number {
    const e = gridLookup.get(`${day}|${period}`);
    if (!e) return 1;
    let count = 1;
    while (gridLookup.get(`${day}|${period + count}`) === e) {
      count++;
    }
    return count;
  }
</script>

<div class="schedule-tab">
  <div class="toolbar">
    <TermPicker year={year} term={term} onBranch={(yy: string, tt: string) => { year = yy; term = tt; load(); }} />
    <button onclick={load} disabled={loading}>刷新</button>
    <span class="spacer"></span>
    <div class="view-toggle">
      <button onclick={() => viewMode = 'list'} class:active={viewMode === 'list'}>列表</button>
      <button onclick={() => viewMode = 'grid'} class:active={viewMode === 'grid'}>表格</button>
    </div>
  </div>

  {#if loading}
    <div class="status-msg loading-inline">
      <Spinner size={24} stroke={3} />
      <span>加载中...</span>
    </div>
  {:else if error}
    <div class="status-msg error">{error}</div>
  {:else if entries.length === 0}
    <EmptyState icon="📅" text="暂无课表数据" />
  {:else if viewMode === 'list'}
    <table>
      <thead>
        <tr>
          <th class="w-day">星期</th>
          <th class="w-slot">节次</th>
          <th>课程</th>
          <th>教师</th>
          <th>地点</th>
          <th>周次</th>
        </tr>
      </thead>
      <tbody>
        {#each entries as e}
          <tr>
            <td>{e.day}</td>
            <td>{e.sessions}</td>
            <td>{e.course_name}</td>
            <td>{e.teacher}</td>
            <td>{e.location}</td>
            <td class="col-weeks">{e.weeks}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <div class="grid-wrapper">
      <table class="grid-table">
        <thead>
          <tr>
            <th class="period-col"></th>
            {#each DAYS as d}
              <th>{d}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each PERIODS as p}
            <tr>
              <td class="period-col">{p}</td>
              {#each DAYS as day}
                {@const show = isSpanStart(day, p)}
                {#if show}
                  {@const entry = gridEntry(day, p)!}
                  {@const rs = rowspan(day, p)}
                  <td
                    class="grid-cell course"
                    rowspan={rs}
                    style="background:{courseColor(entry.course_name)}"
                  >
                    <div class="gc-name">{entry.course_name}</div>
                    <div class="gc-teacher">{entry.teacher}</div>
                    <div class="gc-room">{entry.location}</div>
                    <div class="gc-weeks">{entry.weeks}</div>
                  </td>
                {:else if !gridEntry(day, p)}
                  <td class="grid-cell none"></td>
                {/if}
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .schedule-tab { max-width: 1100px; }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
  }
  .toolbar button {
    padding: 5px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
    transition: background var(--dur-fast) var(--ease);
  }
  .toolbar button:hover { background: var(--bg-hover); }
  .toolbar button.active { background: var(--accent); color: var(--bg-primary); border-color: var(--accent); }
  .toolbar button:disabled { opacity: 0.5; cursor: default; }
  .spacer { flex: 1; }
  .view-toggle { display: flex; gap: 0; border-radius: var(--radius-sm); overflow: hidden; }
  .view-toggle button { border-radius: 0; margin: 0; }
  .view-toggle button:first-child { border-radius: var(--radius-sm) 0 0 var(--radius-sm); }
  .view-toggle button:last-child { border-radius: 0 var(--radius-sm) var(--radius-sm) 0; }

  .status-msg { color: var(--text-secondary); text-align: center; margin-top: 40px; }
  .status-msg.error { color: var(--error); }
  .loading-inline {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  /* ── List table ── */
  .w-day { width: 70px; }
  .w-slot { width: 80px; }
  .col-weeks { font-size: 11px; color: var(--text-secondary); }

  /* ── Grid table ── */
  .grid-wrapper { overflow-x: auto; }
  .grid-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  .grid-table th,
  .grid-table td {
    border: 1px solid var(--border);
    vertical-align: top;
    text-align: center;
    font-size: 12px;
  }
  .grid-table th {
    padding: 8px 4px;
    background: var(--bg-secondary);
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
  }
  .period-col {
    width: 36px;
    padding: 4px 2px !important;
    font-size: 11px !important;
    color: var(--text-secondary);
    background: var(--bg-secondary) !important;
    vertical-align: middle !important;
    font-weight: 500;
  }

  .grid-cell { padding: 4px 3px; height: 36px; }
  .grid-cell.none {
    background: transparent;
  }
  .grid-cell.course {
    background: var(--bg-card);
    min-width: 126px;
    transition: filter var(--dur-fast) var(--ease);
  }
  .grid-cell.course:hover {
    filter: brightness(1.18);
  }
  :global([data-theme="light"]) .grid-cell.course:hover {
    filter: brightness(0.96);
  }
  .gc-name {
    font-weight: 600;
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.3;
  }
  .gc-teacher { font-size: 11px; color: var(--text-secondary); line-height: 1.3; }
  .gc-room   { font-size: 11px; color: var(--text-secondary); line-height: 1.3; }
  .gc-weeks  {
    font-size: 10px;
    color: var(--accent);
    margin-top: 1px;
    line-height: 1.3;
  }
</style>
