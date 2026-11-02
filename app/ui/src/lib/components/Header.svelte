<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { kindPlural, sortLabel } from '../format';
  import { DATE_OPTIONS, DOCUMENT_TYPES, KIND_OPTIONS } from '../filters';
  import type { Sort } from '../ipc';
  import Icon from './Icon.svelte';

  let sortOpen = $state(false);
  let columnsOpen = $state(false);
  let filterMenu = $state<'kind' | 'date' | 'type' | null>(null);
  const columnDefs = [
    ['date', 'Date modified'],
    ['size', 'Size'],
    ['kind', 'Kind'],
    ['from', 'From'],
  ] as const;

  const sorts: Sort[] = ['modified', 'added', 'size', 'name', 'kind', 'frecency'];
  let showZoom = $derived(
    shelf.viewMode === 'grid' &&
      (shelf.view !== 'home' || shelf.text.trim().length > 0) &&
      (shelf.view !== 'cleanup' || shelf.text.trim().length > 0) &&
      shelf.view !== 'settings',
  );
  let showFilters = $derived(
    shelf.text.trim().length > 0 || !['home', 'cleanup', 'settings', 'trash', 'duplicates', 'skills'].includes(shelf.view),
  );
  let kindFilterLabel = $derived(shelf.filterKind ? kindPlural(shelf.filterKind) : 'All kinds');
  let dateFilterLabel = $derived(
    DATE_OPTIONS.find((d) => d.value === shelf.filterDate)?.label ?? 'Any time',
  );
  let typeFilterLabel = $derived(
    DOCUMENT_TYPES.find((t) => t.id === shelf.filterType)?.label ?? 'All types',
  );

  $effect(() => {
    if (shelf.viewMode !== 'list') columnsOpen = false;
  });
  $effect(() => {
    if (!showFilters || !shelf.showTypeFilter) {
      if (filterMenu === 'type') filterMenu = null;
    }
    if (!showFilters) filterMenu = null;
  });

  function toggleFilter(which: 'kind' | 'date' | 'type'): void {
    filterMenu = filterMenu === which ? null : which;
    sortOpen = false;
    columnsOpen = false;
  }

  const directions: Record<Sort, string> = {
    modified: '↓',
    added: '↓',
    size: '↓',
    name: '↑',
    kind: '↑',
    frecency: '↓',
  };

  function chipIcon(kind: string): 'document' | 'recent' | 'sort' | 'download' | 'folder' | 'tag' {
    switch (kind) {
      case 'date':
        return 'recent';
      case 'size':
        return 'sort';
      case 'source':
        return 'download';
      case 'place':
        return 'folder';
      case 'tag':
        return 'tag';
      default:
        return 'document';
    }
  }
</script>

<header class="toolbar">
  <div class="lead">
    <button
      class="icon-button"
      aria-label="Toggle sidebar"
      aria-pressed={shelf.sidebarOpen}
      onclick={() => (shelf.sidebarOpen = !shelf.sidebarOpen)}
    >
      <Icon name="sidebar" size={16} />
    </button>
    <div class="history">
      <button
        class="icon-button"
        aria-label="Back"
        disabled={!shelf.canBack}
        onclick={() => shelf.back()}
      >
        <Icon name="chevron-left" size={16} />
      </button>
      <button
        class="icon-button"
        aria-label="Forward"
        disabled={!shelf.canForward}
        onclick={() => shelf.forward()}
      >
        <Icon name="chevron-right" size={16} />
      </button>
    </div>
    <div class="folder-head">
      <h1 title={shelf.title}>{shelf.title}</h1>
      {#if shelf.currentFolder}
        <div class="folder-tools">
          <button
            class="icon-button"
            title="Open in Terminal"
            aria-label="Open in Terminal"
            onclick={() => shelf.openTerminal(shelf.folderInfo?.path ?? shelf.currentFolder!)}
          >
            <Icon name="terminal" size={15} />
          </button>
          <button
            class="icon-button"
            title="Copy folder path"
            aria-label="Copy folder path"
            onclick={() => shelf.copyPath(shelf.folderInfo?.path ?? shelf.currentFolder!)}
          >
            <Icon name="copy" size={14} />
          </button>
          {#if shelf.folderInfo?.is_codebase}
            <span class="codebase" title={shelf.folderInfo.signals.join(' · ')}>Codebase</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="trail">
    {#if showZoom}
      <label class="zoom" title="Thumbnail size">
        <Icon name="grid" size={13} />
        <input
          type="range"
          min="96"
          max="320"
          step="8"
          value={shelf.cell}
          aria-label="Thumbnail size"
          oninput={(e) => shelf.setCell(Number(e.currentTarget.value), false)}
          onchange={(e) => shelf.setCell(Number(e.currentTarget.value))}
        />
      </label>
    {/if}
    <div class="segmented" role="group" aria-label="View">
      <button
        class="icon-button"
        class:active={shelf.viewMode === 'grid'}
        aria-label="Grid view"
        aria-pressed={shelf.viewMode === 'grid'}
        onclick={() => shelf.setViewMode('grid')}
      >
        <Icon name="grid" size={16} />
      </button>
      <button
        class="icon-button"
        class:active={shelf.viewMode === 'list'}
        aria-label="List view"
        aria-pressed={shelf.viewMode === 'list'}
        onclick={() => shelf.setViewMode('list')}
      >
        <Icon name="list" size={16} />
      </button>
    </div>

    {#if shelf.viewMode === 'list'}
      <div class="sort-wrap">
        <button
          class="icon-button"
          class:active={columnsOpen}
          aria-label="Choose columns"
          aria-haspopup="menu"
          aria-expanded={columnsOpen}
          onclick={() => {
            columnsOpen = !columnsOpen;
            sortOpen = false;
            filterMenu = null;
          }}
        >
          <Icon name="list" size={16} />
        </button>
        {#if columnsOpen}
          <div class="menu-backdrop" role="presentation" onclick={() => (columnsOpen = false)}></div>
          <div class="menu columns-menu" role="menu" aria-label="Columns">
            {#each columnDefs as [key, label] (key)}
              <button
                class="menu-item"
                role="menuitemcheckbox"
                aria-checked={shelf.listColumns[key]}
                onclick={() => shelf.toggleColumn(key)}
              >
                <span>{label}</span>
                {#if shelf.listColumns[key]}<Icon name="check" size={13} />{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="sort-wrap">
      <button
        class="icon-button"
        class:active={sortOpen}
        aria-label={`Sort by ${sortLabel(shelf.sort)}`}
        aria-haspopup="menu"
        aria-expanded={sortOpen}
        onclick={() => {
          sortOpen = !sortOpen;
          columnsOpen = false;
          filterMenu = null;
        }}
      >
        <Icon name="sort" size={16} />
      </button>
      {#if sortOpen}
        <div class="menu-backdrop" role="presentation" onclick={() => (sortOpen = false)}></div>
        <div class="menu" role="menu" aria-label="Sort by">
          {#each sorts as s (s)}
            <button
              class="menu-item"
              class:selected={shelf.sort === s}
              role="menuitemradio"
              aria-checked={shelf.sort === s}
              onclick={() => {
                void shelf.setSort(s);
                sortOpen = false;
              }}
            >
              <span>{sortLabel(s)}</span>
              <span class="direction" aria-hidden="true">{directions[s]}</span>
              {#if shelf.sort === s}<Icon name="check" size={13} />{/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <button
      class="icon-button"
      class:active={shelf.inspectorOpen}
      aria-label="Toggle inspector"
      aria-pressed={shelf.inspectorOpen}
      onclick={() => shelf.toggleInspector()}
    >
      <Icon name="info" size={16} />
    </button>

    <button
      class="icon-button close-app"
      title="Close Shelf"
      aria-label="Close Shelf"
      onclick={() => shelf.closeWindow()}
    >
      <Icon name="close" size={14} />
    </button>
  </div>

  {#if shelf.indexing}
    <div class="progress" aria-hidden="true"></div>
  {/if}
</header>

{#if showFilters}
  <div class="filters-bar" role="toolbar" aria-label="Filters">
    {#if shelf.showKindFilter}
      <div class="filter-wrap">
        <button
          class="filter-pill"
          class:active={shelf.filterKind !== null}
          aria-label={`Kind filter: ${kindFilterLabel}`}
          aria-haspopup="menu"
          aria-expanded={filterMenu === 'kind'}
          onclick={() => toggleFilter('kind')}
        >
          <span class="filter-key">Kind</span>
          <span class="filter-value">{kindFilterLabel}</span>
          <Icon name="chevron-down" size={11} />
        </button>
        {#if filterMenu === 'kind'}
          <div class="menu-backdrop" role="presentation" onclick={() => (filterMenu = null)}></div>
          <div class="menu filter-menu" role="menu" aria-label="Kind">
            {#each KIND_OPTIONS as option (option.label)}
              <button
                class="menu-item"
                class:selected={shelf.filterKind === option.value}
                role="menuitemradio"
                aria-checked={shelf.filterKind === option.value}
                onclick={() => {
                  shelf.setFilterKind(option.value);
                  filterMenu = null;
                }}
              >
                <span>{option.label}</span>
                {#if shelf.filterKind === option.value}<Icon name="check" size={13} />{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="filter-wrap">
      <button
        class="filter-pill"
        class:active={shelf.filterDate !== 'any'}
        aria-label={`Date filter: ${dateFilterLabel}`}
        aria-haspopup="menu"
        aria-expanded={filterMenu === 'date'}
        onclick={() => toggleFilter('date')}
      >
        <span class="filter-key">Date</span>
        <span class="filter-value">{dateFilterLabel}</span>
        <Icon name="chevron-down" size={11} />
      </button>
      {#if filterMenu === 'date'}
        <div class="menu-backdrop" role="presentation" onclick={() => (filterMenu = null)}></div>
        <div class="menu filter-menu" role="menu" aria-label="Date">
          {#each DATE_OPTIONS as option (option.value)}
            <button
              class="menu-item"
              class:selected={shelf.filterDate === option.value}
              role="menuitemradio"
              aria-checked={shelf.filterDate === option.value}
              onclick={() => {
                shelf.setDateFilter(option.value);
                filterMenu = null;
              }}
            >
              <span>{option.label}</span>
              {#if shelf.filterDate === option.value}<Icon name="check" size={13} />{/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    {#if shelf.showTypeFilter}
      <div class="filter-wrap">
        <button
          class="filter-pill"
          class:active={shelf.filterType !== 'all'}
          aria-label={`Document type filter: ${typeFilterLabel}`}
          aria-haspopup="menu"
          aria-expanded={filterMenu === 'type'}
          onclick={() => toggleFilter('type')}
        >
          <span class="filter-key">Type</span>
          <span class="filter-value">{typeFilterLabel}</span>
          <Icon name="chevron-down" size={11} />
        </button>
        {#if filterMenu === 'type'}
          <div class="menu-backdrop" role="presentation" onclick={() => (filterMenu = null)}></div>
          <div class="menu filter-menu" role="menu" aria-label="Document type">
            {#each DOCUMENT_TYPES as option (option.id)}
              <button
                class="menu-item"
                class:selected={shelf.filterType === option.id}
                role="menuitemradio"
                aria-checked={shelf.filterType === option.id}
                onclick={() => {
                  shelf.setFilterType(option.id);
                  filterMenu = null;
                }}
              >
                <span>{option.label}</span>
                {#if shelf.filterType === option.id}<Icon name="check" size={13} />{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    {#if shelf.filtersActive}
      <button
        class="filter-clear"
        onclick={() => {
          shelf.clearFilters();
          filterMenu = null;
        }}
      >
        Clear filters
      </button>
    {/if}
  </div>
{/if}

{#if shelf.chips.length}
  <div class="chips-bar" aria-label="Interpreted search">
    <span class="chips-label">Interpreted as</span>
    {#each shelf.chips as chip, i (chip.kind + chip.label)}
      <button
        class="chip"
        title={`${chip.description} Click to remove.`}
        aria-label={`Remove filter ${chip.label}. ${chip.description}`}
        onclick={() => shelf.removeChip(i)}
      >
        <Icon name={chipIcon(chip.kind)} size={11} stroke={1.6} />
        <span>{chip.label}</span>
      </button>
    {/each}
    <button class="clear-filters" onclick={() => shelf.setText('')}>Clear</button>
  </div>
{/if}

<style>
  .filters-bar {
    position: relative;
    z-index: 9;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-height: 38px;
    flex: 0 0 auto;
    padding: 5px 16px;
    background: var(--surface-content);
    border-bottom: var(--hairline);
  }

  .filter-wrap {
    position: relative;
  }

  .filter-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-content);
    color: var(--ink-secondary);
    font-size: 12px;
    line-height: 1;
  }

  .filter-pill:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .filter-pill.active {
    border-color: color-mix(in srgb, var(--accent) 32%, var(--line-hairline));
    background: var(--accent-soft);
    color: var(--accent);
  }

  .filter-key {
    color: var(--ink-tertiary);
  }

  .filter-pill.active .filter-key {
    color: color-mix(in srgb, var(--accent) 70%, var(--ink-secondary));
  }

  .filter-value {
    max-width: 148px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .filter-menu {
    top: 30px;
    right: auto;
    left: 0;
    width: 208px;
    max-height: 320px;
    overflow-y: auto;
  }

  .filter-clear {
    margin-left: 2px;
    padding: 4px 6px;
    color: var(--ink-tertiary);
    font-size: 12px;
    border-radius: 6px;
  }

  .filter-clear:hover {
    color: var(--ink-primary);
    background: var(--surface-hover);
  }

  .toolbar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    height: 48px;
    flex: 0 0 48px;
    padding: 0 12px 0 16px;
    background: var(--surface-content);
    border-bottom: var(--hairline);
    z-index: 10;
  }

  .lead {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 0 1 auto;
  }

  .history {
    display: flex;
    gap: 2px;
  }

  .folder-head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .folder-tools {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .codebase {
    margin-left: 3px;
    padding: 2px 7px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    color: var(--ink-tertiary);
    font-size: 9.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    line-height: 28px;
    font-weight: var(--weight-title);
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 240px;
  }

  .chips-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    flex: 0 0 32px;
    padding: 0 16px;
    background: var(--surface-content);
    border-bottom: var(--hairline);
    overflow-x: auto;
    scrollbar-width: none;
  }

  .chips-bar::-webkit-scrollbar {
    display: none;
  }

  .chips-label {
    font-size: 10.5px;
    line-height: 14px;
    color: var(--ink-tertiary);
    flex: 0 0 auto;
    margin-right: 2px;
  }

  .clear-filters {
    margin-left: auto;
    flex: 0 0 auto;
    font-size: 11px;
    color: var(--accent);
    padding: 2px 4px;
    border-radius: 5px;
  }

  .clear-filters:hover {
    background: var(--accent-soft);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 8px;
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-caption);
    line-height: 14px;
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--duration-hover) var(--ease-hover);
  }

  .chip:hover {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .trail {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }

  .close-app {
    margin-left: 4px;
  }

  .close-app:hover {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .zoom {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 4px 0 6px;
    height: 26px;
    border-radius: 8px;
    background: var(--surface-hover);
    color: var(--ink-tertiary);
  }

  .zoom input[type='range'] {
    width: 92px;
    accent-color: var(--accent);
    height: 18px;
    cursor: ew-resize;
  }

  .segmented {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 9px;
    background: var(--surface-hover);
  }

  .segmented .icon-button {
    width: 28px;
    height: 26px;
    border-radius: 7px;
  }

  .segmented .icon-button.active {
    background: var(--surface-raised);
    color: var(--ink-primary);
    box-shadow: 0 0 0 1px var(--line-hairline);
  }

  .sort-wrap {
    position: relative;
  }

  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
  }

  .menu {
    position: absolute;
    top: 34px;
    right: 0;
    z-index: 31;
    width: 190px;
    padding: 5px;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
    border: 1px solid var(--line-hairline);
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--ink-primary);
    text-align: left;
  }

  .menu-item:hover {
    background: var(--surface-hover);
  }

  .menu-item.selected {
    color: var(--accent);
  }

  .menu-item > span:first-child {
    flex: 1;
  }

  .direction {
    color: var(--ink-tertiary);
    font-size: 11px;
  }

  .columns-menu {
    width: 176px;
  }

  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    overflow: hidden;
    background: var(--accent-soft);
  }

  .progress::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    width: 30%;
    background: var(--accent);
    animation: slide 1.6s ease-in-out infinite;
  }

  @keyframes slide {
    0% {
      left: -30%;
    }
    100% {
      left: 100%;
    }
  }

  @media (max-width: 900px) {
    h1 {
      max-width: 140px;
    }
  }
</style>
