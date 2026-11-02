<script lang="ts">
  type IconName =
    | 'home'
    | 'recent'
    | 'download'
    | 'screenshot'
    | 'image'
    | 'video'
    | 'music'
    | 'document'
    | 'archive'
    | 'app'
    | 'folder'
    | 'trash'
    | 'clean'
    | 'search'
    | 'grid'
    | 'list'
    | 'sort'
    | 'info'
    | 'sidebar'
    | 'close'
    | 'chevron-down'
    | 'chevron-left'
    | 'chevron-right'
    | 'undo'
    | 'copy'
    | 'external'
    | 'check'
    | 'play'
    | 'plus'
    | 'minus'
    | 'more'
    | 'tag'
    | 'settings'
    | 'terminal'
    | 'edit';

  import CommonIcon from './CommonIcon.svelte';

  let {
    name,
    size = 16,
    stroke = 1.5,
    filled = false,
  }: { name: IconName; size?: number; stroke?: number; filled?: boolean } = $props();

  const COMMON: Partial<Record<IconName, string>> = {
    home: 'home',
    recent: 'time',
    screenshot: 'camera',
    image: 'image-gallery',
    video: 'videos',
    music: 'music-player',
    archive: 'archive',
    app: 'app',
    folder: 'folder',
    trash: 'delete',
    clean: 'broom',
    search: 'search',
    list: 'list',
    sort: 'sort-up',
    info: 'info',
    sidebar: 'sidebar',
    close: 'close',
    'chevron-left': 'back',
    'chevron-right': 'next',
    copy: 'copy',
    check: 'check',
    play: 'play-button-filled',
    tag: 'bookmark-white',
    settings: 'settings',
  };
</script>

{#if COMMON[name]}
  <CommonIcon name={COMMON[name] as string} {size} />
{:else}
<svg
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill={filled ? 'currentColor' : 'none'}
  stroke={filled ? 'none' : 'currentColor'}
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === 'recent'}
    <circle cx="8" cy="8" r="6" />
    <path d="M8 4.5V8l2.6 1.6" />
  {:else if name === 'download'}
    <path d="M8 2v7.5M8 9.5 5.2 6.7M8 9.5l2.8-2.8" />
    <path d="M2.5 12.5v1h11v-1" />
  {:else if name === 'screenshot'}
    <path d="M5 2H2v3M11 2h3v3M14 11v3h-3M5 14H2v-3" />
    <circle cx="8" cy="8" r="2.2" />
  {:else if name === 'image'}
    <rect x="2" y="3" width="12" height="10" rx="2" />
    <circle cx="5.5" cy="6.5" r="1" />
    <path d="m2.5 12 3.6-3.2 2.4 2 1.9-1.4 3.1 2.6" />
  {:else if name === 'video'}
    <rect x="2" y="4" width="12" height="8" rx="2" />
    <path d="m6.8 6.3 3 1.7-3 1.7z" fill="currentColor" stroke="none" />
  {:else if name === 'music'}
    <path d="M6 12V4l7-1.8V10" />
    <circle cx="4.3" cy="12.4" r="1.7" />
    <circle cx="11.3" cy="10.4" r="1.7" />
  {:else if name === 'document'}
    <path d="M4 1.8h5l3 3v9.4H4z" />
    <path d="M9 1.8v3h3M6 8h4M6 10.5h4" />
  {:else if name === 'archive'}
    <path d="M2.8 2.8h10.4v3H2.8zM3.8 5.8v7.4h8.4V5.8M6.5 8.5h3" />
  {:else if name === 'app'}
    <rect x="2.2" y="2.2" width="11.6" height="11.6" rx="2.4" />
    <path d="M2.2 6h11.6M6 6v7.8" />
  {:else if name === 'folder'}
    <path d="M2 4h4.2L7.6 6H14v7.2H2z" />
  {:else if name === 'trash'}
    <path d="M2.5 4h11M5.8 4V2.4h4.4V4M4.2 4l.7 9.6h6.2L11.8 4" />
    <path d="M6.6 6.6v4.6M9.4 6.6v4.6" />
  {:else if name === 'clean'}
    <path d="M2.6 13.4 8 8M6.6 3.4l4 4M9.6 6.4l3.2-3.2M3.4 9.6l3 3" />
  {:else if name === 'search'}
    <circle cx="7" cy="7" r="4.4" />
    <path d="m10.3 10.3 3.2 3.2" />
  {:else if name === 'grid'}
    <rect x="2.2" y="2.2" width="4.8" height="4.8" rx="1" />
    <rect x="9" y="2.2" width="4.8" height="4.8" rx="1" />
    <rect x="2.2" y="9" width="4.8" height="4.8" rx="1" />
    <rect x="9" y="9" width="4.8" height="4.8" rx="1" />
  {:else if name === 'list'}
    <path d="M5.6 3.6h8.2M5.6 8h8.2M5.6 12.4h8.2" />
    <circle cx="2.6" cy="3.6" r="0.9" fill="currentColor" stroke="none" />
    <circle cx="2.6" cy="8" r="0.9" fill="currentColor" stroke="none" />
    <circle cx="2.6" cy="12.4" r="0.9" fill="currentColor" stroke="none" />
  {:else if name === 'sort'}
    <path d="M4.2 2.8v10.4M4.2 13.2 2.2 11.2M4.2 13.2l2-2M9 4h4.8M9 8h3.6M9 12h2.4" />
  {:else if name === 'info'}
    <circle cx="8" cy="8" r="6" />
    <path d="M8 7.2v4M8 4.8v.4" />
  {:else if name === 'sidebar'}
    <rect x="2" y="2.6" width="12" height="10.8" rx="2" />
    <path d="M6 2.6v10.8" />
  {:else if name === 'close'}
    <path d="m3.2 3.2 9.6 9.6M12.8 3.2 3.2 12.8" />
  {:else if name === 'chevron-down'}
    <path d="m4.2 6.2 3.8 3.8 3.8-3.8" />
  {:else if name === 'chevron-left'}
    <path d="M9.8 3.2 5 8l4.8 4.8" />
  {:else if name === 'chevron-right'}
    <path d="M6.2 3.2 11 8l-4.8 4.8" />
  {:else if name === 'undo'}
    <path d="M3 6.2h6.2a3.8 3.8 0 0 1 0 7.6H6.4" />
    <path d="M5.8 3.4 3 6.2l2.8 2.8" />
  {:else if name === 'copy'}
    <rect x="5.2" y="5.2" width="8.4" height="8.4" rx="1.6" />
    <path d="M10.8 5.2V3.6A1.6 1.6 0 0 0 9.2 2H3.6A1.6 1.6 0 0 0 2 3.6v5.6a1.6 1.6 0 0 0 1.6 1.6h1.6" />
  {:else if name === 'external'}
    <path d="M9.2 2.4h4.4v4.4M13.6 2.4 7.2 8.8" />
    <path d="M12 9.4v3.4H3.2V4h3.4" />
  {:else if name === 'check'}
    <path d="m3 8.2 3.4 3.4 6.6-6.8" />
  {:else if name === 'play'}
    <path d="M5 3.2 12.4 8 5 12.8z" />
  {:else if name === 'plus'}
    <path d="M8 3v10M3 8h10" />
  {:else if name === 'minus'}
    <path d="M3 8h10" />
  {:else if name === 'more'}
    <circle cx="3.2" cy="8" r="1" fill="currentColor" stroke="none" />
    <circle cx="8" cy="8" r="1" fill="currentColor" stroke="none" />
    <circle cx="12.8" cy="8" r="1" fill="currentColor" stroke="none" />
  {:else if name === 'tag'}
    <path d="M8.6 2H2v6.6l5.6 5.6 6.6-6.6z" />
    <circle cx="5.2" cy="5.2" r="1" />
  {:else if name === 'terminal'}
    <rect x="2" y="3" width="12" height="10" rx="1.8" />
    <path d="m4.8 6.2 2 1.8-2 1.8M8.4 10.2h3" />
  {:else if name === 'edit'}
    <path d="M3 13h2.6l7.2-7.2a1.8 1.8 0 0 0-2.6-2.6L3 10.4z" />
    <path d="M2.5 14.5h11" />
  {/if}
</svg>
{/if}
