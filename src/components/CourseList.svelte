<script lang="ts">
  let { courses, onDetail }: {
    courses: any[];
    onDetail: (kch_id: string) => void;
    onAddTarget: (course: any) => void;
  } = $props();

  // Deduplicate by kch_id
  let deduped = $derived(() => {
    const seen = new Set<string>();
    return courses.filter((c: any) => {
      const k = c.kch_id || c.kch || c.kcmc;
      if (!k || seen.has(k)) return false;
      seen.add(k);
      return true;
    });
  });
</script>

<table>
  <thead>
    <tr>
      <th style="width:40px">#</th>
      <th>课程名</th>
      <th style="width:50px">学分</th>
      <th style="width:70px">类型</th>
      <th style="width:60px">操作</th>
    </tr>
  </thead>
  <tbody>
    {#each deduped() as c, i}
      <tr>
        <td>{i + 1}</td>
        <td>{c.kcmc || c.kcm || c.courseName || ''}</td>
        <td>{c.xf || c.credit || ''}</td>
        <td style="font-size:11px;color:var(--text-secondary)">
          {c.kzmc || c.kclbmc || c.kcgsmc || ''}
        </td>
        <td>
          <button class="secondary" style="padding:3px 8px;font-size:11px"
            onclick={() => onDetail(c.kch_id || c.kch || '')}>
            详情
          </button>
        </td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  th { text-align: left; padding: 6px 8px; font-size: 10px; color: var(--text-secondary); border-bottom: 1px solid var(--border); }
  td { padding: 5px 8px; border-bottom: 1px solid var(--border); }
</style>
