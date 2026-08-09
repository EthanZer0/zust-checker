<script lang="ts">
  let { rows = 5, cols = ['40%', '30%', '20%', '20%'], headers = null }:
    { rows?: number; cols?: string[]; headers?: string[] | null } = $props();
</script>

<table>
  {#if headers}
    <thead>
      <tr>
        {#each headers as h}<th>{h}</th>{/each}
      </tr>
    </thead>
  {/if}
  <tbody>
    {#each Array(rows) as _}
      <tr>
        {#each cols as w}
          <td><span class="skeleton-line" style="width:{w}"></span></td>
        {/each}
      </tr>
    {/each}
  </tbody>
</table>

<style>
  table { width: 100%; border-collapse: collapse; font-size: 13px; }
  td { padding: 8px 12px; border-bottom: 1px solid var(--border); }
</style>
