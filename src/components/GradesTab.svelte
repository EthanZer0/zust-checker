<script lang="ts">
  import { getGrades, currentSemester } from '../lib/api';
  import type { GradeSummary } from '../lib/types';
  import GradeSummaryWidget from './GradeSummary.svelte';
  import GradeTable from './GradeTable.svelte';
  import TermPicker from './TermPicker.svelte';
  import TableSkeleton from './TableSkeleton.svelte';
  import EmptyState from './EmptyState.svelte';

  let { grades: initialGrades } = $props<{ grades: GradeSummary | null }>();

  let grades = $state(initialGrades);
  let loading = $state(false);
  let error = $state('');

  // Derive default year/term from most recent semester in data, fallback to current date
  function latestYearTerm(d: GradeSummary | null): [string, string] {
    const sems = d?.semesters ?? [];
    const years: number[] = sems.map(s => parseInt(s.year)).filter(n => !isNaN(n));
    years.sort((a, b) => b - a);
    if (years.length > 0) {
      const latestYear = String(years[0]);
      const hasSecond = sems.some(s => s.year === latestYear && s.term === '12');
      return [latestYear, hasSecond ? '12' : '3'];
    }
    const cs = currentSemester();
    return [cs.year, cs.term];
  }

  const [defY, defT] = latestYearTerm(initialGrades);
  let yr = $state(defY);
  let tm = $state(defT);
  let customTerm = $state(false);

  async function loadBranch(qy?: string, qt?: string) {
    const ay = qy ?? yr;
    const at = qt ?? tm;
    loading = true;
    error = '';
    try {
      grades = await getGrades(ay, at);
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function gpaClass(g: number): string {
    if (g >= 4.5) return 'gpa-green';
    if (g >= 3.5) return 'gpa-cyan';
    if (g >= 2.5) return 'gpa-yellow';
    return 'gpa-red';
  }
</script>

<div class="grades-tab">
  {#if grades}
    <div class="toolbar">
      <button class="secondary" onclick={() => {
        customTerm = !customTerm;
        if (customTerm) {
          // 切换到"选择学期"时，用当前选中的学期重新请求
          loadBranch(yr, tm);
        } else {
          // 切换到"全部学期"时，请求全部数据
          loadBranch('', '');
        }
      }}>
        {customTerm ? '全部学期' : '选择学期'}
      </button>
      {#if customTerm}
        <TermPicker
          year={yr} term={tm}
          onBranch={(yy: string, tt: string) => { yr = yy; tm = tt; loadBranch(yy, tt); }}
        />
        <button class="secondary" onclick={() => loadBranch()} disabled={loading}>查询</button>
      {/if}
    </div>

    {#if loading}
      <div class="skeleton-wrap">
        <TableSkeleton
          rows={8}
          headers={['课程名称', '成绩', '学分', '绩点', '类型']}
          cols={['45%', '60px', '50px', '50px', '80px']}
        />
      </div>
    {:else if error}
      <div class="status-msg error">{error}</div>
    {:else}
      <GradeSummaryWidget gpa={grades.gpa} count={grades.courses.length} credits={grades.total_credits} />

      {#each (customTerm ? [{ key: '', year: yr, term: tm, term_name: '', courses: grades.courses, gpa: grades.gpa }] : grades.semesters) as sem}
        {@const label = customTerm ? `${yr} ${tm === '3' ? '上学期' : '下学期'}` : sem.key}
        <div class="semester-block">
          <h3>
            {label}
            <span class="sem-gpa {gpaClass(sem.gpa)}">{sem.gpa.toFixed(2)}</span>
          </h3>
          <GradeTable courses={sem.courses} />
        </div>
      {/each}
    {/if}
  {:else}
    <EmptyState icon="📊" text="暂无成绩数据" />
  {/if}
</div>

<style>
  .grades-tab { max-width: 900px; }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
  }
  .semester-block { margin-bottom: 24px; }
  h3 {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 14px;
    color: var(--text-primary);
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border);
  }
  .sem-gpa { font-size: 16px; font-weight: 700; }
  .skeleton-wrap { margin-top: 8px; }
  .status-msg { color: var(--text-secondary); text-align: center; margin-top: 40px; }
  .status-msg.error { color: var(--error); }
</style>
