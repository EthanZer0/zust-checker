<script lang="ts">
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { logout } from '../lib/api';
  import { refreshDashboard } from '../lib/api';
  import type { UserInfo, DashboardData } from '../lib/types';
  import Sidebar from './Sidebar.svelte';
  import UserBadge from './UserBadge.svelte';
  import Spinner from './Spinner.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import GradesTab from './GradesTab.svelte';
  import ScheduleTab from './ScheduleTab.svelte';
  import ExamsTab from './ExamsTab.svelte';
  import CoursesTab from './CoursesTab.svelte';

  let { user, onLogout }: { user: UserInfo; onLogout: () => void } = $props();

  let activeTab = $state('grades');
  let data = $state<DashboardData | null>(null);
  let loading = $state(true);
  let error = $state('');

  async function loadDashboard() {
    loading = true;
    error = '';
    try {
      data = await refreshDashboard();
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function doLogout() {
    try { await logout(); } catch (_) {}
    onLogout();
  }

  $effect(() => { loadDashboard(); });
</script>

<div class="dashboard">
  <Sidebar {activeTab} onTab={(t: string) => activeTab = t} />

  <div class="main">
    <header>
      <UserBadge {user} />
      <div class="header-actions">
        <ThemeToggle />
        <button class="secondary logout-btn" onclick={doLogout}>登出</button>
      </div>
    </header>

    <div class="content">
      {#if loading}
        <div class="status-msg loading-inline">
          <Spinner size={24} stroke={3} />
          <span>加载中...</span>
        </div>
      {:else if error}
        <div class="status-msg error">{error}</div>
      {:else}
        {#key activeTab}
          <div class="tab-pane" in:fly={{ y: 12, duration: 200, easing: cubicOut }}>
            {#if activeTab === 'grades'}
              <GradesTab grades={data?.grades ?? null} />
            {:else if activeTab === 'schedule'}
              <ScheduleTab />
            {:else if activeTab === 'exams'}
              <ExamsTab />
            {:else if activeTab === 'courses'}
              <CoursesTab />
            {/if}
          </div>
        {/key}
      {/if}
    </div>
  </div>
</div>

<style>
  .dashboard {
    display: flex;
    height: 100vh;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    box-shadow: var(--shadow-sm);
    z-index: 1;
  }
  .logout-btn {
    font-size: 12px;
    padding: 6px 14px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .content {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px;
  }
  .status-msg {
    color: var(--text-secondary);
    text-align: center;
    margin-top: 60px;
  }
  .loading-inline {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .status-msg.error {
    color: var(--error);
  }
</style>
