<script lang="ts">
  import { onMount } from 'svelte';
  import { trapFocus } from '../focus';
  import { ipc, thumbUrl, type FileMeta } from '../ipc';
  import { middleTruncate, parentPath } from '../format';
  import { shelf } from '../stores.svelte';
  import Icon from './Icon.svelte';
  import { smartName } from '../smart';

  let { onclose }: { onclose: () => void } = $props();
  let query = $state('');
  let results = $state<FileMeta[]>([]);
  let active = $state(0);
  let input: HTMLInputElement | null = $state(null);
  let timer: ReturnType<typeof setTimeout> | null = null;
  let error = $state<string | null>(null);
  let loadSeq = 0;

  $effect(() => {
    input?.focus();
    void load('');
  });

  onMount(() => {
    const dismiss = () => onclose();
    window.addEventListener('blur', dismiss);
    return () => window.removeEventListener('blur', dismiss);
  });

  async function load(text: string): Promise<void> {
    const seq = ++loadSeq;
    error = null;
    try {
      if (text.trim()) {
        const page = await ipc.search(text, null, 'frecency', 8, null);
        if (seq !== loadSeq) return;
        results = page.items;
      } else {
        const page = await ipc.listShelf({ view: 'recent', sort: 'added', limit: 8 });
        if (seq !== loadSeq) return;
        results = page.items;
      }
      active = 0;
    } catch (e) {
      if (seq !== loadSeq) return;
      results = [];
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function onInput(event: Event): void {
    query = (event.currentTarget as HTMLInputElement).value;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void load(query), 90);
  }

  async function openResult(file: FileMeta): Promise<void> {
    onclose();
    await shelf.openFile(file);
  }

  function seeAll(): void {
    onclose();
    if (query.trim()) shelf.setText(query.trim());
    else void shelf.setView('home');
  }

  function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      onclose();
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      active = Math.min(results.length - 1, active + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      active = Math.max(0, active - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const file = results[active];
      if (!file) return;
      if (event.ctrlKey || event.metaKey) {
        onclose();
        void shelf.showInFiles([file.path]);
      } else {
        void openResult(file);
      }
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
    aria-label="Quick Search"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="field">
      <Icon name="search" size={18} />
      <input
        bind:this={input}
        value={query}
        oninput={onInput}
        onkeydown={onKey}
        placeholder="Search files…"
        aria-label="Quick search"
        autocomplete="off"
        spellcheck="false"
      />
      <kbd>Esc</kbd>
    </div>
    <ul class="results">
      {#each results as file, i (file.path)}
        <li>
          <button
            class="row"
            class:active={i === active}
            onclick={() => openResult(file)}
            onmouseenter={() => (active = i)}
          >
            <span class="mini">
              {#if file.kind === 'image'}
                <img src={thumbUrl(file, 64)} alt="" loading="lazy" />
              {:else}
                <Icon name={file.kind === 'video' ? 'video' : file.kind === 'music' ? 'music' : file.kind === 'archive' ? 'archive' : 'document'} size={15} />
              {/if}
            </span>
            <span class="text">
              <span class="name">{middleTruncate(smartName(file).title, 48)}{#if smartName(file).detail}<span class="real-name"> · {smartName(file).detail}</span>{/if}</span>
              <span class="path">{parentPath(file.path)}</span>
            </span>
            {#if file.origin_host}<span class="from">{file.origin_host}</span>{/if}
          </button>
        </li>
      {:else}
        <li class="none">{error ?? 'No files match.'}</li>
      {/each}
    </ul>
    <footer>
      <span><kbd>Enter</kbd> open</span>
      <span><kbd>Ctrl</kbd><kbd>Enter</kbd> show in Files</span>
      <button class="see-all" onclick={seeAll}>See all in Shelf</button>
    </footer>
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
    padding-top: 22vh;
    background: color-mix(in srgb, #000 25%, transparent);
  }

  .panel {
    width: min(680px, calc(100vw - 48px));
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
    border: 1px solid var(--line-hairline);
    overflow: hidden;
    animation: reveal 180ms var(--ease-reveal);
  }

  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: var(--hairline);
    color: var(--ink-tertiary);
  }

  .field input {
    flex: 1;
    font-size: var(--text-quick);
    line-height: 28px;
    font-weight: 400;
    letter-spacing: -0.015em;
    color: var(--ink-primary);
  }

  .field input::placeholder {
    color: var(--ink-tertiary);
  }

  kbd {
    font-family: var(--font-ui);
    font-size: 10px;
    line-height: 12px;
    padding: 1px 4px;
    border-radius: 4px;
    border: 1px solid var(--line-hairline);
    color: var(--ink-tertiary);
  }

  .results {
    list-style: none;
    margin: 0;
    padding: 5px;
    max-height: 360px;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 44px;
    padding: 0 10px;
    border-radius: 8px;
    text-align: left;
  }

  .row.active {
    background: var(--accent-soft);
  }

  .mini {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 5px;
    overflow: hidden;
    background: var(--surface-hover);
    color: var(--ink-tertiary);
    flex: 0 0 28px;
  }

  .mini img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }

  .real-name {
    color: var(--ink-tertiary);
    font-family: var(--font-mono);
    font-size: 9px;
  }

  .name {
    font-size: 12px;
    line-height: 15px;
    color: var(--ink-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .path {
    font-size: 10.5px;
    line-height: 13px;
    color: var(--ink-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 420px;
  }

  .from {
    margin-left: auto;
    font-size: 10.5px;
    color: var(--ink-tertiary);
  }

  .none {
    padding: 20px 12px;
    color: var(--ink-tertiary);
    font-size: 12px;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 14px;
    border-top: var(--hairline);
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  footer span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .see-all {
    margin-left: auto;
    color: var(--accent);
  }

  @keyframes reveal {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
  }
</style>
