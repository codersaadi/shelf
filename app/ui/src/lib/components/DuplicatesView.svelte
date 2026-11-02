<script lang="ts">
  import { onMount } from 'svelte';
  import { shelf } from '../stores.svelte';
  import { bytes, dateTime, middleTruncate, parentPath, plural } from '../format';
  import FileTypeIcon from './FileTypeIcon.svelte';
  import Icon from './Icon.svelte';

  let checked = $state<Record<string, boolean>>({});

  let selected = $derived(Object.keys(checked).filter((path) => checked[path]));
  let wasted = $derived(shelf.duplicates.reduce((n, group) => n + group.wasted, 0));
  let copies = $derived(shelf.duplicates.reduce((n, group) => n + group.files.length, 0));
  let selectedBytes = $derived(
    shelf.duplicates
      .flatMap((group) => group.files)
      .filter((file) => checked[file.path])
      .reduce((n, file) => n + file.size, 0),
  );

  onMount(() => {
    void shelf.loadDuplicates();
  });

  function toggle(path: string): void {
    checked = { ...checked, [path]: !checked[path] };
  }

  function selectGroup(index: number): void {
    const group = shelf.duplicates[index];
    if (!group) return;
    const next = { ...checked };
    for (const file of group.files.slice(1)) next[file.path] = true;
    checked = next;
  }

  function selectAll(): void {
    const next: Record<string, boolean> = {};
    for (const group of shelf.duplicates) {
      for (const file of group.files.slice(1)) next[file.path] = true;
    }
    checked = next;
  }

  function moveCopies(): void {
    if (selected.length) shelf.requestTrash(selected, selectedBytes);
  }
</script>

<div class="duplicates">
  <header>
    <div>
      <h2>Duplicates</h2>
      <p>Identical file contents. Keep the first copy in each group, or choose your own.</p>
    </div>
    {#if shelf.duplicates.length}
      <span class="summary">{copies} {plural(copies, 'copy')} · {bytes(wasted)} recoverable</span>
    {/if}
  </header>

  {#if shelf.duplicatesLoading}
    <div class="state"><span class="dot"></span>Hashing candidates…</div>
  {:else if shelf.duplicates.length === 0}
    <div class="state">
      <Icon name="check" size={18} />
      <h3>No duplicates</h3>
      <p>No identical files were found in the index.</p>
    </div>
  {:else}
    <div class="actions">
      <button class="button" onclick={selectAll}>Select all copies</button>
      <button class="button" onclick={() => (checked = {})}>Clear</button>
      <button class="button primary" disabled={!selected.length} onclick={moveCopies}>
        Move {selected.length} to Trash
      </button>
    </div>

    <div class="groups">
      {#each shelf.duplicates as group, index (group.files[0]?.path ?? index)}
        <section class="group">
          <div class="group-head">
            <div>
              <strong>{bytes(group.size)}</strong>
              <span>{group.files.length} copies · {bytes(group.wasted)} recoverable</span>
            </div>
            <button class="link" onclick={() => selectGroup(index)}>Select copies</button>
          </div>
          <div class="files">
            {#each group.files as file, fileIndex (file.path)}
              {@const keeper = fileIndex === 0}
              <label class="file" class:keeper>
                <input
                  type="checkbox"
                  checked={keeper ? false : Boolean(checked[file.path])}
                  disabled={keeper}
                  onchange={() => toggle(file.path)}
                />
                <FileTypeIcon
                  name={file.name}
                  kind={file.kind}
                  size={18}
                  variant="list"
                  path={file.path}
                  mtime={file.mtime}
                />
                <span class="copy">
                  <span class="name" title={file.path}>{middleTruncate(file.name, 56)}</span>
                  <span class="path">{parentPath(file.path)}</span>
                </span>
                {#if keeper}<span class="keep">Keep</span>{/if}
                <span class="date">{dateTime(file.mtime)}</span>
              </label>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .duplicates {
    height: 100%;
    overflow-y: auto;
    padding: 26px 28px 90px;
    background: var(--surface-content);
  }

  header {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 18px;
  }

  header h2 {
    margin: 0;
    font-size: 18px;
    line-height: 24px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }

  header p {
    margin: 4px 0 0;
    color: var(--ink-tertiary);
    font-size: 12px;
    line-height: 17px;
  }

  .summary {
    margin-left: auto;
    color: var(--ink-tertiary);
    font-size: 11px;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-bottom: 14px;
  }

  .groups {
    display: grid;
    gap: 12px;
  }

  .group {
    border: 1px solid var(--line-hairline);
    border-radius: 9px;
    background: var(--surface-raised);
    overflow: hidden;
  }

  .group-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--line-hairline);
  }

  .group-head > div {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .group-head strong {
    font-size: 12px;
    font-weight: 600;
  }

  .group-head span {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .link {
    margin-left: auto;
    color: var(--accent);
    font-size: 10.5px;
  }

  .files {
    display: grid;
  }

  .file {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 38px;
    padding: 4px 12px;
    border-bottom: 1px solid var(--line-hairline);
    cursor: pointer;
  }

  .file:last-child {
    border-bottom: 0;
  }

  .file:hover {
    background: var(--surface-hover);
  }

  .file.keeper {
    cursor: default;
    opacity: 0.7;
  }

  input[type='checkbox'] {
    width: 13px;
    height: 13px;
    accent-color: var(--accent);
  }

  .copy {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 1px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-tertiary);
    font-size: 9.5px;
  }

  .keep {
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    color: var(--ink-tertiary);
    font-size: 9px;
    text-transform: uppercase;
  }

  .date {
    flex: 0 0 auto;
    color: var(--ink-tertiary);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .state {
    display: grid;
    place-items: center;
    gap: 5px;
    padding: 54px 20px;
    color: var(--ink-tertiary);
    text-align: center;
    font-size: 12px;
  }

  .state h3 {
    margin: 4px 0 0;
    color: var(--ink-primary);
    font-size: 13px;
    font-weight: 600;
  }

  .state p {
    margin: 0;
    font-size: 11px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1s infinite alternate;
  }

  @keyframes pulse {
    to {
      opacity: 0.25;
    }
  }
</style>
