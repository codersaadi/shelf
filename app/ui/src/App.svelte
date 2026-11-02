<script lang="ts">
  import { onMount } from 'svelte';
  import { shelf } from './lib/stores.svelte';
  import { folderName, kindPlural, middleTruncate } from './lib/format';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Header from './lib/components/Header.svelte';
  import SearchDock from './lib/components/SearchDock.svelte';
  import Home from './lib/components/Home.svelte';
  import FileGrid from './lib/components/FileGrid.svelte';
  import FileList from './lib/components/FileList.svelte';
  import Inspector from './lib/components/Inspector.svelte';
  import CleanupView from './lib/components/CleanupView.svelte';
  import DuplicatesView from './lib/components/DuplicatesView.svelte';
  import Toast from './lib/components/Toast.svelte';
  import QuickLook from './lib/components/QuickLook.svelte';
  import QuickSearch from './lib/components/QuickSearch.svelte';
  import CommandPalette from './lib/components/CommandPalette.svelte';
  import ContextMenu from './lib/components/ContextMenu.svelte';
  import TrashConfirm from './lib/components/TrashConfirm.svelte';
  import PurgeConfirm from './lib/components/PurgeConfirm.svelte';
  import RenameDialog from './lib/components/RenameDialog.svelte';
  import TransferDialog from './lib/components/TransferDialog.svelte';
  import SettingsView from './lib/components/SettingsView.svelte';
  import FolderRail from './lib/components/FolderRail.svelte';
  import FolderBar from './lib/components/FolderBar.svelte';
  import { comboOf } from './lib/shortcuts';
  import Icon from './lib/components/Icon.svelte';
  import type { FileKind, FileMeta, FolderEntry } from './lib/ipc';

  let columns = $state(4);
  let anchor = $state<string | null>(null);
  let narrow = $state(false);
  let contextMenu = $state<{ x: number; y: number; file: FileMeta; open: boolean } | null>(null);
  let folderMenu = $state<{ x: number; y: number; folder: FolderEntry; open: boolean } | null>(null);

  let isHome = $derived(shelf.view === 'home' && !shelf.text.trim());
  let isCleanup = $derived(shelf.view === 'cleanup' && !shelf.text.trim());
  let isSettings = $derived(shelf.view === 'settings' && !shelf.text.trim());
  let isDuplicates = $derived(shelf.view === 'duplicates' && !shelf.text.trim());
  let isList = $derived(!isHome && !isCleanup && !isSettings && !isDuplicates);
  let grouped = $derived(
    !shelf.text.trim() && (shelf.sort === 'modified' || shelf.sort === 'added'),
  );
  let dateField = $derived(shelf.sort === 'added' ? ('added_at' as const) : ('mtime' as const));
  let empty = $derived(isList && !shelf.loading && shelf.items.length === 0);

  let emptyTitle = $derived.by(() => {
    if (shelf.filtersActive && !shelf.text.trim()) return `No files with ${shelf.filterSummary}`;
    if (shelf.text.trim()) return shelf.chips.length ? 'No files match' : `No files match “${middleTruncate(shelf.text.trim(), 36)}”`;
    if (shelf.view.startsWith('place:')) {
      return `Nothing in ${folderName(shelf.view.slice(6))} yet`;
    }
    if (shelf.view === 'downloads') return 'Nothing here yet';
    if (shelf.view === 'screenshots') return 'No screenshots yet';
    if (shelf.view === 'trash') return 'Trash is empty';
    if (shelf.view === 'recent') return 'Nothing new yet';
    if (shelf.view.startsWith('kind:')) return `No ${kindPlural(shelf.view.slice(5) as FileKind)} yet`;
    return 'No files here';
  });

  let emptyBody = $derived.by(() => {
    if (shelf.filtersActive && !shelf.text.trim()) return 'Clear filters to see more files.';
    if (shelf.text.trim()) return 'Remove a filter to widen the search.';
    if (shelf.view.startsWith('place:')) {
      return 'Files under this location appear as they are indexed.';
    }
    if (shelf.view === 'downloads') return 'New downloads appear as they finish.';
    if (shelf.view === 'screenshots') return 'Screenshots you take appear here.';
    if (shelf.view === 'trash') return 'Files you move to Trash stay here until you empty it.';
    if (shelf.view === 'recent') return 'New files appear as you download and create them.';
    return 'Files of this kind appear as they are indexed.';
  });

  onMount(() => {
    void shelf.init();
    const onResize = (): void => {
      narrow = window.innerWidth < 840;
    };
    onResize();
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('resize', onResize);
      shelf.destroy();
    };
  });

  $effect(() => {
    void shelf.view;
    if (narrow && shelf.sidebarOpen) shelf.sidebarOpen = false;
  });

  $effect(() => {
    document.title = shelf.title === 'Home' ? 'Shelf' : `${shelf.title} — Shelf`;
  });

  function focusSearch(select = false): void {
    const input = document.querySelector<HTMLInputElement>('.search-dock input');
    if (!input) return;
    input.focus();
    if (select) input.select();
    else input.setSelectionRange(shelf.text.length, shelf.text.length);
  }

  function openContext(file: FileMeta, event: MouseEvent): void {
    event.preventDefault();
    if (!shelf.selection.includes(file.path)) shelf.select([file.path]);
    contextMenu = { x: event.clientX, y: event.clientY, file, open: true };
  }

  function closeContext(): void {
    if (contextMenu) contextMenu.open = false;
  }

  function openFolderContext(folder: FolderEntry, event: MouseEvent): void {
    event.preventDefault();
    contextMenu = null;
    folderMenu = { x: event.clientX, y: event.clientY, folder, open: true };
  }

  function closeFolderContext(): void {
    if (folderMenu) folderMenu.open = false;
  }

  function selectPaths(paths: string[], additive: boolean): void {
    if (additive) {
      shelf.select([...new Set([...shelf.selection, ...paths])]);
    } else {
      shelf.select(paths);
    }
    anchor = paths.at(-1) ?? anchor;
  }

  function selectFile(file: FileMeta, event: MouseEvent): void {
    const path = file.path;
    if (event.shiftKey && anchor) {
      const a = shelf.items.findIndex((f) => f.path === anchor);
      const b = shelf.items.findIndex((f) => f.path === path);
      if (a >= 0 && b >= 0) {
        const [from, to] = a < b ? [a, b] : [b, a];
        shelf.select(shelf.items.slice(from, to + 1).map((f) => f.path));
        return;
      }
    }
    if (event.metaKey || event.ctrlKey) {
      const next = shelf.selection.includes(path)
        ? shelf.selection.filter((p) => p !== path)
        : [...shelf.selection, path];
      shelf.select(next);
      anchor = path;
      return;
    }
    shelf.select([path]);
    anchor = path;
  }

  function moveSelection(delta: number, extend = false): void {
    if (!shelf.items.length) return;
    const current = shelf.selection[0];
    let index = current ? shelf.items.findIndex((f) => f.path === current) : -1;
    index = index < 0 ? 0 : Math.min(shelf.items.length - 1, Math.max(0, index + delta));
    const path = shelf.items[index]?.path;
    if (!path) return;
    if (extend) {
      shelf.select([...new Set([...shelf.selection, path])]);
    } else {
      shelf.select([path]);
      anchor = path;
    }
  }

  function stepSelection(delta: number): void {
    moveSelection(delta > 0 ? 1 : -1);
  }

  function onWheel(event: WheelEvent): void {
    if (!event.ctrlKey) return;
    event.preventDefault();
    shelf.setCell(shelf.cell + (event.deltaY < 0 ? 8 : -8));
  }

  function openQuickLook(): void {
    if (shelf.selection.length || shelf.inspector) shelf.quickLookOpen = true;
  }

  function isTyping(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el) return false;
    return el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable;
  }

  function selectAllVisible(): void {
    const paths = shelf.items.length
      ? shelf.items.map((f) => f.path)
      : [
          ...(shelf.home?.recent ?? []),
          ...(shelf.home?.shelves.flatMap((group) => group.files) ?? []),
        ].map((f) => f.path);
    shelf.select([...new Set(paths)]);
  }

  function onKey(event: KeyboardEvent): void {
    if (
      shelf.quickLookOpen ||
      shelf.quickSearchOpen ||
      shelf.paletteOpen ||
      shelf.trashPrompt ||
      shelf.purgePrompt ||
      shelf.renamePrompt ||
      contextMenu?.open ||
      folderMenu?.open
    )
      return;
    const mod = event.ctrlKey || event.metaKey;
    const key = event.key;
    const lower = key.toLowerCase();

    if (key === 'Escape') {
      if (shelf.text) {
        shelf.setText('');
        (document.activeElement as HTMLElement | null)?.blur();
        if (shelf.view.startsWith('saved:') || shelf.view === 'search') void shelf.setView('home');
      } else if (shelf.selection.length) {
        shelf.select([]);
      }
      return;
    }

    if (mod && lower === 'k') {
      event.preventDefault();
      shelf.quickSearchOpen = true;
      return;
    }
    if (mod && event.shiftKey && lower === 'p') {
      event.preventDefault();
      shelf.paletteOpen = true;
      return;
    }
    const combo = comboOf(event);
    if (combo === shelf.shortcuts.search && combo !== 'ctrl+f') {
      event.preventDefault();
      focusSearch(true);
      return;
    }
    if (mod && lower === 'f' && shelf.shortcuts.search === 'ctrl+f') {
      event.preventDefault();
      focusSearch(true);
      return;
    }
    if (isTyping(event.target)) return;

    if (combo !== 'ctrl+a' && combo === shelf.shortcuts.selectAll) {
      event.preventDefault();
      selectAllVisible();
      return;
    }
    if (combo !== 'ctrl+i' && combo === shelf.shortcuts.inspector) {
      event.preventDefault();
      shelf.toggleInspector();
      return;
    }
    if (combo !== 'ctrl+1' && combo === shelf.shortcuts.grid) {
      event.preventDefault();
      shelf.setViewMode('grid');
      return;
    }
    if (combo !== 'ctrl+2' && combo === shelf.shortcuts.list) {
      event.preventDefault();
      shelf.setViewMode('list');
      return;
    }
    if (combo !== 'ctrl+enter' && combo === shelf.shortcuts.openFolder) {
      event.preventDefault();
      void shelf.showInFiles(shelf.selection);
      return;
    }
    if (combo !== 'ctrl+shift+c' && combo === shelf.shortcuts.copyPath) {
      event.preventDefault();
      void shelf.copyPath(shelf.selection.join('\n'));
      return;
    }
    if (combo !== 'delete' && combo === shelf.shortcuts.delete) {
      event.preventDefault();
      void shelf.trashSelection();
      return;
    }
    if (combo !== 'space' && combo === shelf.shortcuts.quickLook) {
      event.preventDefault();
      openQuickLook();
      return;
    }

    if (mod && lower === 'a' && shelf.shortcuts.selectAll === 'ctrl+a') {
      event.preventDefault();
      selectAllVisible();
      return;
    }

    if (key === '/') {
      event.preventDefault();
      focusSearch();
    } else if (!mod && !event.altKey && key.length === 1 && /[^\s]/.test(key)) {
      event.preventDefault();
      focusSearch();
      shelf.setText(shelf.text + key);
    } else if (mod && lower === 'i' && shelf.shortcuts.inspector === 'ctrl+i') {
      event.preventDefault();
      shelf.toggleInspector();
    } else if (mod && lower === 'b') {
      event.preventDefault();
      shelf.sidebarOpen = !shelf.sidebarOpen;
    } else if (
      mod &&
      (key === '1' || key === '2') &&
      (key === '1' ? shelf.shortcuts.grid : shelf.shortcuts.list) === (key === '1' ? 'ctrl+1' : 'ctrl+2')
    ) {
      event.preventDefault();
      shelf.setViewMode(key === '1' ? 'grid' : 'list');
    } else if (mod && (key === '+' || key === '=')) {
      event.preventDefault();
      shelf.setCell(shelf.cell + 24);
    } else if (mod && key === '-') {
      event.preventDefault();
      shelf.setCell(shelf.cell - 24);
    } else if (mod && key === 'Enter' && shelf.shortcuts.openFolder === 'ctrl+enter') {
      event.preventDefault();
      void shelf.showInFiles(shelf.selection);
    } else if (mod && event.shiftKey && lower === 'c' && shelf.shortcuts.copyPath === 'ctrl+shift+c') {
      event.preventDefault();
      const first = shelf.selection[0];
      if (first) void shelf.copyPath(first);
    } else if (mod && lower === 'z') {
      event.preventDefault();
      if (shelf.toast?.undo) void shelf.undo(shelf.toast.undo);
    } else if (event.altKey && key === 'ArrowLeft') {
      event.preventDefault();
      shelf.back();
    } else if (event.altKey && key === 'ArrowRight') {
      event.preventDefault();
      shelf.forward();
    } else if (key === 'F2') {
      event.preventDefault();
      const file = shelf.selectedFile;
      if (file) shelf.requestRename(file.path, file.name);
    } else if (key === 'Delete' && shelf.shortcuts.delete === 'delete') {
      event.preventDefault();
      void shelf.trashSelection();
    } else if (key === ' ' && shelf.shortcuts.quickLook === 'space') {
      const active = document.activeElement as HTMLElement | null;
      // Chrome buttons keep Space for activation; file tiles inside the content
      // area should open Quick Look instead.
      if (active && ['BUTTON', 'A'].includes(active.tagName) && !active.closest('.content')) return;
      event.preventDefault();
      openQuickLook();
    } else if (key === 'ArrowRight') {
      event.preventDefault();
      moveSelection(1, event.shiftKey);
    } else if (key === 'ArrowLeft') {
      event.preventDefault();
      moveSelection(-1, event.shiftKey);
    } else if (key === 'ArrowDown') {
      event.preventDefault();
      moveSelection(shelf.viewMode === 'grid' ? columns : 1, event.shiftKey);
    } else if (key === 'ArrowUp') {
      event.preventDefault();
      moveSelection(shelf.viewMode === 'grid' ? -columns : -1, event.shiftKey);
    } else if (key === 'Home') {
      event.preventDefault();
      const first = shelf.items[0];
      if (first) shelf.select([first.path]);
    } else if (key === 'End') {
      event.preventDefault();
      const last = shelf.items[shelf.items.length - 1];
      if (last) shelf.select([last.path]);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  {#if shelf.sidebarOpen && !narrow}
    <Sidebar />
  {/if}

  {#if narrow && shelf.sidebarOpen}
    <div class="scrim" role="presentation" onclick={() => (shelf.sidebarOpen = false)}></div>
    <Sidebar overlay onclose={() => (shelf.sidebarOpen = false)} />
  {/if}

  <div class="main">
    <Header />

    {#if shelf.view.startsWith('place:') && !shelf.text.trim()}
      <FolderBar />
    {/if}

    {#if (shelf.view === 'downloads' || shelf.view.startsWith('place:')) && !shelf.text.trim()}
      <FolderRail oncontext={openFolderContext} />
    {/if}

    <div class="workspace">
      <div class="content" onwheel={onWheel}>
        {#if isHome}
          <Home oncontext={openContext} />
        {:else if isCleanup}
          <CleanupView />
        {:else if isSettings}
          <SettingsView />
        {:else if isDuplicates}
          <DuplicatesView />
        {:else if shelf.viewMode === 'grid'}
          <FileGrid
            items={shelf.items}
            cell={shelf.cell}
            selected={shelf.selection}
            bind:columns
            grouped={grouped}
            dateField={dateField}
            onselect={selectFile}
            onopen={(file) => shelf.openFile(file)}
            onloadmore={() => shelf.loadMore()}
            oncontext={openContext}
            onselectpaths={selectPaths}
          />
        {:else}
          <FileList
            items={shelf.items}
            selected={shelf.selection}
            onselect={selectFile}
            onopen={(file) => shelf.openFile(file)}
            onloadmore={() => shelf.loadMore()}
            oncontext={openContext}
            onselectpaths={selectPaths}
            columns={shelf.listColumns}
            grouped={grouped}
            dateField={dateField}
          />
        {/if}

        {#if empty}
          <div class="empty">
            <Icon name={shelf.text.trim() || shelf.filtersActive ? 'search' : 'folder'} size={24} />
            <h2>{emptyTitle}</h2>
            <p>{emptyBody}</p>
            {#if shelf.filtersActive}
              <button class="button" onclick={() => shelf.clearFilters()}>Clear filters</button>
            {/if}
            {#if shelf.text.trim()}
              <button class="button" onclick={() => shelf.setText('')}>Clear search</button>
            {/if}
          </div>
        {/if}
      </div>

      {#if shelf.inspectorOpen && shelf.selection.length > 0 && !isSettings && !isCleanup && !isDuplicates}
        <Inspector />
      {/if}
    </div>

    <SearchDock onquicksearch={() => (shelf.quickSearchOpen = true)} />
  </div>
</div>

{#if shelf.quickSearchOpen}
  <QuickSearch onclose={() => (shelf.quickSearchOpen = false)} />
{/if}

{#if shelf.paletteOpen}
  <CommandPalette onclose={() => (shelf.paletteOpen = false)} />
{/if}

{#if shelf.quickLookOpen}
  <QuickLook onclose={() => (shelf.quickLookOpen = false)} onstep={stepSelection} />
{/if}

{#if shelf.trashPrompt}
  <TrashConfirm />
  <PurgeConfirm />
  <RenameDialog />
  <TransferDialog />
{/if}

{#if contextMenu}
  {#key contextMenu.file.path}
    <ContextMenu
      x={contextMenu.x}
      y={contextMenu.y}
      file={contextMenu.file}
      open={contextMenu.open}
      onclose={closeContext}
    />
  {/key}
{/if}

{#if folderMenu}
  {#key folderMenu.folder.path}
    <ContextMenu
      x={folderMenu.x}
      y={folderMenu.y}
      file={{
        id: -1,
        path: folderMenu.folder.path,
        parent: folderMenu.folder.path.slice(0, Math.max(0, folderMenu.folder.path.lastIndexOf('/'))),
        name: folderMenu.folder.name,
        ext: '',
        kind: 'other',
        size: 0,
        mtime: folderMenu.folder.mtime,
        added_at: folderMenu.folder.mtime,
        is_screenshot: false,
        in_trash: false,
      }}
      open={folderMenu.open}
      folder={true}
      onclose={closeFolderContext}
    />
  {/key}
{/if}

<Toast />

<style>
  .app {
    display: flex;
    height: 100%;
    background: var(--surface-window);
    overflow: hidden;
    position: relative;
  }

  .main {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    background: var(--surface-content);
  }

  .workspace {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
    padding-bottom: 58px;
  }

  .content {
    position: relative;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    background: var(--surface-content);
  }

  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 6px;
    padding: 24px;
    text-align: center;
    color: var(--ink-tertiary);
    pointer-events: none;
  }

  .empty :global(button) {
    pointer-events: auto;
  }

  .empty h2 {
    margin: 8px 0 0;
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
    color: var(--ink-primary);
  }

  .empty p {
    margin: 0 0 8px;
    max-width: 42ch;
    font-size: 12px;
    line-height: 17px;
  }

  .scrim {
    position: absolute;
    inset: 0;
    z-index: 35;
    background: color-mix(in srgb, #000 20%, transparent);
  }
</style>
