<script lang="ts">
  import { trapFocus } from '../focus';
  import { shelf } from '../stores.svelte';
  import type { FileKind } from '../ipc';
  import Icon from './Icon.svelte';

  let { onclose }: { onclose: () => void } = $props();
  let query = $state('');
  let active = $state(0);
  let input: HTMLInputElement | null = $state(null);

  const kinds: FileKind[] = ['image', 'video', 'music', 'document', 'archive', 'app'];

  interface Command {
    id: string;
    label: string;
    hint?: string;
    run: () => void;
  }

  const commands: Command[] = [
    { id: 'home', label: 'Go to Home', run: () => void shelf.setView('home') },
    { id: 'downloads', label: 'Go to Downloads', run: () => void shelf.setView('downloads') },
    { id: 'screens', label: 'Go to Screenshots', run: () => void shelf.setView('screenshots') },
    { id: 'recent', label: 'Go to Recent', run: () => void shelf.setView('recent') },
    ...kinds.map((kind) => ({
      id: `kind-${kind}`,
      label: `Go to ${kind.charAt(0).toUpperCase()}${kind.slice(1)}s`,
      run: () => void shelf.setView(`kind:${kind}`),
    })),
    { id: 'cleanup', label: 'Open Clean Up', run: () => void shelf.setView('cleanup') },
    { id: 'trash', label: 'Open Trash', run: () => void shelf.setView('trash') },
    { id: 'grid', label: 'Switch to Grid view', run: () => shelf.setViewMode('grid') },
    { id: 'list', label: 'Switch to List view', run: () => shelf.setViewMode('list') },
    {
      id: 'inspector',
      label: 'Toggle Inspector',
      hint: 'Ctrl I',
      run: () => shelf.toggleInspector(),
    },
    {
      id: 'sidebar',
      label: 'Toggle Sidebar',
      hint: 'Ctrl B',
      run: () => (shelf.sidebarOpen = !shelf.sidebarOpen),
    },
    { id: 'reindex', label: 'Reindex files', run: () => void shelf.reindex() },
    ...(shelf.text.trim()
      ? [
          {
            id: 'save-search',
            label: 'Save current search…',
            run: () => {
              const name = window.prompt('Name this search', shelf.text.trim());
              if (name) void shelf.saveCurrentSearch(name);
            },
          },
        ]
      : []),
  ];

  let filtered = $derived(
    query.trim()
      ? commands.filter((c) => c.label.toLowerCase().includes(query.trim().toLowerCase()))
      : commands,
  );

  $effect(() => {
    input?.focus();
  });

  function run(command: Command | undefined): void {
    if (!command) return;
    onclose();
    command.run();
  }

  function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      onclose();
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      active = Math.min(filtered.length - 1, active + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      active = Math.max(0, active - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      run(filtered[active]);
    }
  }
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={onclose}
  onkeydown={(e) => e.key === 'Escape' && onclose()}
>
  <div
    class="panel"
    role="dialog"
    tabindex="-1"
    use:trapFocus
    aria-modal="true"
    aria-label="Command list"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="field">
      <Icon name="search" size={15} />
      <input
        bind:this={input}
        bind:value={query}
        oninput={() => (active = 0)}
        onkeydown={onKey}
        placeholder="Run a command…"
        aria-label="Command"
        autocomplete="off"
      />
      <kbd>Esc</kbd>
    </div>
    <ul class="list">
      {#each filtered as command, i (command.id)}
        <li>
          <button
            class="row"
            class:active={i === active}
            onclick={() => run(command)}
            onmouseenter={() => (active = i)}
          >
            <span>{command.label}</span>
            {#if command.hint}<kbd>{command.hint}</kbd>{/if}
          </button>
        </li>
      {:else}
        <li class="none">No matching command.</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 55;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 18vh;
    background: color-mix(in srgb, #000 25%, transparent);
  }

  .panel {
    width: min(560px, calc(100vw - 48px));
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    border: 1px solid var(--line-hairline);
    box-shadow: var(--shadow-popover);
    overflow: hidden;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    border-bottom: var(--hairline);
    color: var(--ink-tertiary);
  }

  .field input {
    flex: 1;
    font-size: 14px;
    line-height: 20px;
    color: var(--ink-primary);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 5px;
    max-height: 380px;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border-radius: 7px;
    font-size: 12px;
    text-align: left;
  }

  .row span {
    flex: 1;
  }

  .row.active {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .none {
    padding: 16px 12px;
    color: var(--ink-tertiary);
    font-size: 12px;
  }

  kbd {
    font-family: var(--font-ui);
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 4px;
    border: 1px solid var(--line-hairline);
    color: var(--ink-tertiary);
  }
</style>
