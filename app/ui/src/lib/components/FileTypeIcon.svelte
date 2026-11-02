<script lang="ts">
  import type { FileKind } from '../ipc';
  import { fileTypeInfo, type FileGlyph } from '../extensions';
  import { loadNativeIcon } from '../nativeIcons';

  type Variant = 'list' | 'tile' | 'inspector';
  type State = 'idle' | 'loading' | 'unavailable';

  let {
    name,
    kind,
    size = 64,
    variant = 'tile',
    status = 'idle',
    path = '',
    mtime = 0,
    load = true,
  }: {
    name: string;
    kind: FileKind;
    size?: number;
    variant?: Variant;
    status?: State;
    path?: string;
    mtime?: number;
    load?: boolean;
  } = $props();

  let info = $derived(fileTypeInfo(name, kind));
  let iconColor = $derived(
    info.family === 'image'
      ? 'var(--icon-image)'
      : info.family === 'audio'
        ? 'var(--icon-audio)'
        : info.family === 'video'
          ? 'var(--icon-video)'
          : info.family === 'archive'
            ? 'var(--icon-archive)'
            : info.family === 'spreadsheet'
              ? 'var(--icon-sheet)'
              : 'var(--icon-ink)',
  );
  let sheetW = $derived(
    Math.round(size * (variant === 'inspector' ? 0.72 : variant === 'list' ? 0.9 : 0.56)),
  );
  let glyphSize = $derived(
    Math.max(variant === 'list' ? 14 : 12, Math.round(sheetW * (variant === 'inspector' ? 0.3 : 0.34))),
  );
  let labelSize = $derived(
    Math.min(variant === 'inspector' ? 13 : 12, Math.max(6, sheetW * 0.12)),
  );
  // Image files do not get a document-sheet frame. The photo glyph is the
  // object itself; a frame made every missing thumbnail look like a blank file.
  let pictureW = $derived(
    Math.round(size * (variant === 'inspector' ? 0.72 : 0.62)),
  );
  // The folder is a wider silhouette than the sheet; keep it visually lighter
  // than video/app artwork so archive cards do not dominate a row.
  let attachedW = $derived(
    info.family === 'archive' ? Math.round(sheetW * 0.76) : sheetW,
  );
  let nativeIcon = $state<string | null>(null);
  let isPackage = $derived(
    ['deb', 'rpm', 'flatpak', 'snap', 'msi'].includes(info.extension),
  );
  let attachedFull = $derived(
    nativeIcon
      ? null
      : info.family === 'archive'
        ? 'folder-file'
        : info.glyph === 'video'
          ? info.extension === 'mp4'
            ? 'mp4-file'
            : 'mp4'
          : info.family === 'application'
            ? isPackage
              ? 'computer'
              : 'rocket-lunch'
            : null,
  );
  let directMask = $derived(
    info.extension === 'pdf'
      ? 'pdf'
      : info.family === 'code'
        ? 'code'
        : info.family === 'spreadsheet'
          ? 'table-csv'
          : info.family === 'document' || info.family === 'unknown'
            ? 'file'
            : null,
  );

  $effect(() => {
    const iconPath = path;
    const stamp = mtime;
    nativeIcon = null;
    if (!load || !iconPath || info.family !== 'application') return;
    let alive = true;
    void loadNativeIcon(iconPath, stamp).then((icon) => {
      if (alive) nativeIcon = icon?.data_url ?? null;
    });
    return () => {
      alive = false;
    };
  });
</script>

<div
  class="ft"
  class:list={variant === 'list'}
  class:inspector={variant === 'inspector'}
  class:loading={status === 'loading'}
  class:unavailable={status === 'unavailable'}
  style:--icon-color={iconColor}
  title={`${info.description}${info.extension ? ` · ${info.extension}` : ''}`}
  aria-hidden="true"
>
  {#if variant === 'list'}
    {#if nativeIcon}
      <img class="mini-native" src={nativeIcon} alt="" />
    {:else if attachedFull}
      <span
        class="mini-attached"
        style:width="{glyphSize}px"
        style:height="{glyphSize}px"
        style:--attached={`url(/icons/${attachedFull}.svg)`}
      ></span>
    {:else}
      {#if directMask}
        <span
          class="mini-attached"
          style:width="{glyphSize}px"
          style:height="{glyphSize}px"
          style:--attached={`url(/icons/${directMask}.svg)`}
        ></span>
      {:else}
        <span class="mini-glyph" style:width="{glyphSize}px" style:height="{glyphSize}px">
          {@render glyphSvg(info.glyph)}
        </span>
      {/if}
    {/if}
  {:else if nativeIcon}
    <div class="attached" style:width="{sheetW}px">
      <img class="native" src={nativeIcon} alt="" style:width="{sheetW}px" style:height="{sheetW}px" />
      {#if info.label}
        <span class="ext" style:font-size="{labelSize}px">{info.label}</span>
      {/if}
      {#if variant === 'inspector'}<span class="desc">{info.description}</span>{/if}
    </div>
  {:else if attachedFull}
    <div class="attached" style:width="{attachedW}px">
      <span
        class="attached-mask"
        style:width="{attachedW}px"
        style:height="{attachedW}px"
        style:--attached={`url(/icons/${attachedFull}.svg)`}
      ></span>
      {#if info.label}
        <span class="ext" style:font-size="{labelSize}px">{info.label}</span>
      {/if}
      {#if variant === 'inspector'}<span class="desc">{info.description}</span>{/if}
    </div>
  {:else}
    <div class="picture" style:width="{pictureW}px">
      <span class="picture-glyph" style:width="{pictureW}px" style:height="{pictureW}px">
        {#if directMask}
          <span class="direct-mask" style:--attached={`url(/icons/${directMask}.svg)`}></span>
        {:else}
          {@render glyphSvg(info.glyph)}
        {/if}
      </span>
      {#if info.label}
        <span class="ext" style:font-size="{labelSize}px">{info.label}</span>
      {/if}
      {#if variant === 'inspector'}<span class="desc">{info.description}</span>{/if}
    </div>
  {/if}
</div>

{#snippet glyphSvg(glyph: FileGlyph)}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    {#if glyph === 'document'}
      <path d="M9 10.5h6M9 13.5h6M9 16.5h4" />
    {:else if glyph === 'spreadsheet'}
      <rect x="3.5" y="4" width="17" height="16" rx="1.5" />
      <path d="M3.5 9.5h17M3.5 15h17M9 4v16M15 4v16" />
    {:else if glyph === 'presentation'}
      <rect x="3.5" y="3.5" width="17" height="12" rx="1.5" />
      <path d="M8.5 20l3.5-4.5L15.5 20M7 8h5M7 11.5h8" />
    {:else if glyph === 'code'}
      <path d="M9 7 4.5 12 9 17M15 7l4.5 5L15 17" />
    {:else if glyph === 'archive'}
      <path d="M4 7h16v13H4zM3 4h18v3H3zM10 11h4" />
    {:else if glyph === 'image'}
      <rect x="3.5" y="4.5" width="17" height="15" rx="1.5" />
      <circle cx="8" cy="9" r="1.2" />
      <path d="m4.5 17 4.8-4.5 3.2 2.8 2.6-2 4.4 3.7" />
    {:else if glyph === 'video'}
      <path d="M8.5 5.5 19 12 8.5 18.5z" fill="currentColor" stroke="none" />
    {:else if glyph === 'audio'}
      <path d="M9.5 18V6l9-2v12" />
      <circle cx="7" cy="18.5" r="2.5" />
      <circle cx="16.5" cy="16.5" r="2.5" />
    {:else if glyph === 'font'}
      <path d="M6.5 20 12 4.5 17.5 20M8.4 14.5h7.2" />
    {:else if glyph === 'database'}
      <ellipse cx="12" cy="5.5" rx="7" ry="3" />
      <path d="M5 5.5v13c0 1.7 3.1 3 7 3s7-1.3 7-3v-13M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3" />
    {:else if glyph === 'application'}
      <rect x="3.5" y="4" width="17" height="16" rx="2" />
      <path d="M3.5 9h17M8 9v11" />
    {:else}
      <path d="M7 3.5h6l4 4v13H7z" />
      <path d="M13 3.5v4h4" />
    {/if}
  </svg>
{/snippet}

<style>
  .ft {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: var(--icon-color);
  }

  .ext {
    position: relative;
    max-width: 90%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 650;
    line-height: 1.1;
    letter-spacing: 0.035em;
    color: var(--ink-secondary);
  }

  .desc {
    position: relative;
    max-width: 90%;
    font-size: 10.5px;
    line-height: 13px;
    color: var(--ink-tertiary);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .list {
    color: var(--icon-color);
  }

  .mini-glyph {
    display: grid;
    place-items: center;
  }

  .mini-native {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }

  .mini-attached,
  .attached-mask {
    display: block;
    background: currentColor;
    mask-image: var(--attached);
    mask-position: center;
    mask-repeat: no-repeat;
    mask-size: contain;
    -webkit-mask-image: var(--attached);
    -webkit-mask-position: center;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-size: contain;
  }

  .attached,
  .picture {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4%;
    color: var(--icon-color);
  }

  .picture-glyph {
    display: grid;
    place-items: center;
  }

  .picture-glyph svg,
  .direct-mask {
    width: 100%;
    height: 100%;
  }

  .direct-mask {
    display: block;
    background: currentColor;
    mask-image: var(--attached);
    mask-position: center;
    mask-repeat: no-repeat;
    mask-size: contain;
    -webkit-mask-image: var(--attached);
    -webkit-mask-position: center;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-size: contain;
  }

  .picture .ext,
  .picture .desc {
    position: relative;
  }

  .native {
    object-fit: contain;
  }

  .attached-mask {
    flex: 0 0 auto;
  }

  .attached .ext,
  .attached .desc {
    position: relative;
  }

  .loading .mini-glyph,
  .loading .picture {
    opacity: 0.5;
  }

  .unavailable .picture::after {
    content: '';
    position: absolute;
    left: 20%;
    right: 20%;
    top: 50%;
    border-top: 1.5px solid color-mix(in srgb, var(--ink-primary) 38%, transparent);
    transform: rotate(-30deg);
    pointer-events: none;
  }

</style>
