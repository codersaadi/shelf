<script lang="ts">
  import type { FileMeta } from '../ipc';
  import { thumbUrl } from '../ipc';
  import { formatDuration } from '../format';
  import { extensionOf } from '../extensions';
  import Icon from './Icon.svelte';
  import FileTypeIcon from './FileTypeIcon.svelte';
  import TextPreview from './TextPreview.svelte';
  import { isTextPreviewable } from '../textSnippets';
  import { imageHasTransparency } from '../imageTransparency';

  let {
    file,
    size = 144,
    eager = false,
    radius = 6,
    load = true,
  }: {
    file: FileMeta;
    size?: number;
    eager?: boolean;
    radius?: number;
    load?: boolean;
  } = $props();

  let failed = $state(false);
  let loaded = $state(false);
  let transparent = $state(false);
  let retry = $state(0);
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  const src = $derived(thumbUrl(file, Math.max(96, size * (window.devicePixelRatio || 1)), retry));
  const previewable = $derived(
    (file.id > 0 || Boolean(file.trash_path)) &&
      (file.kind === 'image' ||
        file.kind === 'video' ||
        file.kind === 'music' ||
        extensionOf(file.name) === 'pdf'),
  );
  const showImage = $derived(previewable && !failed);
  const wanted = $derived(showImage && (load || loaded));
  const previewState = $derived(
    previewable ? (failed ? 'unavailable' : !loaded && load ? 'loading' : 'idle') : 'idle',
  );
  const textPreviewable = $derived(
    file.id > 0 && !previewable && size >= 64 && isTextPreviewable(file),
  );

  $effect(() => {
    void file.path;
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = undefined;
    failed = false;
    loaded = false;
    transparent = false;
    retry = 0;
    return () => {
      if (retryTimer) clearTimeout(retryTimer);
    };
  });

  function onImageLoad(event: Event): void {
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = undefined;
    loaded = true;
    transparent = imageHasTransparency(event.currentTarget as HTMLImageElement);
  }

  function onImageError(): void {
    if (retry >= 2) {
      failed = true;
      return;
    }
    const next = retry + 1;
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = setTimeout(() => {
      retry = next;
      retryTimer = undefined;
    }, next * 150);
  }
</script>

<div class="thumb" class:media={wanted && transparent} style:border-radius="{radius}px">
  {#if wanted}
    <img
      {src}
      alt=""
      loading={eager ? 'eager' : 'lazy'}
      decoding="async"
      crossorigin="anonymous"
      draggable="false"
      class:loaded
      onload={onImageLoad}
      onerror={onImageError}
    />
  {/if}
  {#if !wanted || !loaded}
    <div class="placeholder">
      {#if textPreviewable}
        <TextPreview {file} {size} active={load} />
      {:else}
        <FileTypeIcon
          name={file.name}
          kind={file.kind}
          {size}
          variant="tile"
          status={previewState}
          path={file.path}
          mtime={file.mtime}
          load={load}
        />
      {/if}
    </div>
  {/if}
  {#if file.kind === 'video'}
    <span class="play">
      <Icon name="play" size={10} filled />
      {#if file.duration_ms}<span class="duration">{formatDuration(file.duration_ms)}</span>{/if}
    </span>
  {/if}
</div>

<style>
  .thumb {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    display: grid;
    place-items: center;
    background: transparent;
    transition: transform var(--duration-hover) var(--ease-hover);
  }

  .thumb.media {
    background-color: var(--surface-content);
    background-image: conic-gradient(
      var(--checker-a) 25%,
      var(--checker-b) 0 50%,
      var(--checker-a) 0 75%,
      var(--checker-b) 0
    );
    background-size: 16px 16px;
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    opacity: 0;
    transition: opacity 90ms linear;
    background: transparent;
  }

  img.loaded {
    opacity: 1;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: transparent;
  }

  .play {
    position: absolute;
    right: 6px;
    bottom: 6px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 20px;
    padding: 0 6px;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.58);
    color: #fff;
    font-size: 10px;
    line-height: 20px;
    font-variant-numeric: tabular-nums;
    gap: 3px;
  }

  .duration {
    color: #fff;
  }
</style>
