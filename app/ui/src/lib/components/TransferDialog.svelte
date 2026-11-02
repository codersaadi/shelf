<script lang="ts">
  import { onMount } from 'svelte';
  import { ipc } from '../ipc';
  import { shelf } from '../stores.svelte';
  import { trapFocus } from '../focus';

  let input: HTMLInputElement | null = $state(null);
  let home = $state('');
  let dest = $state('');
  let busy = $state(false);

  const prompt = $derived(shelf.transferPrompt);
  const mode = $derived(prompt?.mode ?? 'move');
  const paths = $derived(prompt?.paths ?? []);

  onMount(() => {
    void ipc.settingsInfo().then((info) => {
      home = info.home;
      if (!dest) {
        dest = localStorage.getItem('shelf.transfer.dest') ?? `${info.home}/Downloads`;
      }
      setTimeout(() => input?.focus(), 0);
    });
  });

  async function submit(): Promise<void> {
    if (busy || !dest.trim()) return;
    busy = true;
    const ok = await shelf.transfer(paths, dest, mode);
    busy = false;
    if (ok) {
      try {
        localStorage.setItem('shelf.transfer.dest', dest.trim());
      } catch {
        // Storage is optional.
      }
    }
  }

  function quick(path: string): void {
    dest = path;
    input?.focus();
  }
</script>

{#if prompt}
  <div class="backdrop" role="presentation" onclick={() => shelf.cancelTransfer()}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="transfer-title"
      tabindex="-1"
      use:trapFocus
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === 'Escape') shelf.cancelTransfer();
        e.stopPropagation();
      }}
    >
      <header>
        <h2 id="transfer-title">{mode === 'move' ? 'Move' : 'Copy'} {paths.length} {paths.length === 1 ? 'file' : 'files'}</h2>
        <p>Choose a destination folder. Existing files are never overwritten.</p>
      </header>

      <label class="field">
        <span>Destination</span>
        <input
          bind:this={input}
          bind:value={dest}
          spellcheck="false"
          autocomplete="off"
          placeholder="/home/you/Downloads"
          onkeydown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              void submit();
            }
          }}
        />
      </label>

      {#if home}
        <div class="quick" aria-label="Common destinations">
          <button onclick={() => quick(`${home}/Downloads`)}>Downloads</button>
          <button onclick={() => quick(`${home}/Desktop`)}>Desktop</button>
          <button onclick={() => quick(`${home}/Documents`)}>Documents</button>
          <button onclick={() => quick(`${home}/Pictures`)}>Pictures</button>
          <button onclick={() => quick(`${home}/Videos`)}>Videos</button>
          <button onclick={() => quick(home)}>Home</button>
        </div>
      {/if}

      <footer>
        <button class="button" onclick={() => shelf.cancelTransfer()}>Cancel</button>
        <button class="button primary" disabled={busy || !dest.trim()} onclick={submit}>
          {busy ? `${mode === 'move' ? 'Moving' : 'Copying'}…` : mode === 'move' ? 'Move here' : 'Copy here'}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.28);
  }

  .dialog {
    width: min(520px, 100%);
    padding: 20px;
    border: var(--hairline);
    border-radius: 12px;
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }

  header h2 {
    margin: 0;
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
  }

  header p {
    margin: 4px 0 16px;
    color: var(--ink-secondary);
    font-size: 11.5px;
    line-height: 16px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field span {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  input {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--line-hairline);
    border-radius: 8px;
    background: var(--surface-content);
    color: var(--ink-primary);
    font-family: var(--font-mono);
    font-size: 11.5px;
  }

  input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 10px;
  }

  .quick button {
    padding: 3px 7px;
    border-radius: 5px;
    background: var(--surface-hover);
    color: var(--ink-secondary);
    font-size: 10.5px;
  }

  .quick button:hover {
    color: var(--ink-primary);
    background: var(--surface-select);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
</style>
