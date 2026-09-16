<script lang="ts">
  import { fly } from 'svelte/transition';
  import {
    login,
    loadCredentials,
    refreshCaptcha as refreshCaptchaApi,
    sendReauthCode,
    verifyReauthCode,
  } from '../lib/api';
  import { listen } from '@tauri-apps/api/event';
  import type {
    CaptchaChallenge,
    LoginResponse,
    LoginStep,
    ReauthChallenge,
    UserInfo,
  } from '../lib/types';
  import LoginProgress from './LoginProgress.svelte';
  import Spinner from './Spinner.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  let { onLoginSuccess }: { onLoginSuccess: (u: UserInfo) => void } = $props();

  let username = $state('');
  let casPassword = $state('');
  let wiseduPassword = $state('');
  let captcha = $state('');
  let dynamicCode = $state('');
  let trustDevice = $state(false);
  let captchaChallenge = $state<CaptchaChallenge | null>(null);
  let reauthChallenge = $state<ReauthChallenge | null>(null);
  let loggingIn = $state(false);
  let sendingCode = $state(false);
  let verifyingCode = $state(false);
  let smsSent = $state(false);
  let smsMessage = $state('');
  let error = $state('');
  let steps = $state<LoginStep[]>([]);

  // 启动时尝试加载缓存凭据。
  $effect(() => {
    loadCredentials().then((creds) => {
      if (creds) {
        username = creds.username;
        casPassword = creds.cas_password;
        wiseduPassword = creds.wisedu_password;
      }
    });
  });

  async function withProgress<T>(action: () => Promise<T>): Promise<T> {
    const unlisten = await listen<LoginStep>('login-step', (event) => {
      steps = [...steps, event.payload];
    });
    try {
      return await action();
    } finally {
      unlisten();
    }
  }

  async function doLogin() {
    if (!username || !casPassword || reauthChallenge) return;
    loggingIn = true;
    error = '';
    steps = [];

    try {
      const result: LoginResponse = await withProgress(() =>
        login(username, casPassword, wiseduPassword || casPassword, captcha || null),
      );
      if (result.status === 'success') {
        onLoginSuccess(result.user);
      } else if (result.status === 'captcha_required') {
        captchaChallenge = result.challenge;
        captcha = '';
        reauthChallenge = null;
      } else {
        reauthChallenge = result.challenge;
        captchaChallenge = null;
        dynamicCode = '';
        smsSent = false;
        smsMessage = '';
      }
    } catch (e: any) {
      error = String(e);
    } finally {
      loggingIn = false;
    }
  }

  async function refreshCaptcha() {
    error = '';
    try {
      captchaChallenge = await refreshCaptchaApi();
      captcha = '';
    } catch (e: any) {
      error = String(e);
    }
  }

  async function requestSmsCode() {
    if (sendingCode || !reauthChallenge) return;
    sendingCode = true;
    error = '';
    smsMessage = '';
    try {
      smsMessage = await sendReauthCode();
      smsSent = true;
    } catch (e: any) {
      error = String(e);
    } finally {
      sendingCode = false;
    }
  }

  async function verifySmsCode() {
    if (verifyingCode || !dynamicCode || !reauthChallenge) return;
    verifyingCode = true;
    error = '';
    steps = [];
    try {
      const user = await withProgress(() => verifyReauthCode(dynamicCode, trustDevice));
      onLoginSuccess(user);
    } catch (e: any) {
      error = String(e);
    } finally {
      verifyingCode = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') doLogin();
  }

  function onCodeKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') verifySmsCode();
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
      <label for="username">学号</label>
      <input type="text" bind:value={username} placeholder="Student ID" disabled={loggingIn || !!reauthChallenge} onkeydown={onKeydown} />

      <label for="cas-password">CAS 密码（统一认证）</label>
      <input type="password" bind:value={casPassword} placeholder="CAS Password" disabled={loggingIn || !!reauthChallenge} onkeydown={onKeydown} />

      <label for="wisedu-password">教务密码（留空则与 CAS 相同）</label>
      <input type="password" bind:value={wiseduPassword} placeholder="Wisedu Password (optional)" disabled={loggingIn || !!reauthChallenge} onkeydown={onKeydown} />

      {#if captchaChallenge}
        <div class="challenge captcha-challenge">
          <div class="challenge-title">{captchaChallenge.message}</div>
          <div class="captcha-row">
            <button type="button" class="captcha-image-button" onclick={refreshCaptcha} aria-label="刷新验证码">
              <img src={captchaChallenge.image_base64} alt="图片验证码" />
            </button>
            <button type="button" class="link-button" onclick={refreshCaptcha}>换一张</button>
          </div>
          <input type="text" bind:value={captcha} placeholder="请输入图片验证码" autocomplete="off" disabled={loggingIn} onkeydown={onKeydown} />
        </div>
      {/if}

      {#if !reauthChallenge}
        <button onclick={doLogin} disabled={loggingIn || !username || !casPassword || (!!captchaChallenge && !captcha.trim())}>
          {#if loggingIn}
            <Spinner size={15} stroke={2} color="var(--bg-primary)" />
            <span>登录中...</span>
          {:else if captchaChallenge}
            继续登录
          {:else}
            登录
          {/if}
        </button>
      {/if}

      {#if reauthChallenge}
        <div class="challenge reauth-challenge">
          <div class="challenge-title">{reauthChallenge.message}</div>
          <p class="hint">登录平台需要确认当前设备。点击下方按钮后，验证码会发送到绑定手机。</p>
          <button type="button" class="secondary-button" onclick={requestSmsCode} disabled={sendingCode || verifyingCode}>
            {#if sendingCode}
              <Spinner size={15} stroke={2} color="var(--text-primary)" />
              <span>发送中...</span>
            {:else}
              {smsSent ? '重新发送验证码' : '获取手机验证码'}
            {/if}
          </button>
          {#if smsMessage}
            <div class="success-hint">{smsMessage}</div>
          {/if}
          <input type="text" bind:value={dynamicCode} placeholder="请输入 6 位手机验证码" inputmode="numeric" maxlength="6" autocomplete="one-time-code" disabled={!smsSent || verifyingCode} onkeydown={onCodeKeydown} />
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={trustDevice} disabled={verifyingCode} />
            验证成功后将此设备设为可信设备
          </label>
          <button type="button" onclick={verifySmsCode} disabled={verifyingCode || !smsSent || dynamicCode.length !== 6}>
            {#if verifyingCode}
              <Spinner size={15} stroke={2} color="var(--bg-primary)" />
              <span>验证并登录中...</span>
            {:else}
              验证并继续
            {/if}
          </button>
        </div>
      {/if}

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
    min-height: 100vh;
    padding: 24px;
    box-sizing: border-box;
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
    width: min(400px, 100%);
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
  .challenge {
    margin-top: 14px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-secondary);
  }
  .challenge-title {
    color: var(--text-primary);
    font-size: 13px;
    margin-bottom: 9px;
  }
  .captcha-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 52px;
  }
  .captcha-image-button {
    width: auto;
    min-width: 128px;
    height: 52px;
    padding: 0;
    margin: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    background: var(--bg-card);
  }
  .captcha-image-button img {
    display: block;
    width: 128px;
    height: 52px;
    object-fit: contain;
  }
  .link-button {
    width: auto;
    margin: 0;
    padding: 6px;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
  }
  .secondary-button {
    margin-top: 8px;
    color: var(--text-primary);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }
  .hint,
  .success-hint {
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
    margin: 0 0 8px;
  }
  .success-hint {
    color: var(--success, #22c55e);
    margin-top: 8px;
    margin-bottom: 0;
  }
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 10px;
  }
  .checkbox-label input {
    width: auto;
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
