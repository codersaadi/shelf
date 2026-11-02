<script lang="ts">
  import { shelf } from '../stores.svelte';
  import Icon from './Icon.svelte';

  let path = $derived(shelf.folderInfo?.path ?? shelf.currentFolder ?? '');
  let parts = $derived(path.split('/').filter(Boolean));
  let parent = $derived.by(() => {
    if (!path || path === '/') return null;
    const index = path.lastIndexOf('/');
    return index > 0 ? path.slice(0, index) : '/';
  });

  function goTo(index: number): void {
    const target = '/' + parts.slice(0, index + 1).join('/');
    if (target !== path) shelf.openLocation(target);
  }
</script>

{#if path}
  <div class="folder-bar">
    <button
      class="up"
      aria-label="Go up one folder"
      title="Up one folder"
      disabled={!parent}
      onclick={() => parent && shelf.openLocation(parent)}
    >
      <Icon name="chevron-left" size={14} />
    </button>
    <nav aria-label="Folder path">
      <button class="crumb root" title="/" onclick={() => shelf.openLocation('/')}>
        <Icon name="home" size={12} />
      </button>
      {#each parts as part, index (index)}
        <span class="slash">/</span>
        <button
          class="crumb"
          class:current={index === parts.length - 1}
          title={'/' + parts.slice(0, index + 1).join('/')}
          onclick={() => goTo(index)}
        >
          {part}
        </button>
      {/each}
    </nav>
  </div>
{/if}

<style>
  .folder-bar {
    display: flex;
    flex: 0 0 34px;
    align-items: center;
    gap: 3px;
    min-width: 0;
    padding: 0 16px;
    border-bottom: var(--hairline);
    background: var(--surface-content);
  }

  .up {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 5px;
    color: var(--ink-tertiary);
  }

  .up:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .up:disabled {
    opacity: 0.3;
    cursor: default;
  }

  nav {
    display: flex;
    align-items: center;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  nav::-webkit-scrollbar {
    display: none;
  }

  .crumb {
    flex: 0 0 auto;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 3px 5px;
    border-radius: 5px;
    color: var(--ink-tertiary);
  }

  .crumb:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .crumb.current {
    color: var(--ink-secondary);
  }

  .crumb.root {
    display: grid;
    place-items: center;
    width: 22px;
    padding: 0;
  }

  .slash {
    flex: 0 0 auto;
    opacity: 0.5;
  }
</style>
