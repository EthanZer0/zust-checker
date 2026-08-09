<script lang="ts">
  import { themeState, toggleTheme } from '../lib/theme.svelte';
</script>

<button
  class="theme-toggle secondary"
  onclick={toggleTheme}
  title={themeState.current === 'dark' ? '切换到浅色主题' : '切换到深色主题'}
  aria-label="切换主题"
>
  <span class="icon">{themeState.current === 'dark' ? '☀️' : '🌙'}</span>
</button>

<style>
  .theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-pill);
    font-size: 15px;
    line-height: 1;
  }
  .icon {
    display: block;
    transition: transform var(--dur) var(--ease);
  }
  .theme-toggle:hover .icon {
    transform: rotate(20deg) scale(1.1);
  }
</style>
