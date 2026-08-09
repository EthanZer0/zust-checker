<script lang="ts">
  let { year = '2025', term = '3', onBranch }:
    { year?: string; term?: string; onBranch?: (year: string, term: string) => void } = $props();

  const TERM_OPTIONS = [
    { value: '3', label: '上学期' },
    { value: '12', label: '下学期' },
  ];

  function onYearInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    onBranch?.(val, term);
  }

  function onTermChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    onBranch?.(year, val);
  }
</script>

<div class="term-picker">
  <input type="text" value={year} oninput={onYearInput} placeholder="年份" style="width:80px" />
  <select value={term} onchange={onTermChange}>
    {#each TERM_OPTIONS as o}
      <option value={o.value}>{o.label}</option>
    {/each}
  </select>
</div>

<style>
  .term-picker { display: flex; gap: 6px; align-items: center; }
  select {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    padding: 9px 12px;
    font-size: 13px;
    outline: none;
  }
</style>
