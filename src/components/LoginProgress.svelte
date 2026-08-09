<script lang="ts">
  import { fly } from 'svelte/transition';
  import type { LoginStep } from '../lib/types';
  import Spinner from './Spinner.svelte';

  let { steps }: { steps: LoginStep[] } = $props();
</script>

<div class="progress">
  {#each steps as s, i (s.step)}
    {@const done = i < steps.length - 1}
    <div class="step" in:fly={{ x: -8, duration: 200 }}>
      <span class="step-num" class:done>
        {#if done}
          ✓
        {:else}
          <Spinner size={12} stroke={2} color="var(--bg-primary)" />
        {/if}
      </span>
      <span class="step-msg" class:current={!done}>{s.message}</span>
    </div>
  {/each}
</div>

<style>
  .progress {
    margin-top: 20px;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: var(--radius);
  }
  .step {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 0;
    font-size: 13px;
  }
  .step-num {
    background: var(--accent);
    color: var(--bg-primary);
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .step-num.done {
    background: var(--success);
  }
  .step-msg {
    color: var(--text-secondary);
  }
  .step-msg.current {
    color: var(--text-primary);
  }
</style>
