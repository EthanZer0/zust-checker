<script lang="ts">
  import type { UserInfo } from '../lib/types';
  let { user }: { user: UserInfo } = $props();

  let initial = $derived((user.name || '?').trim().charAt(0));
</script>

<div class="user-badge">
  <span class="avatar">{initial}</span>
  <div class="info">
    <span class="name">{user.name}</span>
    <span class="meta">{user.student_id} · {user.department}</span>
  </div>
</div>

<style>
  .user-badge {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .avatar {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 700;
    flex-shrink: 0;
    border: 1px solid var(--border);
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    line-height: 1.3;
  }
  .name { font-weight: 600; color: var(--text-primary); font-size: 14px; }
  .meta { font-size: 12px; color: var(--text-secondary); }
</style>
