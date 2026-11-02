<script lang="ts">
  import { trapFocus } from '../focus';
  import { tick } from 'svelte';
  import { ipc, type FileMeta, type OpenWithApp } from '../ipc';
  import { shelf } from '../stores.svelte';
  import Icon from './Icon.svelte';
  import { smartName } from '../smart';

  let {
    folder = false,
    x,
    y,
    file,
    open,
    onclose,
  }: {
    folder?: boolean;
    x: number;
    y: number;
    file: FileMeta;
    open: boolean;
    onclose: () => void;
  } = $props();

  // Snapshot the prop while the menu is mounted. Closing the menu nulls the
  // parent's context state; reading the reactive prop getter after that throws.
  // svelte-ignore state_referenced_locally
  const target = file;

  let menu: HTMLDivElement | null = $state(null);
  let apps = $state<OpenWithApp[] | null>(null);
  let loadingApps = $state(false);
  let showApps = $state(false);
  let error = $state<string | null>(null);
  let settingDefault = $state<string | null>(null);
  const paths = $derived(shelf.selection.includes(target.path) ? shelf.selection : [target.path]);

  const WIDTH = 232;
  const left = $derived(Math.max(8, Math.min(x, window.innerWidth - WIDTH - 8)));
  const top = $derived(Math.max(8, Math.min(y, window.innerHeight - 120)));

  $effect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    void tick().then(() => menu?.focus());
    return () => previous?.focus();
  });

  async function showAppChoices(): Promise<void> {
    if (!showApps) await toggleApps();
  }

  async function toggleApps(): Promise<void> {
    if (showApps) {
      showApps = false;
      return;
    }
    showApps = true;
    if (apps) return;
    loadingApps = true;
    try {
      apps = await ipc.openWithApps(target.path);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loadingApps = false;
    }
  }

  // Capture the file before closing the menu. Closing unmounts this component,
  // and reading the prop after that is what produced the intermittent
  // "null is not an object" error.
  async function openDefault(): Promise<void> {
    onclose();
    await shelf.openFile(target);
  }

  async function openWith(app: string): Promise<void> {
    onclose();
    try {
      await ipc.openWith(target.path, app);
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    }
  }

  async function makeDefault(app: OpenWithApp): Promise<void> {
    settingDefault = app.id;
    try {
      await ipc.setDefaultApp(target.path, app.id);
      apps = await ipc.openWithApps(target.path);
      shelf.showToast(`${app.name} is now the default for this file type.`);
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    } finally {
      settingDefault = null;
    }
  }

  async function restoreFromTrash(): Promise<void> {
    onclose();
    try {
      await ipc.restoreTrash(target.trash_path ?? '', target.path);
      shelf.showToast('Restored from Trash.');
      await shelf.refresh();
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    }
  }

  function deletePermanently(): void {
    onclose();
    if (target.trash_path) shelf.requestPurge([target.trash_path]);
  }

  async function openTerminal(): Promise<void> {
    onclose();
    try {
      await ipc.openTerminal(target.path);
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    }
  }

  function moveTo(): void {
    onclose();
    shelf.requestTransfer(paths, 'move');
  }

  function copyTo(): void {
    onclose();
    shelf.requestTransfer(paths, 'copy');
  }

  async function showInFolder(): Promise<void> {
    onclose();
    await shelf.showInFiles([target.path]);
  }

  async function copyPath(): Promise<void> {
    onclose();
    await shelf.copyPath(paths.join('\n'));
  }

  async function quickLook(): Promise<void> {
    onclose();
    shelf.select([target.path]);
    shelf.quickLookOpen = true;
  }

  function rename(): void {
    onclose();
    shelf.requestRename(target.path, target.name);
  }

  async function moveToTrash(): Promise<void> {
    onclose();
    shelf.requestTrash(paths);
  }

  function onKey(event: KeyboardEvent): void {
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
<div class="backdrop" role="presentation" onclick={onclose} oncontextmenu={(e) => { e.preventDefault(); onclose(); }}></div>

<div
  class="menu"
  bind:this={menu}
  role="menu"
  tabindex="-1"
  use:trapFocus
  style:left="{left}px"
  style:top="{top}px"
  style:max-height="calc(100vh - {top + 12}px)"
>
  <div class="head">
    <span class="name" title={target.path}>{smartName(target).title}</span>
    {#if smartName(target).detail}<span class="real-name">{smartName(target).detail}</span>{/if}
  </div>

  {#if shelf.view === 'trash'}
    <button class="item" role="menuitem" onclick={restoreFromTrash}>
      <Icon name="undo" size={14} /><span>Restore</span>
    </button>
    <button class="item danger" role="menuitem" onclick={deletePermanently}>
      <Icon name="trash" size={14} /><span>Delete permanently</span>
    </button>
  {:else}
    <button class="item" role="menuitem" onclick={openDefault}>
      <Icon name="external" size={14} /><span>Open</span>
    </button>
  {/if}
  {#if !folder && shelf.view !== 'trash'}
    <button
      class="item"
      role="menuitem"
      onclick={toggleApps}
      onmouseenter={showAppChoices}
      onfocus={showAppChoices}
      aria-expanded={showApps}
    >
      <Icon name="app" size={14} /><span>Open with</span>
      <Icon name={showApps ? 'chevron-down' : 'chevron-right'} size={12} />
    </button>
    {#if showApps}
    <div class="sub">
      {#if loadingApps}
        <p class="note">Finding applications…</p>
      {:else if error}
        <p class="note">{error}</p>
      {:else if apps && apps.length}
        {#each apps as app (app.id)}
          <div class="app-row">
            <button class="item app" role="menuitem" onclick={() => openWith(app.id)}>
              <span class="app-name">{app.name}</span>
              {#if app.default}<span class="tag">Default</span>{/if}
            </button>
            {#if !app.default}
              <button
                class="make-default"
                title={`Make ${app.name} the default for this file type`}
                disabled={settingDefault === app.id}
                onclick={() => makeDefault(app)}
              >
                {settingDefault === app.id ? '…' : 'Set default'}
              </button>
            {/if}
          </div>
        {/each}
      {:else}
        <p class="note">No applications registered for this file type.</p>
      {/if}
    </div>
    {/if}
  {/if}

  {#if shelf.view !== 'trash'}
  <div class="sep"></div>
  {#if !folder}
  <button class="item" role="menuitem" onclick={quickLook}>
    <Icon name="search" size={14} /><span>Quick Look</span>
    <kbd>Space</kbd>
  </button>
  <button class="item" role="menuitem" onclick={showInFolder}>
    <Icon name="folder" size={14} /><span>Open in folder</span>
    <kbd>Ctrl ↵</kbd>
  </button>
  {/if}
  <button class="item" role="menuitem" onclick={openTerminal}>
    <Icon name="external" size={14} /><span>Open in Terminal</span>
  </button>
  <div class="sep"></div>
  <button class="item" role="menuitem" onclick={copyTo}>
    <Icon name="copy" size={14} /><span>Copy to…</span>
  </button>
  <button class="item" role="menuitem" onclick={moveTo}>
    <Icon name="folder" size={14} /><span>Move to…</span>
  </button>
  <button class="item" role="menuitem" onclick={copyPath}>
    <Icon name="copy" size={14} /><span>Copy path</span>
  </button>
  <button class="item" role="menuitem" onclick={rename}>
    <Icon name="edit" size={14} /><span>Rename</span>
    <kbd>F2</kbd>
  </button>
  <div class="sep"></div>
  <button class="item danger" role="menuitem" onclick={moveToTrash}>
    <Icon name="trash" size={14} /><span>Move to Trash</span>
    <kbd>Del</kbd>
  </button>
  {/if}
</div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 44;
  }

  .menu {
    position: fixed;
    z-index: 45;
    width: 232px;
    overflow-y: auto;
    padding: 5px;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    border: 1px solid var(--line-hairline);
    box-shadow: var(--shadow-popover);
  }

  .real-name {
    color: var(--ink-tertiary);
    font-family: var(--font-mono);
    font-size: 9.5px;
  }

  .head {
    padding: 6px 8px 7px;
    border-bottom: var(--hairline);
    margin-bottom: 4px;
  }

  .name {
    display: block;
    font-size: 11px;
    line-height: 14px;
    color: var(--ink-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 28px;
    padding: 0 8px;
    border-radius: 6px;
    color: var(--ink-primary);
    font-size: 12px;
    text-align: left;
  }

  .item:hover,
  .item:focus-visible {
    background: var(--surface-hover);
  }

  .item > span:nth-child(2) {
    flex: 1;
  }

  .item.danger {
    color: var(--danger);
  }

  .sub {
    margin: 2px 0 4px 10px;
    padding-left: 6px;
    border-left: var(--hairline);
    max-height: 220px;
    overflow-y: auto;
  }

  .item.app {
    min-height: 26px;
    font-size: 11.5px;
  }

  .app-row {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .app-row .item.app {
    flex: 1;
    min-width: 0;
  }

  .make-default {
    flex: 0 0 auto;
    padding: 2px 5px;
    border-radius: 4px;
    color: var(--ink-tertiary);
    font-size: 9.5px;
  }

  .make-default:hover {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .app-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    flex: 0 0 auto !important;
    font-size: 9.5px;
    color: var(--ink-tertiary);
  }

  .note {
    margin: 4px 8px;
    font-size: 11px;
    line-height: 15px;
    color: var(--ink-tertiary);
  }

  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line-hairline);
  }

  kbd {
    font-family: var(--font-ui);
    font-size: 9.5px;
    color: var(--ink-tertiary);
    padding: 1px 4px;
    border-radius: 4px;
    border: 1px solid var(--line-hairline);
  }
</style>
