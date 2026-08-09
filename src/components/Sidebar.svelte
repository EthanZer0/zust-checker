<script lang="ts">
  let { activeTab, onTab }: { activeTab: string; onTab: (tab: string) => void } = $props();

  const tabs = [
    { id: 'grades', label: '成绩', icon: '📊' },
    { id: 'schedule', label: '课表', icon: '📅' },
    { id: 'exams', label: '考试', icon: '📝' },
    { id: 'courses', label: '选课', icon: '📚' },
  ];
</script>

<nav>
  <div class="logo"><img src="/logo.png" alt="ZUST" /></div>
  {#each tabs as tab}
    <button
      class="nav-item"
      class:active={activeTab === tab.id}
      onclick={() => onTab(tab.id)}
    >
      <span class="icon">{tab.icon}</span>
      <span class="label">{tab.label}</span>
    </button>
  {/each}
</nav>

<style>
  nav {
    width: 200px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    padding: 16px 0;
    display: flex;
    flex-direction: column;
  }
  .logo {
    padding: 8px 16px 20px;
    display: flex;
    justify-content: center;
  }
  .logo img {
    width: 60%;
    height: auto;
    display: block;
    opacity: 0.9;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 2px 10px;
    padding: 11px 14px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
    text-align: left;
    width: calc(100% - 20px);
    position: relative;
  }
  .nav-item::before {
    content: '';
    position: absolute;
    left: -10px;
    top: 50%;
    transform: translateY(-50%) scaleY(0);
    width: 3px;
    height: 60%;
    background: var(--accent);
    border-radius: 0 2px 2px 0;
    transition: transform var(--dur) var(--ease);
  }
  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    transform: none;
    box-shadow: none;
  }
  .nav-item:hover .icon {
    transform: scale(1.15);
  }
  .nav-item.active {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .nav-item.active::before {
    transform: translateY(-50%) scaleY(1);
  }
  .icon {
    font-size: 16px;
    transition: transform var(--dur-fast) var(--ease);
  }
</style>
