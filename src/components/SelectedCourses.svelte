<script lang="ts">
  let { data }: { data: any } = $props();

  let items = $derived(() => {
    if (Array.isArray(data)) return data;
    if (data?.items) return data.items;
    if (data?.rows) return data.rows;
    return [];
  });
</script>

{#if items().length === 0}
  <div class="status-msg">暂无已选课程</div>
{:else}
  <table>
    <thead>
      <tr>
        <th style="width:40px">#</th>
        <th>课程名</th>
        <th style="width:80px">教师</th>
        <th style="width:50px">学分</th>
      </tr>
    </thead>
    <tbody>
      {#each items() as c, i}
        <tr>
          <td>{i + 1}</td>
          <td>{c.kcmc || c.kcm || c.courseName || ''}</td>
          <td style="font-size:12px;color:var(--text-secondary)">{c.jsxm || c.jsmc || c.teacherName || ''}</td>
          <td>{c.xf || c.credit || ''}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .status-msg { color: var(--text-secondary); padding: 16px; text-align: center; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  th { text-align: left; padding: 6px 8px; font-size: 10px; color: var(--text-secondary); border-bottom: 1px solid var(--border); }
  td { padding: 5px 8px; border-bottom: 1px solid var(--border); }
</style>
