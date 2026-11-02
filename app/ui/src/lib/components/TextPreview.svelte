<script lang="ts">
  import type { FileMeta, TextSnippet } from '../ipc';
  import { loadTextSnippet } from '../textSnippets';
  import FileTypeIcon from './FileTypeIcon.svelte';

  let {
    file,
    size = 160,
    active = true,
  }: { file: FileMeta; size?: number; active?: boolean } = $props();

  let snippet = $state<TextSnippet | null>(null);
  let failed = $state(false);
  let loading = $state(false);

  $effect(() => {
    const path = file.path;
    const mtime = file.mtime;
    if (!active) {
      loading = false;
      return;
    }
    let alive = true;
    loading = true;
    failed = false;
    snippet = null;
    void loadTextSnippet(file).then((result) => {
      if (!alive) return;
      snippet = result;
      failed = result === null;
      loading = false;
    });
    return () => {
      alive = false;
      void path;
      void mtime;
    };
  });
</script>

{#if snippet}
  <div
    class="paper"
    class:markdown={snippet.html}
    style:--preview-font="{Math.min(11.5, Math.max(7, size * 0.062))}px"
    aria-hidden="true"
  >
    {#if snippet.html}
      <div class="md-body">{@html snippet.html}</div>
    {:else}
      <p class="text-body">{snippet.text}</p>
    {/if}
    {#if snippet.truncated}<span class="fade"></span>{/if}
    <span class="fold"></span>
  </div>
{:else}
  <FileTypeIcon
    name={file.name}
    kind={file.kind}
    {size}
    variant="tile"
    status={loading && !failed ? 'loading' : 'idle'}
    path={file.path}
    mtime={file.mtime}
  />
{/if}

<style>
  .paper {
    position: relative;
    display: block;
    width: 86%;
    height: 88%;
    overflow: hidden;
    padding: 11% 10% 13%;
    border: 1px solid var(--line-hairline);
    border-radius: 2px;
    background: var(--surface-raised);
    color: var(--ink-secondary);
    font-size: var(--preview-font);
    line-height: 1.38;
    text-align: left;
  }

  .text-body {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .md-body {
    max-height: 100%;
    overflow: hidden;
  }

  .md-body :global(p) {
    margin: 0 0 0.55em;
  }

  .md-body :global(.md-h) {
    margin: 0 0 0.5em;
    color: var(--ink-primary);
    font-weight: 700;
    font-size: 1.18em;
    line-height: 1.2;
  }

  .md-body :global(strong) {
    color: var(--ink-primary);
    font-weight: 650;
  }

  .md-body :global(em) {
    font-style: italic;
  }

  .md-body :global(code) {
    padding: 0.1em 0.25em;
    border-radius: 2px;
    background: var(--surface-hover);
    font-family: var(--font-mono);
    font-size: 0.9em;
  }

  .md-body :global(pre) {
    margin: 0 0 0.55em;
    overflow: hidden;
    font-family: var(--font-mono);
    font-size: 0.9em;
    white-space: pre-wrap;
  }

  .md-body :global(ul) {
    margin: 0 0 0.55em;
    padding-left: 1.1em;
  }

  .md-body :global(li) {
    margin: 0;
  }

  .md-body :global(blockquote) {
    margin: 0 0 0.55em;
    padding-left: 0.6em;
    border-left: 2px solid var(--line-hairline);
    color: var(--ink-tertiary);
  }

  .md-body :global(hr) {
    margin: 0.5em 0;
    border: 0;
    border-top: 1px solid var(--line-hairline);
  }

  .md-body :global(.md-img) {
    color: var(--ink-tertiary);
  }

  .fade {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 28%;
    background: linear-gradient(to bottom, transparent, var(--surface-raised) 85%);
    pointer-events: none;
  }

  .fold {
    position: absolute;
    top: 0;
    right: 0;
    width: 0;
    height: 0;
    border-bottom: 13px solid color-mix(in srgb, var(--ink-primary) 7%, var(--surface-raised));
    border-left: 13px solid transparent;
  }
</style>
