<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { bytes, kindPlural, kindIcon, count as fmtCount, plural } from '../format';
  import { ipc, type FileKind, type KnownLocation } from '../ipc';
  import Icon from './Icon.svelte';
  import { trapFocus } from '../focus';
  import { tick } from 'svelte';

  let { overlay = false, onclose }: { overlay?: boolean; onclose?: () => void } = $props();

  const kinds: FileKind[] = ['image', 'video', 'music', 'document', 'archive', 'app'];
  let activeView = $derived(
    shelf.text.trim() && !shelf.view.startsWith('saved:') ? '' : shelf.view,
  );
  let adding = $state(false);
  let newPath = $state('');
  let addError = $state<string | null>(null);
  let pathInput: HTMLInputElement | null = $state(null);
  let suggestions = $state<KnownLocation[]>([]);
  let picked = $state<string[]>([]);
  let loadingSuggestions = $state(false);

  async function openAdd(): Promise<void> {
    adding = true;
    addError = null;
    picked = [];
    newPath = '';
    loadingSuggestions = true;
    try {
      const known = await ipc.knownLocations();
      const added = new Set(shelf.locations.map((l) => l.path));
      suggestions = known.filter((l) => !added.has(l.path));
    } catch {
      suggestions = [];
    } finally {
      loadingSuggestions = false;
    }
    await tick();
    if (suggestions.length === 0) pathInput?.focus();
  }

  function togglePick(path: string): void {
    picked = picked.includes(path) ? picked.filter((p) => p !== path) : [...picked, path];
  }

  function selectAll(): void {
    picked = suggestions.map((l) => l.path);
  }

  async function submitLocation(): Promise<void> {
    const custom = newPath.trim();
    if (custom && !(await ipc.pathIsDir(custom).catch(() => false))) {
      addError = 'That folder does not exist or is not readable.';
      return;
    }
    const paths = [...picked];
    if (custom && !paths.includes(custom)) paths.push(custom);
    if (paths.length === 0) return;
    let added = 0;
    for (const path of paths) {
      if (shelf.addLocation(path)) {
        added += 1;
        void ipc.addIndexRoot(path).catch(() => {});
        void ipc.indexRoot(path).catch(() => {});
      }
    }
    if (added === 0) {
      addError = 'Those locations are already in the sidebar.';
      return;
    }
    shelf.showToast(`Added ${added} ${added === 1 ? 'location' : 'locations'}; indexing…`);
    adding = false;
    newPath = '';
    picked = [];
  }
</script>

<aside class="sidebar" class:overlay aria-label="Shelf">
  <div class="brand">
    <span class="wordmark">Shelf</span>
    {#if overlay}
      <button class="icon-button close" onclick={onclose} aria-label="Close sidebar">
        <Icon name="close" size={14} />
      </button>
    {/if}
  </div>

  <nav class="nav">
    <div class="group">
      <button class="row" class:active={activeView === 'home'} onclick={() => shelf.setView('home')}>
        <Icon name="home" size={18} />
        <span class="label">Home</span>
      </button>
      <button
        class="row"
        class:active={activeView === 'recent'}
        onclick={() => shelf.setView('recent')}
      >
        <Icon name="recent" size={18} />
        <span class="label">Recent</span>
      </button>
      <button
        class="row"
        class:active={activeView === 'downloads'}
        onclick={() => shelf.setView('downloads')}
      >
        <Icon name="download" size={18} />
        <span class="label">Downloads</span>
      </button>
      <button
        class="row"
        class:active={activeView === 'screenshots'}
        onclick={() => shelf.setView('screenshots')}
      >
        <Icon name="screenshot" size={18} />
        <span class="label">Screenshots</span>
      </button>
    </div>

    <div class="group">
      {#each kinds as kind (kind)}
        <button
          class="row"
          class:active={activeView === `kind:${kind}`}
          onclick={() => shelf.setView(`kind:${kind}`)}
        >
          <Icon name={kindIcon(kind)} size={18} />
          <span class="label">{kindPlural(kind)}</span>
        </button>
      {/each}
    </div>

    {#if shelf.saved.length > 0}
      <div class="group">
        {#each shelf.saved as saved (saved.id)}
          <div class="saved-row">
            <button
              class="row"
              class:active={activeView === `saved:${saved.id}`}
              onclick={() => shelf.openSaved(saved)}
              title={saved.query}
            >
              <Icon name="search" size={18} />
              <span class="label">{saved.name}</span>
            </button>
            <button
              class="row-action"
              aria-label={`Delete saved search ${saved.name}`}
              onclick={() => shelf.deleteSaved(saved.id)}
            >
              <Icon name="close" size={12} />
            </button>
          </div>
        {/each}
      </div>
    {/if}

    <div class="group locations">
      <div class="group-label">
        <span>Locations</span>
        <button class="add-location" aria-label="Add location" onclick={openAdd}>
          <Icon name="plus" size={12} />
        </button>
      </div>
      {#each shelf.locations as location (location.path)}
        <div class="saved-row">
          <button
            class="row"
            class:active={activeView === `place:${location.path}` ||
              activeView.startsWith(`place:${location.path}/`)}
            title={location.path}
            onclick={() => shelf.openLocation(location.path)}
          >
            <Icon name="folder" size={18} />
            <span class="label">{location.name}</span>
          </button>
          <button
            class="row-action"
            aria-label={`Remove location ${location.name}`}
            onclick={() => shelf.removeLocation(location.path)}
          >
            <Icon name="close" size={12} />
          </button>
        </div>
      {/each}
    </div>

    <div class="group bottom">
      <button
        class="row"
        class:active={activeView === 'cleanup'}
        onclick={() => shelf.setView('cleanup')}
      >
        <Icon name="clean" size={18} />
        <span class="label">Clean Up</span>
        {#if shelf.counts.reclaimable > 1_000_000_000}
          <span class="badge">{bytes(shelf.counts.reclaimable)}</span>
        {/if}
      </button>
      <button class="row" class:active={activeView === 'trash'} onclick={() => shelf.setView('trash')}>
        <Icon name="trash" size={18} />
        <span class="label">Trash</span>
        {#if shelf.counts.trash > 0}
          <span class="badge">{fmtCount(shelf.counts.trash)}</span>
        {/if}
      </button>
      <button
        class="row"
        class:active={activeView === 'settings'}
        onclick={() => shelf.setView('settings')}
      >
        <Icon name="settings" size={18} />
        <span class="label">Settings</span>
      </button>
    </div>
  </nav>

  <div class="footer">
    {#if shelf.indexing}
      <p class="status"><span class="dot" aria-hidden="true"></span>{shelf.indexing}</p>
    {:else}
      <p class="status quiet">
        {fmtCount(shelf.counts.total)} {plural(shelf.counts.total, 'file')} indexed
      </p>
    {/if}
  </div>
</aside>

{#if adding}
  <div class="modal-backdrop" role="presentation" onclick={() => (adding = false)}>
    <div
      class="location-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="location-title"
      tabindex="-1"
      use:trapFocus
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === 'Escape') adding = false;
        if (e.key === 'Enter' && (e.target as HTMLElement).tagName === 'INPUT') {
          e.preventDefault();
          void submitLocation();
        }
        e.stopPropagation();
      }}
    >
      <h2 id="location-title">Add locations</h2>
      <p>Choose one or more folders to show in the sidebar. You can come back and add more any time.</p>
      {#if suggestions.length}
        <div class="quick-tools">
          <button onclick={selectAll}>Select all</button>
          <button onclick={() => (picked = [])}>Clear</button>
        </div>
      {/if}
      <div class="quick-list" aria-label="Suggested folders">
        {#if loadingSuggestions}
          <p class="quick-empty">Finding your folders…</p>
        {:else if suggestions.length === 0}
          <p class="quick-empty">All common folders are already added. Enter another path below.</p>
        {:else}
          {#each suggestions as location (location.path)}
            <button
              class="quick-row"
              class:picked={picked.includes(location.path)}
              aria-pressed={picked.includes(location.path)}
              onclick={() => togglePick(location.path)}
            >
              <span class="check" aria-hidden="true">
                {#if picked.includes(location.path)}<Icon name="check" size={11} />{/if}
              </span>
              <Icon name="folder" size={16} />
              <span class="quick-copy">
                <span class="quick-name">{location.name}</span>
                <span class="quick-path">{location.path}</span>
              </span>
            </button>
          {/each}
        {/if}
      </div>
      <div class="or"><span>or add a path</span></div>
      <input
        bind:this={pathInput}
        bind:value={newPath}
        placeholder="/home/you/work"
        spellcheck="false"
        autocomplete="off"
      />
      {#if addError}<p class="form-error">{addError}</p>{/if}
      <div class="dialog-actions">
        <button class="button" onclick={() => (adding = false)}>Cancel</button>
        <button
          class="button primary"
          disabled={(picked.length === 0 && !newPath.trim()) || loadingSuggestions}
          onclick={submitLocation}
        >
          {picked.length + (newPath.trim() ? 1 : 0) > 1
            ? `Add ${picked.length + (newPath.trim() ? 1 : 0)} locations`
            : 'Add location'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .sidebar {
    width: 216px;
    min-width: 216px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface-sidebar);
    border-right: var(--hairline);
    padding: 10px 10px 8px;
    user-select: none;
  }

  .sidebar.overlay {
    position: absolute;
    z-index: 40;
    top: 0;
    bottom: 0;
    left: 0;
    box-shadow: var(--shadow-popover);
    background: var(--surface-window);
  }

  .brand {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 10px;
    margin-bottom: 8px;
  }

  .wordmark {
    font-size: 16px;
    line-height: 21px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
    flex: 1;
    padding-bottom: 8px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .group-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
    padding: 0 10px;
    color: var(--ink-tertiary);
    font-size: 10px;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .add-location {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--ink-tertiary);
  }

  .add-location:hover {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.3);
  }

  .location-dialog {
    width: min(440px, 100%);
    padding: 18px;
    border: 1px solid var(--line-hairline);
    border-radius: 12px;
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }

  .location-dialog h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .location-dialog p {
    margin: 4px 0 12px;
    color: var(--ink-secondary);
    font-size: 11.5px;
    line-height: 16px;
  }

  .quick-tools {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-bottom: 6px;
  }

  .quick-tools button {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .quick-tools button:hover {
    color: var(--accent);
  }

  .quick-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 260px;
    overflow-y: auto;
    margin-bottom: 12px;
  }

  .quick-row {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 42px;
    padding: 5px 8px;
    border: 1px solid var(--line-hairline);
    border-radius: 8px;
    background: var(--surface-content);
    color: var(--ink-primary);
    text-align: left;
  }

  .quick-row:hover {
    background: var(--surface-hover);
  }

  .quick-row.picked {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .check {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border: 1px solid var(--line-hairline);
    border-radius: 4px;
    color: var(--on-accent);
    background: transparent;
  }

  .quick-row.picked .check {
    border-color: var(--accent);
    background: var(--accent);
  }

  .quick-copy {
    display: grid;
    min-width: 0;
    gap: 1px;
  }

  .quick-name {
    font-size: 12px;
  }

  .quick-path {
    color: var(--ink-tertiary);
    font-family: var(--font-mono);
    font-size: 9.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-empty {
    margin: 0;
    color: var(--ink-tertiary);
    font-size: 11px;
    line-height: 16px;
  }

  .or {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    color: var(--ink-tertiary);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .or::before,
  .or::after {
    content: '';
    flex: 1;
    border-top: var(--hairline);
  }

  .location-dialog {
    max-height: min(640px, 86vh);
    overflow-y: auto;
  }

  .location-dialog input {
    width: 100%;
    height: 34px;
    padding: 0 9px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-content);
    color: var(--ink-primary);
    font-family: var(--font-mono);
    font-size: 11.5px;
  }

  .location-dialog input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .form-error {
    margin: 6px 0 0 !important;
    color: var(--danger) !important;
    font-size: 10.5px !important;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 14px;
  }

  .group.bottom {
    margin-top: auto;
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border-radius: 8px;
    color: var(--ink-primary);
    text-align: left;
    transition:
      background var(--duration-hover) var(--ease-hover),
      color var(--duration-hover) var(--ease-hover);
  }

  .row:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .row.active {
    background: var(--surface-select);
    color: var(--ink-primary);
  }

  .row :global(.icon),
  .row > :global(svg) {
    color: var(--ink-secondary);
  }

  .row.active :global(.icon),
  .row.active > :global(svg) {
    color: var(--accent);
  }

  .label {
    flex: 1;
    font-size: 13.5px;
    line-height: 18px;
    font-weight: 450;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    min-width: 18px;
    height: 16px;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--surface-hover);
    color: var(--ink-secondary);
    font-size: 10.5px;
    line-height: 16px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .saved-row {
    position: relative;
    display: flex;
    align-items: center;
  }

  .saved-row .row {
    padding-right: 26px;
  }

  .row-action {
    position: absolute;
    right: 4px;
    display: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    color: var(--ink-tertiary);
  }

  .saved-row:hover .row-action,
  .row-action:focus-visible {
    display: grid;
  }

  .row-action:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .footer {
    padding: 8px 10px 0;
    border-top: var(--hairline);
  }

  .status {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    line-height: 14px;
    color: var(--ink-secondary);
    font-variant-numeric: tabular-nums;
  }

  .status.quiet {
    color: var(--ink-tertiary);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .close {
    width: 24px;
    height: 24px;
  }

  @media (max-width: 839px) {
    .sidebar:not(.overlay) {
      width: 200px;
      min-width: 200px;
    }
  }
</style>
