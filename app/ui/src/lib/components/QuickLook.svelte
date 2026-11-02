<script lang="ts">
  import { trapFocus } from '../focus';
  import { shelf } from '../stores.svelte';
  import { bytes, dateTime, formatDuration } from '../format';
  import { extensionOf } from '../extensions';
  import { fullUrl, thumbUrl } from '../ipc';
  import FileTypeIcon from './FileTypeIcon.svelte';
  import Icon from './Icon.svelte';
  import { smartName } from '../smart';
  import { imageHasTransparency } from '../imageTransparency';

  let { onclose, onstep }: { onclose: () => void; onstep: (delta: number) => void } = $props();
  let file = $derived(shelf.selectedFile);
  let failed = $state(false);
  let loaded = $state(false);
  let transparent = $state(false);
  let src = $state('');
  let fallback = $state(false);
  let retry = $state(0);
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let dimensions = $state<{ width: number; height: number } | null>(null);
  let media = $derived(
    file
      ? file.kind === 'image' ||
          file.kind === 'video' ||
          file.kind === 'music' ||
          extensionOf(file.name) === 'pdf'
      : false,
  );

  $effect(() => {
    const current = file;
    failed = false;
    loaded = false;
    transparent = false;
    fallback = false;
    retry = 0;
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = undefined;
    dimensions = null;
    src = current ? (current.kind === 'image' ? fullUrl(current) : thumbUrl(current, 1200)) : '';
    return () => {
      if (retryTimer) clearTimeout(retryTimer);
    };
  });

  function imageFailed(): void {
    if (!fallback && file) {
      fallback = true;
      retry = 0;
      src = thumbUrl(file, 1200);
      return;
    }
    if (retry < 2 && file) {
      const next = retry + 1;
      if (retryTimer) clearTimeout(retryTimer);
      retryTimer = setTimeout(() => {
        retry = next;
        retryTimer = undefined;
        src = fallback ? thumbUrl(file, 1200, next) : fullUrl(file, next);
      }, next * 150);
      return;
    }
    failed = true;
  }

  function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' || event.key === ' ') {
      event.preventDefault();
      onclose();
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      onstep(1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      onstep(-1);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div
  class="backdrop"
  role="presentation"
  onclick={onclose}
  onkeydown={(e) => e.key === 'Escape' && onclose()}
>
  <div
    class="panel"
    role="dialog"
    tabindex="-1"
    use:trapFocus
    aria-modal="true"
    aria-label={file ? file.name : 'Preview'}
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    {#if file}
      <div class="stage" class:media={media && !failed && transparent}>
        {#if media && !failed}
          <img
            class:loaded
            {src}
            alt=""
            crossorigin="anonymous"
            onload={(event) => {
              const image = event.currentTarget as HTMLImageElement;
              loaded = true;
              dimensions = { width: image.naturalWidth, height: image.naturalHeight };
              transparent = imageHasTransparency(image);
            }}
            onerror={imageFailed}
          />
        {/if}
        {#if media && !failed && !loaded}
          <div class="glyph overlay">
            <div class="preview-icon">
              <FileTypeIcon
                name={file.name}
                kind={file.kind}
                size={180}
                variant="inspector"
                status="loading"
                path={file.path}
                mtime={file.mtime}
              />
            </div>
            <p class="reason">Preparing preview…</p>
          </div>
        {:else if !media || failed}
          <div class="glyph">
            <div class="preview-icon">
              <FileTypeIcon
                name={file.name}
                kind={file.kind}
                size={180}
                variant="inspector"
                status={media ? 'unavailable' : 'idle'}
                path={file.path}
                mtime={file.mtime}
              />
            </div>
            {#if media}
              <p class="reason">Preview unavailable. Open the file to view it.</p>
            {:else}
              <p class="reason">No preview for this file type.</p>
            {/if}
            <button class="button" onclick={() => shelf.openFile(file)}>Open</button>
          </div>
        {/if}
      </div>
      <button class="close" aria-label="Close preview" onclick={onclose}>
        <Icon name="close" size={15} />
      </button>
      <button class="nav prev" aria-label="Previous file" onclick={() => onstep(-1)}>
        <Icon name="chevron-left" size={20} />
      </button>
      <button class="nav next" aria-label="Next file" onclick={() => onstep(1)}>
        <Icon name="chevron-right" size={20} />
      </button>

      <footer>
        <div class="names">
          <p class="name" title={file.path}>{smartName(file).title}</p>
          <p class="meta">
            {#if smartName(file).detail}{smartName(file).detail} · {/if}{bytes(file.size)}{dimensions && file.kind === 'image' ? ` · ${dimensions.width} × ${dimensions.height}` : ''}{file.duration_ms ? ` · ${formatDuration(file.duration_ms)}` : ''} · Modified {dateTime(file.mtime)}
          </p>
        </div>
        <div class="actions">
          <button class="button" onclick={() => shelf.showInFiles([file.path])}>Show in folder</button>
          <button class="button primary" onclick={() => shelf.openFile(file)}>Open</button>
        </div>
      </footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 5vh 5vw;
    background: color-mix(in srgb, #000 45%, transparent);
    animation: fade 180ms var(--ease-reveal);
  }

  .panel {
    width: min(1100px, 90vw);
    height: min(760px, 80vh);
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
    border: 1px solid var(--line-hairline);
    overflow: hidden;
  }

  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    background: var(--surface-window);
    padding: 16px;
  }

  .stage.media {
    background-color: var(--surface-content);
    background-image: conic-gradient(
      var(--checker-a) 25%,
      var(--checker-b) 0 50%,
      var(--checker-a) 0 75%,
      var(--checker-b) 0
    );
    background-size: 24px 24px;
  }

  .stage img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: var(--radius-thumb);
    display: block;
    opacity: 0;
    transition: opacity 90ms linear;
  }

  .stage img.loaded {
    opacity: 1;
  }

  .close,
  .nav {
    position: absolute;
    z-index: 3;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--surface-raised) 92%, transparent);
    color: var(--ink-secondary);
    border: 1px solid var(--line-hairline);
    box-shadow: var(--shadow-popover);
  }

  .close {
    top: 12px;
    right: 12px;
  }

  .nav {
    top: 50%;
    transform: translateY(-50%);
  }

  .nav.prev {
    left: 12px;
  }

  .nav.next {
    right: 12px;
  }

  .close:hover,
  .nav:hover {
    color: var(--ink-primary);
    background: var(--surface-raised);
  }

  .glyph.overlay {
    position: absolute;
    inset: 16px;
    background: var(--surface-content);
    place-content: center;
  }

  .glyph {
    display: grid;
    justify-items: center;
    gap: 10px;
    color: var(--ink-tertiary);
    max-width: 44ch;
    text-align: center;
  }

  .preview-icon {
    width: 180px;
    height: 210px;
  }

  .reason {
    margin: 0;
    font-size: 12px;
    line-height: 17px;
    color: var(--ink-secondary);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-top: var(--hairline);
  }

  .names {
    min-width: 0;
  }

  .name {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60vw;
  }

  .meta {
    margin: 2px 0 0;
    font-size: 11.5px;
    color: var(--ink-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  @media (max-width: 620px) {
    .actions .button:first-child {
      display: none;
    }

    .nav {
      display: none;
    }
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
