<script lang="ts">
  import type { GradeEntry } from '../lib/types';
  let { courses }: { courses: GradeEntry[] } = $props();

  function scoreClass(s: string): string {
    const n = parseFloat(s);
    if (!isNaN(n)) {
      if (n >= 90) return 'gpa-green';
      if (n >= 80) return 'gpa-cyan';
      if (n >= 70) return 'gpa-yellow';
    }
    if (s === '优秀' || s === '良好') return 'gpa-green';
    if (s === '中等') return 'gpa-cyan';
    if (s === '合格' || s === '及格') return 'gpa-yellow';
    return 'gpa-red';
  }
</script>

<table>
  <thead>
    <tr>
      <th>课程名称</th>
      <th style="width:60px">成绩</th>
      <th style="width:50px">学分</th>
      <th style="width:50px">绩点</th>
      <th style="width:80px">类型</th>
    </tr>
  </thead>
  <tbody>
    {#each courses as c}
      <tr>
        <td>{c.name}</td>
        <td class={scoreClass(c.score)}>{c.score}</td>
        <td>{c.credit}</td>
        <td>{c.point}</td>
        <td style="color:var(--text-secondary);font-size:12px">{c.nature}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    text-align: left;
    padding: 8px 10px;
    font-size: 11px;
    color: var(--text-secondary);
    border-bottom: 2px solid var(--border);
  }
  td {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border);
  }
</style>
