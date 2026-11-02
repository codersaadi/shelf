<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { middleTruncate, relativeDate } from '../format';
  import Icon from './Icon.svelte';
  import { dragScroll } from '../dragScroll';
  import type { FolderEntry } from '../ipc';

  let { oncontext }: { oncontext?: (folder: FolderEntry, event: MouseEvent) => void } = $props();
  let expanded = $state(localStorage.getItem('shelf.folders.expanded') === '1');

  function toggleExpanded(): void {
    expanded = !expanded;
    try {
      localStorage.setItem('shelf.folders.expanded', expanded ? '1' : '0');
    } catch {
      // Preference is optional.
    }
  }
</script>

{#if shelf.folders.length}
  <section class="folders" class:expanded={expanded} aria-labelledby="download-folders">
    <div class="head">
      <h2 id="download-folders">Folders</h2>
      <span>{shelf.folders.length}</span>
      <button
        class="expand"
        aria-pressed={expanded}
        aria-label={expanded ? 'Collapse folders' : 'Expand all folders'}
        onclick={toggleExpanded}
      >
        <Icon name={expanded ? 'chevron-down' : 'grid'} size={13} />
      </button>
    </div>
    <div class="row" class:expanded use:dragScroll aria-label="Folders">
      {#each shelf.folders as folder (folder.path)}
        <button
          class="folder"
          title={folder.path}
          onclick={() => shelf.openLocation(folder.path)}
          oncontextmenu={(e) => {
            e.preventDefault();
            oncontext?.(folder, e);
          }}
        >
          <span class="folder-icon"><Icon name="folder" size={17} /></span>
          <span class="copy">
            <span class="name">{middleTruncate(folder.name, 28)}</span>
            <span class="meta">{relativeDate(folder.mtime)}</span>
          </span>
          <span class="enter" aria-hidden="true"><Icon name="chevron-right" size={12} /></span>
        </button>
      {/each}
    </div>
  </section>
{/if}

<style>
  .folders {
    flex: 0 0 auto;
    padding: 10px 16px 8px;
    border-bottom: var(--hairline);
    background: var(--surface-content);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin-bottom: 6px;
  }

  .head h2 {
    margin: 0;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    font-weight: 550;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .head span {
    color: var(--ink-tertiary);
    font-size: 10px;
  }

  .expand {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-left: auto;
    border-radius: 5px;
    color: var(--ink-tertiary);
  }

  .expand:hover,
  .expand[aria-pressed='true'] {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .row {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    cursor: grab;
    padding-bottom: 2px;
    scrollbar-width: none;
  }

  .row::-webkit-scrollbar {
    display: none;
  }

  .row.expanded {
    flex-wrap: wrap;
    overflow: visible;
    max-height: 182px;
    overflow-y: auto;
    cursor: default;
  }

  .folders.expanded {
    max-height: 240px;
  }

  .folder {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    width: 190px;
    min-height: 46px;
    padding: 5px 8px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-raised);
    text-align: left;
  }

  .folder:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
    border-color: color-mix(in srgb, var(--accent) 28%, var(--line-hairline));
  }

  .folder-icon {
    display: grid;
    place-items: center;
    flex: 0 0 28px;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--surface-hover);
    color: var(--icon-ink);
  }

  .copy {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 1px;
  }

  .enter {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    color: var(--ink-tertiary);
    opacity: 0;
    transition: opacity 120ms var(--ease-hover);
  }

  .folder:hover .enter {
    opacity: 1;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
  }

  .meta {
    color: var(--ink-tertiary);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }
</style>
