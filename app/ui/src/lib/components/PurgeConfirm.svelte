<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { trapFocus } from '../focus';
  import Icon from './Icon.svelte';

  const paths = $derived(shelf.purgePrompt ?? []);
  let busy = $state(false);

  async function confirm(): Promise<void> {
    if (busy) return;
    busy = true;
    await shelf.purgeTrash(paths);
    busy = false;
  }
</script>

{#if shelf.purgePrompt}
  <div class="backdrop" role="presentation" onclick={() => shelf.cancelPurge()}>
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="purge-title"
      tabindex="-1"
      use:trapFocus
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === 'Escape') shelf.cancelPurge();
        if (e.key === 'Enter') {
          e.preventDefault();
          void confirm();
        }
        e.stopPropagation();
      }}
    >
      <div class="icon"><Icon name="trash" size={18} /></div>
      <h2 id="purge-title">Delete permanently?</h2>
      <p>
        {paths.length} {paths.length === 1 ? 'item' : 'items'} will be removed from Trash. This
        cannot be undone.
      </p>
      <div class="actions">
        <button class="button" onclick={() => shelf.cancelPurge()}>Cancel</button>
        <button class="button danger" disabled={busy} onclick={confirm}>
          {busy ? 'Deleting…' : 'Delete permanently'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 62;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.3);
  }

  .dialog {
    width: min(400px, 100%);
    padding: 20px;
    border: 1px solid var(--line-hairline);
    border-radius: 12px;
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }

  .icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    margin-bottom: 12px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  p {
    margin: 6px 0 18px;
    color: var(--ink-secondary);
    font-size: 12px;
    line-height: 17px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
