<script lang="ts">
  import { shelf } from '../stores.svelte';
  import Icon from './Icon.svelte';
  import { folderName, kindPlural } from '../format';

  let { onquicksearch }: { onquicksearch: () => void } = $props();
  let input: HTMLInputElement | null = $state(null);
  let hintOpen = $state(false);

  let placeholder = $derived.by(() => {
    if (shelf.view === 'trash') return 'Type to search Trash…';
    if (shelf.view.startsWith('place:')) {
      return `Type to search ${folderName(shelf.view.slice(6))} — name, type, date…`;
    }
    if (shelf.view === 'downloads') return 'Type to search Downloads — name, type, date…';
    if (shelf.view === 'screenshots') return 'Type to search screenshots — name, date…';
    if (shelf.view.startsWith('kind:')) {
      return `Type to search ${kindPlural(shelf.view.slice(5) as never)} — name, date…`;
    }
    if (shelf.view === 'recent') return 'Type to search recent files — name, type, date…';
    return 'Type to search files — try “pdf last week” or “from gmail”';
  });

  const examples = [
    ['PDFs from last week', 'pdf last week'],
    ['Images over 5 MB', 'images over 5 MB'],
    ['Files from gmail', 'from gmail'],
    ['Files tagged work', 'tag:work'],
    ['Anything this year', 'this year'],
  ];

  function useExample(query: string): void {
    shelf.setText(query);
    hintOpen = false;
    input?.focus();
  }

  function onInput(event: Event): void {
    shelf.setText((event.currentTarget as HTMLInputElement).value);
  }

  function clear(): void {
    shelf.setText('');
    input?.focus();
  }

  function onKey(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      void shelf.searchNow();
    } else if (event.key === 'Escape' && shelf.text.trim()) {
      event.preventDefault();
      shelf.setText('');
    }
  }
</script>

<div class="search-dock" class:filled={Boolean(shelf.text.trim())}>
  <div class="field">
    <Icon name="search" size={16} />
    <input
      bind:this={input}
      value={shelf.text}
      oninput={onInput}
      onkeydown={onKey}
      type="search"
      placeholder={placeholder}
      aria-label={placeholder}
      title={placeholder}
      autocomplete="off"
      spellcheck="false"
    />
    {#if shelf.text.trim()}
      <button class="clear" aria-label="Clear search" onclick={clear}>
        <Icon name="close" size={12} />
      </button>
    {/if}
    <button class="help" aria-label="Search examples" aria-expanded={hintOpen} title="Search examples" onclick={() => (hintOpen = !hintOpen)}>
      ?
    </button>
    <button class="quick" aria-label="Open Quick Search" title="Quick Search (Ctrl K)" onclick={onquicksearch}>
      <kbd>Ctrl</kbd><kbd>K</kbd>
    </button>
  </div>
  {#if hintOpen}
    <div class="hints">
      <p>Search by name, type, date, size, source or tag.</p>
      {#each examples as [label, query] (query)}
        <button onclick={() => useExample(query)}>
          <span>{label}</span><code>{query}</code>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .search-dock {
    position: absolute;
    left: 50%;
    bottom: 16px;
    z-index: 35;
    width: min(560px, calc(100% - 32px));
    transform: translateX(-50%);
    transition: width 200ms var(--ease-reveal);
    animation: dock-in 180ms var(--ease-reveal);
  }

  .search-dock:focus-within {
    width: min(620px, calc(100% - 32px));
  }

  .field {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 10px 0 13px;
    border: 1px solid var(--line-hairline);
    border-radius: 12px;
    background: var(--surface-raised);
    color: var(--ink-tertiary);
    box-shadow: var(--shadow-popover);
    transition:
      border-color 150ms var(--ease-hover),
      box-shadow 150ms var(--ease-hover);
  }

  .field:focus-within,
  .search-dock.filled .field {
    border-color: color-mix(in srgb, var(--accent) 46%, var(--line-hairline));
    box-shadow:
      var(--shadow-popover),
      0 0 0 3px var(--accent-soft);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    color: var(--ink-primary);
    font-size: 14px;
    font-weight: 450;
    background: transparent;
    -webkit-appearance: none;
    appearance: none;
  }

  input:focus,
  input:focus-visible {
    outline: none;
  }

  input::placeholder {
    color: var(--ink-tertiary);
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .help {
    display: grid;
    place-items: center;
    flex: 0 0 20px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--ink-tertiary);
    font-size: 11px;
    font-weight: 600;
  }

  .help:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .clear,
  .quick {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex: 0 0 auto;
    border-radius: 6px;
    color: var(--ink-tertiary);
  }

  .clear {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
  }

  .clear:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .quick {
    padding: 2px 4px;
  }

  .quick:hover {
    background: var(--surface-hover);
  }

  @keyframes dock-in {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .search-dock {
      animation: none;
      transition: none;
    }
  }

  .hints {
    position: absolute;
    right: 0;
    bottom: calc(100% + 8px);
    width: min(330px, 100%);
    padding: 8px;
    border: 1px solid var(--line-hairline);
    border-radius: 10px;
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }

  .hints p {
    margin: 2px 6px 7px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    line-height: 15px;
  }

  .hints button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    min-height: 27px;
    padding: 0 6px;
    border-radius: 5px;
    color: var(--ink-secondary);
    font-size: 11px;
    text-align: left;
  }

  .hints button:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .hints code {
    color: var(--ink-tertiary);
    font-family: var(--font-mono);
    font-size: 9.5px;
  }

  kbd {
    font-family: var(--font-ui);
    font-size: 9.5px;
    line-height: 12px;
    padding: 1px 4px;
    border: 1px solid var(--line-hairline);
    border-radius: 4px;
    color: var(--ink-tertiary);
  }
</style>
