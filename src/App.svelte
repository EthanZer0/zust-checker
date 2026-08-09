<script lang="ts">
  import { fade } from 'svelte/transition';
  import { checkSession } from './lib/api';
  import type { UserInfo } from './lib/types';
  import LoginForm from './components/LoginForm.svelte';
  import Dashboard from './components/Dashboard.svelte';
  import Spinner from './components/Spinner.svelte';

  let loggedIn = $state(false);
  let user = $state<UserInfo | null>(null);
  let checkingSession = $state(true);

  $effect(() => {
    checkSession()
      .then((u) => {
        if (u) {
          user = u;
          loggedIn = true;
        }
      })
      .catch(() => {})
      .finally(() => (checkingSession = false));
  });

  function onLoginSuccess(u: UserInfo) {
    user = u;
    loggedIn = true;
  }

  function onLogout() {
    user = null;
    loggedIn = false;
  }
</script>

{#if checkingSession}
  <div class="loading-screen" in:fade={{ duration: 200 }}>
    <Spinner size={28} stroke={3} />
    <p>验证会话中...</p>
  </div>
{:else if loggedIn && user}
  <Dashboard {user} {onLogout} />
{:else}
  <LoginForm {onLoginSuccess} />
{/if}

<style>
  .loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    height: 100vh;
    color: var(--text-secondary);
    font-size: 15px;
  }
</style>
