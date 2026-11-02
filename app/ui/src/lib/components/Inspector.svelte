<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { bytes, dateTime, formatDuration, kindLabel, middleTruncate, parentPath, plural } from '../format';
  import { thumbUrl } from '../ipc';
  import { extensionOf } from '../extensions';
  import FileTypeIcon from './FileTypeIcon.svelte';
  import Icon from './Icon.svelte';
  import Thumb from './Thumb.svelte';
  import { imageHasTransparency } from '../imageTransparency';
  import { smartName } from '../smart';

  let selected = $derived(shelf.selectedFiles);
  let file = $derived(shelf.inspector?.file ?? selected[0] ?? null);
  let multi = $derived(selected.length > 1);
  let total = $derived(selected.reduce((n, f) => n + f.size, 0));
  let tags = $derived(shelf.inspector?.tags ?? []);
  let tagDraft = $state('');

  async function addTag(): Promise<void> {
    const tag = tagDraft.trim().replace(/^#/, '');
    if (!tag || tags.includes(tag)) return;
    tagDraft = '';
    await shelf.saveTags(file.path, [...tags, tag]);
  }

  async function removeTag(tag: string): Promise<void> {
    await shelf.saveTags(file.path, tags.filter((t) => t !== tag));
  }
  let previewFailed = $state(false);
  let previewLoaded = $state(false);
  let previewTransparent = $state(false);
  let previewRetry = $state(0);
  let previewRetryTimer: ReturnType<typeof setTimeout> | undefined;
  let previewable = $derived(
    file
      ? file.kind === 'image' ||
          file.kind === 'video' ||
          file.kind === 'music' ||
          extensionOf(file.name) === 'pdf'
      : false,
  );
  const previewSrc = $derived(file ? thumbUrl(file, 640, previewRetry) : '');

  $effect(() => {
    void file?.path;
    if (previewRetryTimer) clearTimeout(previewRetryTimer);
    previewRetryTimer = undefined;
    previewFailed = false;
    previewLoaded = false;
    previewTransparent = false;
    previewRetry = 0;
    return () => {
      if (previewRetryTimer) clearTimeout(previewRetryTimer);
    };
  });

  function onPreviewLoad(event: Event): void {
    if (previewRetryTimer) clearTimeout(previewRetryTimer);
    previewRetryTimer = undefined;
    previewLoaded = true;
    previewTransparent = imageHasTransparency(event.currentTarget as HTMLImageElement);
  }

  function onPreviewError(): void {
    if (previewRetry >= 2) {
      previewFailed = true;
      return;
    }
    const next = previewRetry + 1;
    if (previewRetryTimer) clearTimeout(previewRetryTimer);
    previewRetryTimer = setTimeout(() => {
      previewRetry = next;
      previewRetryTimer = undefined;
    }, next * 150);
  }

  async function showInFiles(): Promise<void> {
    await shelf.showInFiles(selected.length ? selected.map((f) => f.path) : file ? [file.path] : []);
  }

  async function copyPath(): Promise<void> {
    if (file) await shelf.copyPath(file.path);
  }

  async function trash(): Promise<void> {
    await shelf.trashSelection();
  }
</script>

<aside class="inspector" aria-label="Inspector">
  {#if !file}
    <div class="empty">
      <Icon name="info" size={22} />
      <p>Select a file to see its details.</p>
    </div>
  {:else if multi}
    <div class="stack" aria-hidden="true">
      {#each selected.slice(0, 3) as f, i (f.path)}
        <div class="stack-card" style:transform="translate({i * 6}px, {i * -6}px)">
          <Thumb file={f} size={260} eager={i === 0} />
        </div>
      {/each}
    </div>
    <div class="heading">
      <h2>{selected.length} {plural(selected.length, 'file')} selected</h2>
      <p class="sub">{bytes(total)} combined</p>
    </div>
    <dl class="facts">
      {#each selected.slice(0, 6) as f (f.path)}
        <div class="fact wide">
          <dt class="visually-hidden">Selected file</dt>
          <dd class="path" title={f.path}>{middleTruncate(f.name, 40)}</dd>
        </div>
      {/each}
    </dl>
  {:else}
    <div class="preview" class:media={previewable && !previewFailed && previewTransparent}>
      {#if previewable && !previewFailed}
        <img
          class="preview-img"
          class:preview-loaded={previewLoaded}
          src={previewSrc}
          alt=""
          crossorigin="anonymous"
          onload={onPreviewLoad}
          onerror={onPreviewError}
        />
      {/if}
      {#if !previewable || previewFailed || !previewLoaded}
        <div class="preview-glyph" class:overlay={previewable && !previewFailed}>
          <div class="big-ext">
            <FileTypeIcon
              name={file.name}
              kind={file.kind}
              size={150}
              variant="inspector"
              status={previewFailed ? 'unavailable' : previewable ? 'loading' : 'idle'}
              path={file.path}
              mtime={file.mtime}
            />
          </div>
          <p class="preview-reason">
            {#if file.size === 0}
              Empty file.
            {:else if previewFailed}
              Preview unavailable. Open the file to view it.
            {:else if previewable}
              Preparing preview…
            {:else}
              No preview for this file type. Open it in its app.
            {/if}
          </p>
          <button class="button" onclick={() => shelf.openFile(file)}>Open</button>
        </div>
      {/if}
    </div>

    <div class="heading">
      <div class="heading-text">
        <h2 title={file.path}>{smartName(file).title}</h2>
        {#if smartName(file).detail}<p class="smart-detail">{smartName(file).detail}</p>{/if}
        <p class="sub">{kindLabel(file.kind)}{file.is_screenshot ? ' · Screenshot' : ''}</p>
      </div>
      <button
        class="icon-button open"
        aria-label="Open file"
        title="Open with the default application"
        onclick={() => shelf.openFile(file)}
      >
        <Icon name="external" size={15} />
      </button>
    </div>

    <dl class="facts">
      <div class="fact"><dt>Size</dt><dd>{bytes(file.size)}</dd></div>
      {#if shelf.inspector?.width && shelf.inspector?.height}
        <div class="fact">
          <dt>Dimensions</dt>
          <dd>{shelf.inspector.width} × {shelf.inspector.height}</dd>
        </div>
      {/if}
      <div class="fact"><dt>Modified</dt><dd>{dateTime(file.mtime)}</dd></div>
      {#if file.duration_ms}
        <div class="fact"><dt>Duration</dt><dd>{formatDuration(file.duration_ms)}</dd></div>
      {/if}
      {#if shelf.inspector?.created}
        <div class="fact"><dt>Created</dt><dd>{dateTime(shelf.inspector.created)}</dd></div>
      {/if}
      <div class="fact"><dt>Added</dt><dd>{dateTime(file.added_at)}</dd></div>
      <div class="fact"><dt>Location</dt><dd class="path" title={file.path}>{middleTruncate(parentPath(file.path), 30)}</dd></div>
      {#if file.origin_host || file.origin_app || file.origin_url}
        <div class="fact">
          <dt>From</dt>
          <dd>
            <span title={file.origin_url ?? ''}>
              {file.origin_host ?? file.origin_app ?? 'Downloaded'}
            </span>
          </dd>
        </div>
      {/if}
      {#if file.opens}
        <div class="fact"><dt>Opened</dt><dd>{file.opens} {plural(file.opens, 'time')}</dd></div>
      {/if}
    </dl>

    {#if shelf.inspector?.text_preview}
      <pre class="text-preview">{shelf.inspector.text_preview}</pre>
    {/if}

    <div class="tags">
      {#each tags as tag (tag)}
        <button class="tag" title="Remove tag" onclick={() => removeTag(tag)}>
          <Icon name="tag" size={11} />{tag}<Icon name="close" size={10} />
        </button>
      {/each}
    </div>
    <form
      class="tag-form"
      onsubmit={(e) => {
        e.preventDefault();
        void addTag();
      }}
    >
      <input
        bind:value={tagDraft}
        placeholder="Add tag"
        aria-label="Add tag"
        spellcheck="false"
        autocomplete="off"
      />
      <button class="button" type="submit" disabled={!tagDraft.trim()}>Add</button>
    </form>

    {#if !shelf.inspector?.on_disk}
      <p class="missing">This file is no longer on disk. Reindex to update the shelf.</p>
    {/if}
  {/if}

  {#if file}
    <div class="actions">
      <button class="button primary" onclick={showInFiles}>
        <Icon name="external" size={13} />Show in Files
      </button>
      <button class="button" onclick={copyPath}><Icon name="copy" size={13} />Copy path</button>
      {#if shelf.view !== 'trash'}
        <button class="button danger" onclick={trash}><Icon name="trash" size={13} />Move to Trash</button>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .inspector {
    width: 300px;
    min-width: 300px;
    height: 100%;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: var(--surface-raised);
    border-left: var(--hairline);
  }

  .preview {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    min-height: 180px;
    border-radius: var(--radius-panel);
    background: transparent;
    overflow: visible;
  }

  .preview.media {
    background-color: var(--surface-content);
    background-image: conic-gradient(
      var(--checker-a) 25%,
      var(--checker-b) 0 50%,
      var(--checker-a) 0 75%,
      var(--checker-b) 0
    );
    background-size: 20px 20px;
  }

  .preview-img {
    width: 100%;
    max-height: 320px;
    object-fit: contain;
    display: block;
    opacity: 0;
    transition: opacity 90ms linear;
  }

  .preview-img.preview-loaded {
    opacity: 1;
  }

  .preview-glyph.overlay {
    position: absolute;
    inset: 0;
    background: var(--surface-content);
  }

  .preview-reason {
    margin: 0;
    max-width: 30ch;
    font-size: 11px;
    line-height: 15px;
    color: var(--ink-tertiary);
    text-align: center;
  }

  .big-ext {
    width: 150px;
    height: 195px;
  }

  .preview :global(.thumb) {
    border-radius: var(--radius-panel);
    background: transparent;
  }

  .preview :global(img) {
    max-height: 320px;
    object-fit: contain;
  }

  .preview-glyph {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 16px 0;
    color: var(--ink-tertiary);
  }

  .heading {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .heading-text {
    flex: 1;
    min-width: 0;
  }

  .open {
    margin-top: -4px;
  }

  .heading h2 {
    margin: 0;
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
    max-width: 44ch;
  }

  .heading .sub {
    margin: 2px 0 0;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-secondary);
  }

  .facts {
    margin: 0;
    display: grid;
    gap: 6px;
  }

  .fact {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    line-height: 16px;
    min-width: 0;
  }

  .fact dt {
    color: var(--ink-tertiary);
    flex: 0 0 auto;
  }

  .fact dd {
    margin: 0;
    color: var(--ink-secondary);
    text-align: right;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .fact.wide dd {
    text-align: left;
    white-space: normal;
  }

  .path {
    font-variant-numeric: normal;
  }

  .text-preview {
    margin: 0;
    padding: 10px 12px;
    max-height: 190px;
    overflow: auto;
    border-radius: 8px;
    background: var(--surface-hover);
    color: var(--ink-secondary);
    font-family: var(--font-mono);
    font-size: 10.5px;
    line-height: 15px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .smart-detail {
    margin: -2px 0 8px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    letter-spacing: 0.01em;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tag-form {
    display: flex;
    gap: 5px;
  }

  .tag-form input {
    flex: 1;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--line-hairline);
    border-radius: 6px;
    background: var(--surface-content);
    color: var(--ink-primary);
    font-size: 11px;
  }

  .tag-form input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    color: var(--ink-secondary);
    font-size: 11px;
  }

  .missing {
    margin: 0;
    padding: 8px 10px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--caution) 12%, transparent);
    color: var(--ink-secondary);
    font-size: 11.5px;
    line-height: 15px;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: auto;
    padding-top: 8px;
  }

  .actions .button {
    justify-content: center;
    height: 30px;
  }

  .stack {
    position: relative;
    height: 200px;
    border-radius: var(--radius-panel);
    background: var(--surface-hover);
    overflow: hidden;
  }

  .stack-card {
    position: absolute;
    inset: 12px;
    border-radius: var(--radius-panel);
    overflow: hidden;
    background: var(--surface-raised);
    box-shadow: 0 0 0 1px var(--line-hairline);
  }

  .empty {
    display: grid;
    justify-items: center;
    gap: 8px;
    margin: auto;
    color: var(--ink-tertiary);
    text-align: center;
    font-size: 12px;
  }

  .empty p {
    margin: 0;
    max-width: 22ch;
  }

  @media (max-width: 959px) {
    .inspector {
      position: absolute;
      top: 0;
      right: 0;
      bottom: 0;
      z-index: 20;
      box-shadow: var(--shadow-popover);
    }
  }
</style>
