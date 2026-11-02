<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { bytes, count as fmtCount, kindPlural, middleTruncate, plural, relativeDate } from '../format';
  import type { FileMeta } from '../ipc';
  import Icon from './Icon.svelte';
  import Thumb from './Thumb.svelte';
  import { dragScroll } from '../dragScroll';
  import { ipc } from '../ipc';
  import { startFileDrag } from '../fileDrag';
  import { smartName } from '../smart';

  let { oncontext }: { oncontext?: (file: FileMeta, event: MouseEvent) => void } = $props();

  let anchor = $state<string | null>(null);
  let setupPath = $state('');
  let setupBusy = $state(false);

  async function indexFolder(): Promise<void> {
    const path = setupPath.trim();
    if (!path || setupBusy) return;
    if (!(await ipc.pathIsDir(path).catch(() => false))) {
      shelf.showToast('That folder does not exist or is not readable.');
      return;
    }
    setupBusy = true;
    try {
      await ipc.addIndexRoot(path);
      await ipc.indexRoot(path);
      shelf.showToast('Folder added and indexing in the background.');
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    } finally {
      setupBusy = false;
    }
  }

  function context(file: FileMeta, event: MouseEvent): void {
    event.preventDefault();
    if (!shelf.selection.includes(file.path)) shelf.select([file.path]);
    oncontext?.(file, event);
  }

  function selectCard(file: FileMeta, event: MouseEvent, list: FileMeta[]): void {
    const additive = event.ctrlKey || event.metaKey;
    if (event.shiftKey && anchor) {
      const a = list.findIndex((f) => f.path === anchor);
      const b = list.findIndex((f) => f.path === file.path);
      if (a >= 0 && b >= 0) {
        const range = list.slice(Math.min(a, b), Math.max(a, b) + 1).map((f) => f.path);
        shelf.select(additive ? [...new Set([...shelf.selection, ...range])] : range);
        return;
      }
    }
    if (additive) {
      const next = new Set(shelf.selection);
      if (next.has(file.path)) next.delete(file.path);
      else next.add(file.path);
      shelf.select([...next]);
      anchor = file.path;
      return;
    }
    shelf.select([file.path]);
    anchor = file.path;
  }

  function open(file: FileMeta): void {
    void shelf.openFile(file);
  }

  function openShelf(kind: string): void {
    void shelf.setView(kind === 'skill' ? 'skills' : `kind:${kind}`);
  }
</script>

<div class="home">
  {#if shelf.home && shelf.home.recent.length}
    <section class="rail-section" aria-labelledby="just-added">
      <div class="section-head">
        <h2 id="just-added">Just added</h2>
        <span class="section-meta">{shelf.home.recent.length} newest</span>
      </div>
      <div class="rail" use:dragScroll aria-label="Just added">
        {#each shelf.home.recent as file (file.path)}
          <button
            class="rail-card"
            class:selected={shelf.selection.includes(file.path)}
            aria-pressed={shelf.selection.includes(file.path)}
            onclick={(e) => selectCard(file, e, shelf.home?.recent ?? [])}
            ondblclick={() => open(file)}
            oncontextmenu={(e) => context(file, e)}
            title={file.path}
          >
            <span
              class="rail-thumb"
              draggable="true"
              role="button"
              tabindex="-1"
              aria-label={`Drag ${file.name} to another app`}
              title="Drag to another app"
              onpointerdown={(e) => e.stopPropagation()}
              ondragstart={(e) => {
                e.stopPropagation();
                startFileDrag(
                  e,
                  shelf.selection.includes(file.path) ? shelf.selection : [file.path],
                );
              }}
            ><Thumb {file} size={160} /></span>
            <span class="name">{middleTruncate(smartName(file).title, 26)}</span>
            {#if smartName(file).detail}<span class="real-name">{smartName(file).detail}</span>{/if}
            <span class="meta">{relativeDate(file.added_at)}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#each shelf.home?.shelves ?? [] as group (group.kind)}
    <section class="shelf-section" aria-labelledby={`shelf-${group.kind}`}>
      <div class="section-head">
        <button class="shelf-title" onclick={() => openShelf(group.kind)}>
          <h2 id={`shelf-${group.kind}`}>{kindPlural(group.kind)}</h2>
          <span class="section-meta">
            {fmtCount(group.count)} {plural(group.count, 'file')} · {bytes(group.bytes)}
          </span>
        </button>
        <button class="see-all" onclick={() => openShelf(group.kind)}>
          See all<Icon name="chevron-right" size={13} />
        </button>
      </div>
      <div class="shelf-row" use:dragScroll aria-label="{kindPlural(group.kind)} preview">
        {#each group.files as file (file.path)}
          <button
            class="shelf-card"
            class:selected={shelf.selection.includes(file.path)}
            aria-pressed={shelf.selection.includes(file.path)}
            onclick={(e) => selectCard(file, e, group.files)}
            ondblclick={() => open(file)}
            oncontextmenu={(e) => context(file, e)}
            title={file.path}
          >
            <span
              class="shelf-thumb"
              draggable="true"
              role="button"
              tabindex="-1"
              aria-label={`Drag ${file.name} to another app`}
              title="Drag to another app"
              onpointerdown={(e) => e.stopPropagation()}
              ondragstart={(e) => {
                e.stopPropagation();
                startFileDrag(
                  e,
                  shelf.selection.includes(file.path) ? shelf.selection : [file.path],
                );
              }}
            ><Thumb {file} size={128} /></span>
            <span class="name">{middleTruncate(smartName(file).title, 22)}</span>
            {#if smartName(file).detail}<span class="real-name">{smartName(file).detail}</span>{/if}
          </button>
        {/each}
      </div>
    </section>
  {/each}

  {#if !shelf.loading && (shelf.home?.shelves.length ?? 0) === 0 && shelf.counts.total === 0}
    <div class="blank">
      <Icon name="folder" size={28} />
      <h2>{shelf.indexing ? 'Indexing your files' : 'Welcome to Shelf'}</h2>
      <p>
        {shelf.indexing
          ? 'Indexing in the background. Search results will fill in as it goes.'
          : 'Shelf finds files across the folders you choose. Everything stays on this computer.'}
      </p>
      <div class="welcome-actions">
        <button class="button primary" onclick={() => shelf.reindex()}>Index my folders</button>
      </div>
      <form
        class="choose-folder"
        onsubmit={(e) => {
          e.preventDefault();
          void indexFolder();
        }}
      >
        <input
          bind:value={setupPath}
          placeholder="or index another folder, e.g. /home/you/work"
          spellcheck="false"
          autocomplete="off"
          aria-label="Folder to index"
        />
        <button class="button" type="submit" disabled={!setupPath.trim() || setupBusy}>
          {setupBusy ? 'Starting…' : 'Index'}
        </button>
      </form>
      <p class="privacy-note">Local only. No account, no upload.</p>
    </div>
  {/if}
</div>

<style>
  .home {
    height: 100%;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .section-head {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 12px;
  }

  .section-head h2 {
    margin: 0;
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .section-meta {
    font-size: 11.5px;
    line-height: 14px;
    color: var(--ink-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .shelf-title {
    display: flex;
    align-items: baseline;
    gap: 10px;
    text-align: left;
  }

  .see-all {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    font-size: 12px;
    color: var(--accent);
    border-radius: 6px;
    padding: 2px 4px;
  }

  .see-all:hover {
    background: var(--accent-soft);
  }

  .rail {
    display: flex;
    gap: 12px;
    overflow-x: auto;
    touch-action: pan-y;
    overscroll-behavior-x: contain;
    cursor: grab;
    user-select: none;
    scrollbar-width: none;
    padding-bottom: 6px;
  }

  .rail-card,
  .shelf-card {
    position: relative;
  }

  .rail-card {
    flex: 0 0 160px;
    width: 160px;
    text-align: left;
  }

  .rail-thumb {
    display: block;
    cursor: grab;
    width: 160px;
    height: 160px;
    border-radius: var(--radius-thumb);
    overflow: hidden;
    background: transparent;
  }

  .rail-card.selected .rail-thumb,
  .shelf-card.selected .shelf-thumb {
    box-shadow: inset 0 0 0 2px var(--accent);
  }

  .rail-thumb,
  .shelf-thumb {
    position: relative;
  }

  .rail-card .name {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    line-height: 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rail-card .meta {
    display: block;
    margin-top: 1px;
    font-size: 11px;
    line-height: 13px;
    color: var(--ink-tertiary);
  }

  .shelf-row {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 128px;
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 4px;
    touch-action: pan-y;
    overscroll-behavior-x: contain;
    cursor: grab;
    user-select: none;
    scrollbar-width: none;
  }

  .rail:global(.dragging),
  .shelf-row:global(.dragging) {
    cursor: grabbing;
  }

  .rail::-webkit-scrollbar,
  .shelf-row::-webkit-scrollbar {
    display: none;
  }

  .rail:global(.dragging) .rail-card,
  .shelf-row:global(.dragging) .shelf-card {
    pointer-events: none;
  }

  .shelf-card {
    width: 128px;
    text-align: left;
  }

  .shelf-thumb {
    display: block;
    cursor: grab;
    width: 128px;
    height: 128px;
    border-radius: var(--radius-thumb);
    overflow: hidden;
    background: transparent;
  }

  .shelf-card .name {
    display: block;
    margin-top: 6px;
    font-size: 11.5px;
    line-height: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .welcome-actions {
    margin-top: 4px;
  }

  .choose-folder {
    display: flex;
    gap: 6px;
    width: min(440px, 100%);
    margin-top: 2px;
  }

  .choose-folder input {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 9px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-content);
    color: var(--ink-primary);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .choose-folder input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .privacy-note {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .blank {
    display: grid;
    justify-items: center;
    gap: 8px;
    margin: auto;
    padding: 48px 24px;
    text-align: center;
    color: var(--ink-secondary);
  }

  .blank h2 {
    margin: 8px 0 0;
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
    color: var(--ink-primary);
  }

  .blank p {
    margin: 0 0 8px;
    max-width: 42ch;
    font-size: 12px;
    line-height: 17px;
  }

  @media (max-width: 839px) {
    .home {
      padding: 16px;
      gap: 24px;
    }
  }
</style>
