<script lang="ts">
  import { onMount } from 'svelte';
  import { ipc, type DefaultCategory, type SettingsInfo } from '../ipc';
  import { shelf } from '../stores.svelte';
  import Icon from './Icon.svelte';
  import SelectMenu from './SelectMenu.svelte';
  import { loadThemeFile } from '../theme';
  import {
    comboOf,
    prettyCombo,
    SHORTCUT_LABELS,
    shortcutKeys,
    type ShortcutAction,
  } from '../shortcuts';

  let info = $state<SettingsInfo | null>(null);
  let categories = $state<DefaultCategory[]>([]);
  let loading = $state(true);
  let saving = $state<string | null>(null);
  let error = $state<string | null>(null);
  let newRoot = $state('');
  let addingRoot = $state(false);
  let rootError = $state<string | null>(null);
  let recording = $state<ShortcutAction | null>(null);
  let themePath = $state('');
  let themeError = $state<string | null>(null);
  let themeBusy = $state(false);
  const textSizes = [
    { scale: 0.9, label: 'Compact' },
    { scale: 1, label: 'Default' },
    { scale: 1.12, label: 'Large' },
    { scale: 1.25, label: 'Larger' },
  ];
  const shortcutList = shortcutKeys();

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const [nextInfo, nextCategories] = await Promise.all([ipc.settingsInfo(), ipc.defaultApps()]);
      info = nextInfo;
      categories = nextCategories;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function chooseDefault(category: DefaultCategory, app: string): Promise<void> {
    if (!app) return;
    saving = category.mime;
    try {
      await ipc.setDefaultMime(category.mime, app);
      categories = await ipc.defaultApps();
      shelf.showToast('Default application updated.');
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    } finally {
      saving = null;
    }
  }

  async function addRoot(): Promise<void> {
    const path = newRoot.trim();
    if (!path || addingRoot) return;
    if (!(await ipc.pathIsDir(path).catch(() => false))) {
      rootError = 'That folder does not exist or is not readable.';
      return;
    }
    addingRoot = true;
    rootError = null;
    try {
      await ipc.addIndexRoot(path);
      await ipc.indexRoot(path);
      await load();
      newRoot = '';
      shelf.showToast('Folder added and indexing.');
    } catch (e) {
      rootError = e instanceof Error ? e.message : String(e);
    } finally {
      addingRoot = false;
    }
  }

  async function openTheme(): Promise<void> {
    try {
      await ipc.openThemeFile();
    } catch (e) {
      themeError = e instanceof Error ? e.message : String(e);
    }
  }

  async function reloadTheme(): Promise<void> {
    themeBusy = true;
    themeError = null;
    try {
      await loadThemeFile();
      shelf.showToast('Theme reloaded.');
    } catch (e) {
      themeError = e instanceof Error ? e.message : String(e);
    } finally {
      themeBusy = false;
    }
  }

  async function openData(): Promise<void> {
    try {
      await ipc.openDataFolder();
    } catch (e) {
      shelf.showToast(e instanceof Error ? e.message : String(e));
    }
  }

  onMount(() => {
    void load();
    void ipc
      .themeFile()
      .then((path) => (themePath = path))
      .catch(() => {});

    const capture = (event: KeyboardEvent): void => {
      if (!recording) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.key === 'Escape') {
        recording = null;
        return;
      }
      const combo = comboOf(event);
      if (!combo) return;
      const conflict = shortcutList.find(
        (action) => action !== recording && shelf.shortcuts[action] === combo,
      );
      if (conflict) {
        shelf.showToast(`${prettyCombo(combo)} is already used by ${SHORTCUT_LABELS[conflict]}.`);
        return;
      }
      shelf.setShortcut(recording, combo);
      recording = null;
    };
    window.addEventListener('keydown', capture, true);
    return () => window.removeEventListener('keydown', capture, true);
  });
</script>

<div class="settings">
  <div class="settings-scroll">
    <header class="head">
      <h2>Settings</h2>
      <p>Appearance, indexing, default applications and access. Changes apply immediately.</p>
    </header>

    <section class="appearance">
      <div class="section-head">
        <div>
          <h3>Appearance</h3>
          <p>Follow the desktop theme or pin Shelf to light or dark.</p>
        </div>
      </div>
      <div class="theme-choice" role="group" aria-label="Theme">
        {#each [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']] as [value, label] (value)}
          <button
            class="theme-button"
            class:active={shelf.theme === value}
            aria-pressed={shelf.theme === value}
            onclick={() => shelf.setTheme(value as 'system' | 'light' | 'dark')}
          >
            {label}
          </button>
        {/each}
      </div>

      <div class="text-size">
        <span>Text size</span>
        <div class="theme-choice" role="group" aria-label="Text size">
          {#each textSizes as { scale, label } (scale)}
            <button
              class="theme-button"
              class:active={Math.abs(shelf.zoom - scale) < 0.01}
              aria-pressed={Math.abs(shelf.zoom - scale) < 0.01}
              onclick={() => shelf.setZoom(scale)}
            >
              {label}
            </button>
          {/each}
        </div>
      </div>

      <div class="theme-file">
        <div class="theme-file-copy">
          <span>Theme file</span>
          <code>{themePath || 'Creating theme file…'}</code>
          <p>Fonts, colors and radii are CSS custom properties. Edit the file, then reload.</p>
        </div>
        <div class="theme-file-actions">
          <button class="button" disabled={!themePath} onclick={openTheme}>Open file</button>
          <button class="button" disabled={themeBusy} onclick={reloadTheme}>
            {themeBusy ? 'Reloading…' : 'Reload'}
          </button>
        </div>
      </div>
      {#if themeError}<p class="root-error">{themeError}</p>{/if}
    </section>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <section>
      <div class="section-head">
        <div>
          <h3>Indexed folders</h3>
          <p>Shelf only reads these locations. Search never leaves the index.</p>
        </div>
        <button class="button" onclick={() => shelf.reindex()} disabled={shelf.indexing !== null}>
          Reindex all
        </button>
      </div>
      {#if loading && !info}
        <p class="quiet">Reading settings…</p>
      {:else if info}
        <ul class="roots">
          {#each info.roots as root (root)}
            <li><Icon name="folder" size={14} /><span>{root}</span></li>
          {/each}
        </ul>
        <div class="path-row">
          <span>Database</span>
          <code>{info.db}</code>
        </div>
        <button class="button" onclick={openData}>Open data folder</button>
        <form
          class="root-form"
          onsubmit={(e) => {
            e.preventDefault();
            void addRoot();
          }}
        >
          <input
            bind:value={newRoot}
            placeholder="Add a folder, e.g. /home/you/work"
            spellcheck="false"
            autocomplete="off"
            aria-label="Folder to index"
          />
          <button class="button" type="submit" disabled={!newRoot.trim() || addingRoot}>
            {addingRoot ? 'Adding…' : 'Add folder'}
          </button>
        </form>
        {#if rootError}<p class="root-error">{rootError}</p>{/if}
      {/if}
    </section>

    <section>
      <div class="section-head">
        <div>
          <h3>Default applications</h3>
          <p>These are the desktop MIME defaults. Set any file type from its Open with menu too.</p>
        </div>
      </div>
      {#if loading}
        <p class="quiet">Reading handlers…</p>
      {:else}
        <div class="defaults">
          {#each categories as category (category.mime)}
            <label class="default-row">
              <span class="default-copy">
                <strong>{category.label}</strong>
                <span>{category.mime}</span>
              </span>
              <SelectMenu
                value={category.default_app ?? ''}
                options={category.apps.map((app) => ({ value: app.id, label: app.name }))}
                placeholder="System default"
                label={`Default application for ${category.label}`}
                disabled={saving === category.mime || category.apps.length === 0}
                onchange={(app) => chooseDefault(category, app)}
              />
            </label>
          {/each}
        </div>
      {/if}
    </section>

    <section>
      <div class="section-head">
        <div>
          <h3>Keyboard</h3>
          <p>Choose Rebind, then press the new chord. Escape cancels.</p>
        </div>
      </div>
      <dl class="shortcuts">
        {#each shortcutList as action (action)}
          <div class="shortcut-row">
            <dt>{SHORTCUT_LABELS[action]}</dt>
            <dd>
              <kbd>{prettyCombo(shelf.shortcuts[action])}</kbd>
              <button
                class="rebind"
                class:recording={recording === action}
                onclick={() => (recording = recording === action ? null : action)}
              >
                {recording === action ? 'Press keys…' : 'Rebind'}
              </button>
            </dd>
          </div>
        {/each}
      </dl>
      <button class="button reset" onclick={() => shelf.resetShortcuts()}>Reset shortcuts</button>
    </section>

    <section>
      <div class="section-head">
        <div>
          <h3>Tracking</h3>
          <p>What Shelf records, and when it records it.</p>
        </div>
      </div>
      <div class="tracking">
        <div class="track-row">
          <div>
            <strong>Learn from opens</strong>
            <p>Counts local opens for Frecency. Turning this off stops new counts.</p>
          </div>
          <button
            class="switch"
            class:on={shelf.trackOpens}
            role="switch"
            aria-checked={shelf.trackOpens}
            aria-label="Learn from opens"
            onclick={() => shelf.setTrackOpens(!shelf.trackOpens)}
          >
            <span></span>
          </button>
        </div>
        <div class="track-row">
          <div>
            <strong>Recent</strong>
            <p>New files only. Editing or opening a file never moves it into Recent; source code is excluded.</p>
          </div>
          <span class="track-state">Automatic</span>
        </div>
        <div class="track-row">
          <div>
            <strong>Origin</strong>
            <p>Read from file xattrs and messaging-app filenames. Shelf does not read browser history.</p>
          </div>
          <span class="track-state">Local only</span>
        </div>
      </div>
      <button class="button" onclick={() => shelf.clearActivity()}>Reset open activity</button>
    </section>

    <section class="privacy">
      <div class="section-head">
        <div>
          <h3>Permissions & privacy</h3>
          <p>
            Everything is local. Shelf does not upload filenames, thumbnails or metadata. It reads
            files only to generate previews you request, and writes only to its own database,
            thumbnail cache and Trash.
          </p>
        </div>
      </div>
    </section>
  </div>
</div>

<style>
  .settings {
    height: 100%;
    overflow: hidden;
    background: var(--surface-content);
  }

  .settings-scroll {
    height: 100%;
    overflow-y: auto;
    width: min(760px, 100%);
    margin: 0 auto;
    padding: 30px 28px 80px;
    display: flex;
    flex-direction: column;
    gap: 30px;
  }

  .text-size {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 12px;
  }

  .text-size > span {
    width: 68px;
    color: var(--ink-tertiary);
    font-size: 11.5px;
  }

  .theme-choice {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--line-hairline);
    border-radius: 8px;
    background: var(--surface-window);
  }

  .theme-button {
    min-width: 78px;
    height: 30px;
    padding: 0 10px;
    border-radius: 6px;
    color: var(--ink-tertiary);
    font-size: 11.5px;
    transition:
      background 120ms var(--ease-hover),
      color 120ms var(--ease-hover);
  }

  .theme-button:hover {
    color: var(--ink-primary);
  }

  .theme-button.active {
    background: var(--primary);
    color: var(--on-primary);
  }

  .theme-file {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-top: 18px;
    padding-top: 16px;
    border-top: var(--hairline);
  }

  .theme-file-copy {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 3px;
  }

  .theme-file-copy > span {
    font-size: 11.5px;
    font-weight: 500;
  }

  .theme-file-copy code {
    color: var(--ink-secondary);
    font-size: 10.5px;
  }

  .theme-file-copy p {
    margin: 2px 0 0;
    max-width: 52ch;
    color: var(--ink-tertiary);
    font-size: 11px;
    line-height: 16px;
  }

  .theme-file-actions {
    display: flex;
    gap: 6px;
    flex: 0 0 auto;
  }

  .head h2 {
    margin: 0;
    font-size: 21px;
    line-height: 27px;
    font-weight: 620;
    letter-spacing: -0.02em;
  }

  .head p {
    margin: 5px 0 0;
    max-width: 56ch;
    color: var(--ink-secondary);
    font-size: 13px;
    line-height: 19px;
  }

  section {
    padding-top: 22px;
    border-top: var(--hairline);
  }

  .section-head {
    display: flex;
    align-items: flex-start;
    gap: 18px;
    margin-bottom: 14px;
  }

  .section-head > div {
    flex: 1;
    min-width: 0;
  }

  .section-head h3 {
    margin: 0;
    font-size: 13.5px;
    line-height: 19px;
    font-weight: 600;
    letter-spacing: -0.005em;
  }

  .section-head p {
    margin: 4px 0 0;
    max-width: 62ch;
    color: var(--ink-tertiary);
    font-size: 12px;
    line-height: 17px;
  }

  .roots {
    display: grid;
    gap: 4px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
  }

  .roots li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 9px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    color: var(--ink-secondary);
    font-size: 11.5px;
  }

  .roots li span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .root-form {
    display: flex;
    gap: 6px;
    width: min(620px, 100%);
    margin-top: 10px;
  }

  .root-form input {
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

  .root-form input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .root-error {
    margin: 6px 0 0;
    color: var(--danger);
    font-size: 11px;
  }

  .path-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    color: var(--ink-secondary);
  }

  .defaults {
    display: grid;
    border-top: 1px solid var(--line-hairline);
  }

  .default-row {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 50px;
    padding: 7px 2px;
    border-bottom: 1px solid var(--line-hairline);
  }

  .default-copy {
    display: grid;
    flex: 1;
    gap: 1px;
    min-width: 0;
  }

  .default-copy strong {
    font-size: 12px;
    font-weight: 500;
  }

  .default-copy span {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .tracking {
    display: grid;
    border-top: 1px solid var(--line-hairline);
  }

  .track-row {
    display: flex;
    align-items: center;
    gap: 18px;
    min-height: 54px;
    padding: 8px 2px;
    border-bottom: 1px solid var(--line-hairline);
  }

  .track-row > div {
    flex: 1;
    min-width: 0;
  }

  .track-row strong {
    display: block;
    font-size: 12px;
    font-weight: 500;
  }

  .track-row p {
    margin: 2px 0 0;
    max-width: 60ch;
    color: var(--ink-tertiary);
    font-size: 11px;
    line-height: 16px;
  }

  .track-state {
    flex: 0 0 auto;
    color: var(--ink-tertiary);
    font-size: 10px;
  }

  .switch {
    flex: 0 0 auto;
    position: relative;
    width: 34px;
    height: 20px;
    padding: 2px;
    border-radius: 999px;
    background: var(--line-hairline);
    transition: background 140ms var(--ease-hover);
  }

  .switch span {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--surface-raised);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: transform 140ms var(--ease-hover);
  }

  .switch.on {
    background: var(--accent);
  }

  .switch.on span {
    transform: translateX(14px);
  }

  .shortcuts {
    display: grid;
    gap: 2px;
    margin: 0;
  }

  .shortcuts > div {
    display: flex;
    align-items: center;
    min-height: 36px;
    padding: 0 2px;
    border-bottom: 1px solid var(--line-hairline);
  }

  .shortcuts dt {
    flex: 1;
    font-size: 11.5px;
  }

  .shortcuts dd {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }

  .rebind {
    padding: 4px 7px;
    border-radius: 5px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .rebind:hover,
  .rebind.recording {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .reset {
    margin-top: 8px;
  }

  kbd {
    padding: 2px 5px;
    border: 1px solid var(--line-hairline);
    border-radius: 4px;
    color: var(--ink-tertiary);
    font-family: var(--font-ui);
    font-size: 10px;
  }

  .privacy p {
    max-width: 66ch;
  }

  .error {
    padding: 8px 10px;
    border-radius: 7px;
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    color: var(--danger);
    font-size: 11.5px;
  }

  .quiet {
    color: var(--ink-tertiary);
    font-size: 11.5px;
  }

  @media (max-width: 839px) {
    .settings-scroll {
      padding: 16px;
    }

    .default-row {
      flex-wrap: wrap;
      padding: 8px 2px;
    }

    .default-row :global(.select-wrap) {
      width: 100%;
    }

    .default-row :global(.select) {
      width: 100%;
      max-width: none;
    }

  }
</style>
