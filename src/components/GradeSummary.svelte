<script lang="ts">
  let { gpa, count, credits }: { gpa: number; count: number; credits: number } = $props();

  function gpaClass(g: number): string {
    if (g >= 4.5) return 'gpa-green';
    if (g >= 3.5) return 'gpa-cyan';
    if (g >= 2.5) return 'gpa-yellow';
    return 'gpa-red';
  }
</script>

<div class="summary">
  <div class="stat card">
    <span class="stat-label">GPA</span>
    <span class="stat-value {gpaClass(gpa)}">{gpa.toFixed(2)}</span>
  </div>
  <div class="stat card">
    <span class="stat-label">课程数</span>
    <span class="stat-value">{count}</span>
  </div>
  <div class="stat card">
    <span class="stat-label">学分</span>
    <span class="stat-value">{credits.toFixed(0)}</span>
  </div>
</div>

<style>
  .summary {
    display: flex;
    gap: 16px;
    margin-bottom: 20px;
  }
  .stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px 20px;
  }
  .stat-label {
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .stat-value {
    font-size: 26px;
    font-weight: 700;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
</style>
