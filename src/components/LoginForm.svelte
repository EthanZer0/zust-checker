<script lang="ts">
  import { fly } from 'svelte/transition';
  import { login, loadCredentials } from '../lib/api';
  import { listen } from '@tauri-apps/api/event';
  import type { UserInfo, LoginStep } from '../lib/types';
  import LoginProgress from './LoginProgress.svelte';
  import Spinner from './Spinner.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  let { onLoginSuccess }: { onLoginSuccess: (u: UserInfo) => void } = $props();

  let username = $state('');
  let casPassword = $state('');
  let wiseduPassword = $state('');
  let loggingIn = $state(false);
  let error = $state('');
  let steps = $state<LoginStep[]>([]);

  // 启动时尝试加载缓存凭据
  $effect(() => {
    loadCredentials().then(creds => {
      if (creds) {
        username = creds.username;
        casPassword = creds.cas_password;
        wiseduPassword = creds.wisedu_password;
      }
    });
  });

  async function doLogin() {
    if (!username || !casPassword) return;
    loggingIn = true;
    error = '';
    steps = [];

    // Listen for progress events
    const unlisten = listen<LoginStep>('login-step', (e) => {
      steps = [...steps, e.payload];
    });

    try {
      const u = await login(username, casPassword, wiseduPassword || casPassword);
      onLoginSuccess(u);
    } catch (e: any) {
      error = String(e);
    } finally {
      loggingIn = false;
      (await unlisten)();
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') doLogin();
  }
</script>

<div class="login-container">
  <div class="theme-corner">
    <ThemeToggle />
  </div>
  <div class="login-card" in:fly={{ y: 16, duration: 280 }}>
    <h1>ZUST 教务查询</h1>
    <p class="subtitle">浙江科技大学 · 方正教务系统</p>

    <div class="form">
      <label>学号</label>
      <input type="text" bind:value={username} placeholder="Student ID" disabled={loggingIn} onkeydown={onKeydown} />

      <label>CAS 密码（统一认证）</label>
      <input type="password" bind:value={casPassword} placeholder="CAS Password" disabled={loggingIn} onkeydown={onKeydown} />

      <label>教务密码（留空则与 CAS 相同）</label>
      <input type="password" bind:value={wiseduPassword} placeholder="Wisedu Password (optional)" disabled={loggingIn} onkeydown={onKeydown} />

      <button onclick={doLogin} disabled={loggingIn || !username || !casPassword}>
        {#if loggingIn}
          <Spinner size={15} stroke={2} color="var(--bg-primary)" />
          <span>登录中...</span>
        {:else}
          登录
        {/if}
      </button>

      {#if error}
        <div class="error">{error}</div>
      {/if}
    </div>

    {#if steps.length > 0}
      <LoginProgress {steps} />
    {/if}
  </div>
</div>

<style>
  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, var(--bg-primary) 0%, var(--bg-secondary) 100%);
  }
  .theme-corner {
    position: fixed;
    top: 16px;
    right: 16px;
  }
  .login-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 40px;
    width: 400px;
    box-shadow: var(--shadow-lg);
  }
  h1 {
    font-size: 24px;
    text-align: center;
    margin-bottom: 6px;
    color: var(--accent);
  }
  .subtitle {
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    margin-bottom: 28px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  label {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 8px;
  }
  button {
    margin-top: 20px;
    width: 100%;
    padding: 12px;
    font-size: 15px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .error {
    background: rgba(248, 113, 113, 0.12);
    color: var(--error);
    padding: 10px;
    border-radius: var(--radius);
    font-size: 13px;
    margin-top: 10px;
  }
</style>
