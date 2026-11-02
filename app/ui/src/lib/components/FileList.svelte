<script lang="ts">
  import type { FileMeta } from '../ipc';
  import { bytes, kindLabel, middleTruncate, relativeDate } from '../format';
  import FileTypeIcon from './FileTypeIcon.svelte';
  import { fileTypeInfo } from '../extensions';
  import { startFileDrag } from '../fileDrag';
  import { dateGroup } from '../dateGroups';
  import { smartName } from '../smart';

  let {
    items,
    selected = [],
    onselect,
    onopen,
    onloadmore,
    oncontext,
    onselectpaths,
    columns = { date: true, size: true, kind: true, from: true },
    grouped = false,
    dateField = 'mtime',
  }: {
    items: FileMeta[];
    selected?: string[];
    onselect: (file: FileMeta, event: MouseEvent) => void;
    onopen: (file: FileMeta) => void;
    onloadmore: () => void;
    oncontext?: (file: FileMeta, event: MouseEvent) => void;
    onselectpaths?: (paths: string[], additive: boolean) => void;
    columns?: { date: boolean; size: boolean; kind: boolean; from: boolean };
    grouped?: boolean;
    dateField?: 'mtime' | 'added_at';
  } = $props();

  let template = $derived(
    `minmax(180px, 2.4fr)${columns.date ? ' 130px' : ''}${columns.size ? ' 90px' : ''}${
      columns.kind ? ' 100px' : ''
    }${columns.from ? ' minmax(90px, 1fr)' : ''}`,
  );

  const ROW = 32;
  const HEAD = 28;

  let scroller: HTMLDivElement | null = $state(null);
  let height = $state(600);
  let scrollTop = $state(0);
  let loadingMore = $state(false);
  let ticking = false;
  let selectedSet = $derived(new Set(selected));

  let overscan = $derived(Math.max(1, Math.ceil(height / ROW)));
  let marquee = $state<{ x1: number; y1: number; x2: number; y2: number } | null>(null);
  type DragState = {
    x: number;
    y: number;
    additive: boolean;
    onRow: boolean;
    active: boolean;
    pointerId: number;
  };
  let drag: DragState | null = null;
  let suppressClick = false;
  let skipNextScroll = false;
  let lastScrolledPath = '';

  const GROUP = 28;

  type ListRow =
    | { kind: 'group'; label: string; y: number }
    | { kind: 'file'; file: FileMeta; index: number; y: number };

  let body = $derived.by(() => {
    const rows: ListRow[] = [];
    let y = 0;
    let label = '';
    items.forEach((file, index) => {
      const next = grouped ? dateGroup(file[dateField]) : '';
      if (grouped && next !== label) {
        rows.push({ kind: 'group', label: next, y });
        y += GROUP;
        label = next;
      }
      rows.push({ kind: 'file', file, index, y });
      y += ROW;
    });
    return { rows, height: y };
  });
  let visible = $derived(
    body.rows.filter(
      (row) =>
        row.y + (row.kind === 'group' ? GROUP : ROW) >= scrollTop - overscan * ROW &&
        row.y <= scrollTop + height + overscan * ROW,
    ),
  );

  $effect(() => {
    if (!scroller) return;
    const el = scroller;
    const ro = new ResizeObserver(() => (height = el.clientHeight));
    ro.observe(el);
    height = el.clientHeight;
    return () => ro.disconnect();
  });

  $effect(() => {
    // A new view can be shorter than the old scroll offset; never leave the
    // viewport past the end of the list.
    void items.length;
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
    const row = body.rows.find((r) => r.kind === 'file' && r.file.path === path);
    if (!row) return;
    const top = row.y;
    if (top < scroller.scrollTop) scroller.scrollTop = Math.max(0, top - 8);
    else if (top + ROW > scroller.scrollTop + scroller.clientHeight) {
      scroller.scrollTop = top + ROW - scroller.clientHeight;
    }
  });

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0 || event.pointerType !== 'mouse' || !scroller) return;
    const rect = scroller.getBoundingClientRect();
    if (event.clientX > rect.right - 14) return;
    const target = event.target as HTMLElement;
    drag = {
      x: event.clientX,
      y: event.clientY,
      additive: event.shiftKey || event.ctrlKey || event.metaKey,
      onRow: Boolean(target.closest('.row')),
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
      for (const node of scroller.querySelectorAll<HTMLElement>('.row[data-path]')) {
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
    if (!drag.active && !drag.onRow) onselectpaths?.([], false);
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

<div class="list" bind:this={scroller} onscroll={onScroll} onpointerdown={onPointerDown} role="grid" tabindex="-1" aria-label="Files">
  <div class="head" role="row" style:grid-template-columns={template}>
    <span class="c-name"><span class="head-gap"></span>Name</span>
    {#if columns.date}<span class="c-date">Date modified</span>{/if}
    {#if columns.size}<span class="c-size">Size</span>{/if}
    {#if columns.kind}<span class="c-kind">Kind</span>{/if}
    {#if columns.from}<span class="c-from">From</span>{/if}
  </div>
  <div class="body" style:height="{body.height}px">
    {#each visible as row (row.kind === 'group' ? `g:${row.y}` : row.file.path)}
      {#if row.kind === 'group'}
        <div class="date-group" style:transform="translateY({row.y}px)">{row.label}</div>
      {:else}
      {@const { file, index } = row}
      <button
        class="row"
        class:selected={selectedSet.has(file.path)}
        class:even={index % 2 === 0}
        style:transform="translateY({row.y}px)"
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
        aria-label={`${smartName(file).title}, ${fileTypeInfo(file.name, file.kind).description}, ${bytes(file.size)}, modified ${relativeDate(file.mtime)}`}
        data-path={file.path}
        role="row"
        style:grid-template-columns={template}
      >
        <span class="c-name" title={file.path}>
          <span
            class="file-icon"
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
          ><FileTypeIcon
            name={file.name}
            kind={file.kind}
            size={20}
            variant="list"
            path={file.path}
            mtime={file.mtime}
          /></span>
          <span class="file-label">
            {middleTruncate(smartName(file).title, 52)}
            {#if smartName(file).detail}<span class="real-name">{smartName(file).detail}</span>{/if}
          </span>
        </span>
        {#if columns.date}<span class="c-date">{relativeDate(file.mtime)}</span>{/if}
        {#if columns.size}<span class="c-size">{bytes(file.size)}</span>{/if}
        {#if columns.kind}<span class="c-kind">{kindLabel(file.kind)}</span>{/if}
        {#if columns.from}
          <span class="c-from">{file.origin_host ?? file.origin_app ?? ''}</span>
        {/if}
      </button>
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
  .list {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    contain: strict;
    background: var(--surface-content);
    user-select: none;
  }

  .marquee {
    position: fixed;
    z-index: 30;
    border: 1px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    border-radius: 2px;
    pointer-events: none;
  }

  .row {
    position: relative;
  }

  .date-group {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    height: 28px;
    padding: 0 24px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    font-weight: 550;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    background: var(--surface-content);
    border-bottom: var(--hairline);
  }

  .head,
  .row {
    display: grid;
    align-items: center;
    gap: 12px;
    padding: 0 24px;
  }

  .head {
    position: sticky;
    top: 0;
    z-index: 2;
    height: 28px;
    background: var(--surface-content);
    border-bottom: var(--hairline);
    color: var(--ink-tertiary);
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.005em;
  }

  .body {
    position: relative;
    width: 100%;
  }

  .row {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 32px;
    contain: layout style paint;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-primary);
    font-size: 12.5px;
    font-weight: 450;
    text-align: left;
    white-space: nowrap;
    transition: background var(--duration-hover) var(--ease-hover);
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .row.selected {
    background: var(--surface-select);
  }

  .c-name {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .head-gap,
  .file-icon {
    flex: 0 0 20px;
    width: 20px;
    height: 20px;
    cursor: grab;
  }

  .real-name {
    margin-left: 6px;
    color: var(--ink-tertiary);
    font-family: var(--font-mono);
    font-size: 9.5px;
    font-weight: 400;
  }

  .file-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .c-date,
  .c-size,
  .c-kind {
    color: var(--ink-secondary);
    font-variant-numeric: tabular-nums;
  }

  .c-size {
    text-align: right;
  }

  .c-from {
    color: var(--ink-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
