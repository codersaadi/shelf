<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { trapFocus } from '../focus';
  import { tick } from 'svelte';

  let input: HTMLInputElement | null = $state(null);
  let value = $state('');
  let busy = $state(false);

  $effect(() => {
    const prompt = shelf.renamePrompt;
    if (!prompt) return;
    value = prompt.name;
    void tick().then(() => {
      input?.focus();
      const dot = prompt.name.lastIndexOf('.');
      input?.setSelectionRange(0, dot > 0 ? dot : prompt.name.length);
    });
  });

  async function save(): Promise<void> {
    if (busy || !value.trim()) return;
    busy = true;
    await shelf.renameWithName(value);
    busy = false;
  }
</script>

{#if shelf.renamePrompt}
  <div class="backdrop" role="presentation" onclick={() => shelf.cancelRename()}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="rename-title"
      tabindex="-1"
      use:trapFocus
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === 'Escape') shelf.cancelRename();
        if (e.key === 'Enter') {
          e.preventDefault();
          void save();
        }
        e.stopPropagation();
      }}
    >
      <h2 id="rename-title">Rename</h2>
      <input
        bind:this={input}
        bind:value
        spellcheck="false"
        autocomplete="off"
        aria-label="New name"
        onfocus={(e) => (e.currentTarget as HTMLInputElement).select()}
      />
      <div class="actions">
        <button class="button" onclick={() => shelf.cancelRename()}>Cancel</button>
        <button class="button primary" disabled={busy || !value.trim()} onclick={save}>
          {busy ? 'Renaming…' : 'Rename'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 64;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.28);
  }

  .dialog {
    width: min(420px, 100%);
    padding: 18px;
    border: 1px solid var(--line-hairline);
    border-radius: 12px;
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }

  h2 {
    margin: 0 0 10px;
    font-size: 15px;
    font-weight: 600;
  }

  input {
    width: 100%;
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-content);
    color: var(--ink-primary);
    font-size: 13px;
  }

  input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 7px;
    margin-top: 14px;
  }
</style>
