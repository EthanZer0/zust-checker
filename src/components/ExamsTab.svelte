<script lang="ts">
  import { getExams, currentSemester } from '../lib/api';
  import type { ExamEntry } from '../lib/types';
  import TermPicker from './TermPicker.svelte';
  import TableSkeleton from './TableSkeleton.svelte';
  import EmptyState from './EmptyState.svelte';

  let entries = $state<ExamEntry[]>([]);
  let loading = $state(true);
  let error = $state('');
  const sem = currentSemester();
  let year = $state(sem.year);
  let term = $state(sem.term);

  async function load() {
    loading = true;
    error = '';
    try {
      entries = await getExams(year, term);
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { load(); });
</script>

<div class="exams-tab">
  <div class="toolbar">
    <TermPicker year={year} term={term} onBranch={(yy: string, tt: string) => { year = yy; term = tt; }} />
    <button class="secondary" onclick={load} disabled={loading}>刷新</button>
  </div>

  {#if loading}
    <TableSkeleton
      rows={5}
      headers={['课程', '时间', '地点', '座位号']}
      cols={['40%', '180px', '30%', '80px']}
    />
  {:else if error}
    <div class="status-msg error">{error}</div>
  {:else if entries.length === 0}
    <EmptyState icon="📝" text="暂无考试安排" />
  {:else}
    <table>
      <thead>
        <tr>
          <th>课程</th>
          <th style="width:180px">时间</th>
          <th>地点</th>
          <th style="width:80px">座位号</th>
        </tr>
      </thead>
      <tbody>
        {#each entries as e}
          <tr>
            <td>{e.course_name}</td>
            <td>{e.datetime}</td>
            <td>{e.location}</td>
            <td>{e.seat}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .exams-tab { max-width: 800px; }
  .toolbar { display: flex; gap: 10px; margin-bottom: 16px; }
  .status-msg { color: var(--text-secondary); text-align: center; margin-top: 40px; }
  .status-msg.error { color: var(--error); }
</style>
