<script lang="ts">
  import type { FileMeta } from '../ipc';
  import { bytes, middleTruncate, relativeDate } from '../format';
  import Thumb from './Thumb.svelte';
  import { fileTypeInfo } from '../extensions';
  import { formatDuration } from '../format';
  import { startFileDrag } from '../fileDrag';
  import { dateGroup } from '../dateGroups';
  import { smartName } from '../smart';

  let {
    items,
    cell = 144,
    selected = [],
    columns = $bindable(1),
    onselect,
    onopen,
    onloadmore,
    oncontext,
    onselectpaths,
    grouped = false,
    dateField = 'mtime',
  }: {
    items: FileMeta[];
    cell?: number;
    selected?: string[];
    columns?: number;
    onselect: (file: FileMeta, event: MouseEvent) => void;
    onopen: (file: FileMeta) => void;
    onloadmore: () => void;
    oncontext?: (file: FileMeta, event: MouseEvent) => void;
    onselectpaths?: (paths: string[], additive: boolean) => void;
    grouped?: boolean;
    dateField?: 'mtime' | 'added_at';
  } = $props();

  const GAP = 12;
  const PAD = 24;
  const LABEL = 30;
  const GROUP = 30;

  let scroller: HTMLDivElement | null = $state(null);
  let width = $state(800);
  let height = $state(600);
  let scrollTop = $state(0);
  let loadingMore = $state(false);
  let ticking = false;

  let cols = $derived(Math.max(1, Math.floor((width - PAD * 2 + GAP) / (cell + GAP))));
  // Like the prototype's `minmax(cell, 1fr)`: the slider sets the minimum, and
  // the last column stretches so the grid fills the content width.
  let slot = $derived((width - PAD * 2 - GAP * (cols - 1)) / Math.max(1, cols));
  let tileW = $derived(Math.max(64, Math.floor(slot)));
  let tileH = $derived(tileW + LABEL + GAP);
  type GridRow =
    | { kind: 'group'; label: string; y: number }
    | { kind: 'tiles'; items: { file: FileMeta; index: number }[]; y: number };

  let layout = $derived.by(() => {
    const groups: { label: string; items: { file: FileMeta; index: number }[] }[] = [];
    items.forEach((file, index) => {
      const label = grouped ? dateGroup(file[dateField]) : '';
      if (groups.length === 0 || groups[groups.length - 1].label !== label) {
        groups.push({ label, items: [] });
      }
      groups[groups.length - 1].items.push({ file, index });
    });
    const rows: GridRow[] = [];
    let y = PAD;
    for (const group of groups) {
      if (group.label) {
        rows.push({ kind: 'group', label: group.label, y });
        y += GROUP;
      }
      for (let i = 0; i < group.items.length; i += cols) {
        rows.push({ kind: 'tiles', items: group.items.slice(i, i + cols), y });
        y += tileH;
      }
    }
    return { rows, height: Math.max(y + PAD, height) };
  });
  let totalH = $derived(layout.height);
  let overscan = $derived(Math.max(1, Math.ceil(height / tileH)));
  let visibleRows = $derived(
    layout.rows.filter(
      (row) =>
        row.y + (row.kind === 'group' ? GROUP : tileH) >= scrollTop - overscan * tileH &&
        row.y <= scrollTop + height + overscan * tileH,
    ),
  );
  let selectedSet = $derived(new Set(selected));
  let marquee = $state<{ x1: number; y1: number; x2: number; y2: number } | null>(null);

  type DragState = {
    x: number;
    y: number;
    additive: boolean;
    onTile: boolean;
    active: boolean;
    pointerId: number;
  };
  let drag: DragState | null = null;
  let suppressClick = false;
  let skipNextScroll = false;
  let lastScrolledPath = '';

  $effect(() => {
    columns = cols;
  });

  $effect(() => {
    if (!scroller) return;
    const el = scroller;
    const ro = new ResizeObserver(() => {
      width = el.clientWidth;
      height = el.clientHeight;
    });
    ro.observe(el);
    width = el.clientWidth;
    height = el.clientHeight;
    return () => ro.disconnect();
  });

  $effect(() => {
    // A new view can be shorter than the old scroll offset; never leave the
    // viewport past the end of the list.
    void totalH;
    if (scroller) {
      const max = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
      if (scroller.scrollTop > max) scroller.scrollTop = max;
    }
  });

  $effect(() => {
    if (marquee || !scroller) return;
    const path = selected[0];
    if (!path) {
      lastScrolledPath = '';
      skipNextScroll = false;
      return;
    }
    if (path === lastScrolledPath) return;
    lastScrolledPath = path;
    if (skipNextScroll) {
      skipNextScroll = false;
      return;
    }
    const row = layout.rows.find(
      (r) => r.kind === 'tiles' && r.items.some((item) => item.file.path === path),
    );
    if (!row) return;
    const top = row.y;
    const bottom = top + tileH;
    if (top < scroller.scrollTop) scroller.scrollTop = Math.max(0, top - PAD);
    else if (bottom > scroller.scrollTop + scroller.clientHeight) {
      scroller.scrollTop = bottom - scroller.clientHeight + PAD;
    }
  });

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0 || event.pointerType !== 'mouse' || !scroller) return;
    const rect = scroller.getBoundingClientRect();
    if (event.clientX > rect.right - 14) return; // scrollbar
    const target = event.target as HTMLElement;
    drag = {
      x: event.clientX,
      y: event.clientY,
      additive: event.shiftKey || event.ctrlKey || event.metaKey,
      onTile: Boolean(target.closest('.hit')),
      active: false,
      pointerId: event.pointerId,
    };
  }

  function onPointerMove(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.pointerId) return;
    if (!drag.active) {
      if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 6) return;
      drag.active = true;
      suppressClick = true;
      document.body.classList.add('selecting');
      if (!drag.additive) onselectpaths?.([], false);
    }
    event.preventDefault();
    const box = {
      x1: Math.min(drag.x, event.clientX),
      y1: Math.min(drag.y, event.clientY),
      x2: Math.max(drag.x, event.clientX),
      y2: Math.max(drag.y, event.clientY),
    };
    marquee = box;
    if (scroller) {
      const paths: string[] = [];
      for (const node of scroller.querySelectorAll<HTMLElement>('.tile[data-path]')) {
        const r = node.getBoundingClientRect();
        if (r.left < box.x2 && r.right > box.x1 && r.top < box.y2 && r.bottom > box.y1) {
          const path = node.dataset.path;
          if (path) paths.push(path);
        }
      }
      onselectpaths?.(paths, drag.additive);
    }
  }

  function endDrag(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.pointerId) return;
    if (!drag.active && !drag.onTile) onselectpaths?.([], false);
    const wasActive = drag.active;
    drag = null;
    marquee = null;
    if (wasActive) skipNextScroll = true;
    document.body.classList.remove('selecting');
    if (wasActive) setTimeout(() => (suppressClick = false), 0);
  }

  function onScroll(): void {
    if (!scroller || ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      if (scroller) {
        scrollTop = scroller.scrollTop;
        // Fetch the next page while there is still a screen and a half of
        // runway, so the append happens off the critical scroll path.
        if (
          !loadingMore &&
          scroller.scrollTop + scroller.clientHeight >
            scroller.scrollHeight - scroller.clientHeight * 1.5
        ) {
          loadingMore = true;
          onloadmore();
          setTimeout(() => (loadingMore = false), 300);
        }
      }
      ticking = false;
    });
  }
</script>

<div
  class="grid-scroll"
  bind:this={scroller}
  onscroll={onScroll}
  onpointerdown={onPointerDown}
  role="grid"
  tabindex="-1"
  aria-label="Files"
>
  <div class="canvas" style:height="{totalH}px">
    {#each visibleRows as row (row.kind === 'group' ? `g:${row.y}` : `r:${row.y}`)}
      {#if row.kind === 'group'}
        <div class="date-group" style:top="{row.y}px">{row.label}</div>
      {:else}
        {#each row.items as { file, index }, col (file.path)}
      <div
        class="tile"
        class:selected={selectedSet.has(file.path)}
        style:transform="translate3d({PAD + col * (tileW + GAP)}px, {row.y}px, 0)"
        style:width="{tileW}px"
        role="gridcell"
        aria-selected={selectedSet.has(file.path)}
        data-path={file.path}
      >
        <button
          class="hit"
          onclick={(e) => {
            if (suppressClick) {
              e.preventDefault();
              return;
            }
            onselect(file, e);
          }}
          ondblclick={() => onopen(file)}
          oncontextmenu={(e) => {
            e.preventDefault();
            oncontext?.(file, e);
          }}
          aria-label={`${smartName(file).title}, ${fileTypeInfo(file.name, file.kind).description}, ${bytes(file.size)}, modified ${relativeDate(file.mtime)}${file.duration_ms ? `, duration ${formatDuration(file.duration_ms)}` : ''}`}
        >
          <span
            class="frame"
            style:height="{tileW}px"
            draggable="true"
            role="button"
            tabindex="-1"
            aria-label={`Drag ${file.name} to another app`}
            title="Drag to another app"
            onpointerdown={(e) => e.stopPropagation()}
            ondragstart={(e) => {
              e.stopPropagation();
              startFileDrag(e, selectedSet.has(file.path) ? selected : [file.path]);
            }}
          >
            <Thumb {file} size={tileW} eager={index < 24} load={true} />
          </span>
          <span class="name"
            >{middleTruncate(smartName(file).title, tileW >= 220 ? 30 : tileW > 120 ? 24 : 18)}{#if smartName(file).detail}<span class="real-name"> · {smartName(file).detail}</span>{/if}</span
          >
          {#if tileW >= 220}
            <span class="meta">{bytes(file.size)} · {relativeDate(file.mtime)}</span>
          {/if}
        </button>
      </div>
        {/each}
      {/if}
    {/each}
  </div>
</div>

{#if marquee}
  <div
    class="marquee"
    style:left="{marquee.x1}px"
    style:top="{marquee.y1}px"
    style:width="{marquee.x2 - marquee.x1}px"
    style:height="{marquee.y2 - marquee.y1}px"
  ></div>
{/if}

<svelte:window onpointermove={onPointerMove} onpointerup={endDrag} onpointercancel={endDrag} />

<style>
  .grid-scroll {
    position: relative;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    contain: strict;
    user-select: none;
  }

  .canvas {
    position: relative;
    width: 100%;
  }

  .marquee {
    position: fixed;
    z-index: 30;
    border: 1px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    border-radius: 2px;
    pointer-events: none;
  }

  .date-group {
    position: absolute;
    left: 24px;
    right: 24px;
    display: flex;
    align-items: center;
    height: 30px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    font-weight: 550;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    border-bottom: var(--hairline);
    pointer-events: none;
  }

  .tile {
    position: absolute;
    top: 0;
    left: 0;
    contain: layout style paint;
  }

  .hit {
    display: block;
    width: 100%;
    text-align: left;
    border-radius: var(--radius-thumb);
  }

  .frame {
    position: relative;
    display: block;
    cursor: grab;
    width: 100%;
    border-radius: var(--radius-thumb);
    background: transparent;
    overflow: hidden;
  }

  .frame::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: inset 0 0 0 2px transparent;
    transition: box-shadow var(--duration-fade) linear;
    pointer-events: none;
  }

  .tile.selected .frame::after {
    box-shadow: inset 0 0 0 2px var(--accent);
  }

  .hit:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .name {
    display: block;
    margin-top: 6px;
    padding: 0 2px;
    font-size: 13px;
    line-height: 17px;
    font-weight: 450;
    color: var(--ink-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tile.selected .name {
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
  }

  .real-name {
    color: var(--ink-tertiary);
    font-family: var(--font-mono);
    font-size: 9.5px;
    font-weight: 400;
  }

  .meta {
    display: block;
    padding: 0 2px;
    margin-top: 1px;
    font-size: 11.5px;
    line-height: 14px;
    color: var(--ink-tertiary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hit:hover :global(.thumb) {
    transform: scale(1.02);
  }

  :global(.scrolling) .hit:hover :global(.thumb) {
    transform: none;
  }
</style>
