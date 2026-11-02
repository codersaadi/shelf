<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { fade } from 'svelte/transition';
  import Icon from './Icon.svelte';
</script>

{#if shelf.toast}
  <div class="toast" role="status" aria-live="polite" transition:fade={{ duration: 90 }}>
    <span class="message">{shelf.toast.message}</span>
    {#if shelf.toast.undo}
      <button class="undo" onclick={() => shelf.undo(shelf.toast?.undo ?? 0)}>
        <Icon name="undo" size={13} />Undo
      </button>
    {/if}
    <button class="close" aria-label="Dismiss" onclick={() => shelf.dismissToast()}>
      <Icon name="close" size={12} />
    </button>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    z-index: 60;
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 40px;
    max-width: min(560px, calc(100vw - 48px));
    padding: 8px 10px 8px 16px;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    border: 1px solid var(--line-hairline);
    box-shadow: var(--shadow-popover);
  }

  .message {
    font-size: 12px;
    line-height: 17px;
    color: var(--ink-primary);
  }

  .undo {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    padding: 3px 8px;
    border-radius: 6px;
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
  }

  .undo:hover {
    background: var(--accent-soft);
  }

  .close {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    color: var(--ink-tertiary);
  }

  .close:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }
</style>
