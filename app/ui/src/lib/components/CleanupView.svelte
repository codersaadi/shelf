<script lang="ts">
  import { shelf } from '../stores.svelte';
  import { bytes, middleTruncate, parentPath, plural, riskLabel } from '../format';
  import type { Finding, Risk } from '../ipc';
  import Icon from './Icon.svelte';
  import Thumb from './Thumb.svelte';

  let review = $state<Finding | null>(null);
  let checked = $state<Record<string, boolean>>({});
  let dryRun = $state(false);
  let busy = $state(false);
  let copiedId = $state<string | null>(null);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  let riskFilter = $state<'all' | Risk>('all');

  let total = $derived(
    shelf.findings.filter((f) => !f.command).reduce((n, f) => n + f.bytes, 0),
  );
  let totalFiles = $derived(shelf.findings.reduce((n, f) => n + f.count, 0));
  let reportCount = $derived(shelf.findings.filter((f) => f.command).length);
  let riskSplit = $derived({
    low: shelf.findings.filter((f) => !f.command && f.risk === 'low').reduce((n, f) => n + f.bytes, 0),
    medium: shelf.findings.filter((f) => !f.command && f.risk === 'medium').reduce((n, f) => n + f.bytes, 0),
    high: shelf.findings.filter((f) => !f.command && f.risk === 'high').reduce((n, f) => n + f.bytes, 0),
  });
  let visible = $derived(
    riskFilter === 'all' ? shelf.findings : shelf.findings.filter((f) => f.risk === riskFilter),
  );
  let actionable = $derived(visible.filter((f) => !f.command));
  let system = $derived(visible.filter((f) => f.command));

  let chosen = $derived(review ? review.files.filter((f) => checked[f.path]) : []);
  let chosenBytes = $derived(chosen.reduce((n, f) => n + f.size, 0));

  const riskOptions: { value: 'all' | Risk; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'low', label: 'Low' },
    { value: 'medium', label: 'Medium' },
    { value: 'high', label: 'High' },
  ];

  function riskWidth(value: number): string {
    return total > 0 ? `${Math.round((value / total) * 100)}%` : '0%';
  }

  function findingIcon(finding: Finding): 'app' | 'image' | 'archive' | 'info' | 'clean' | 'document' {
    if (finding.id === 'installers') return 'app';
    if (finding.id === 'old-screenshots') return 'image';
    if (finding.id === 'download-archives') return 'archive';
    if (finding.id === 'large-files') return 'info';
    if (finding.id === 'journal') return 'document';
    return 'clean';
  }

  function openReview(finding: Finding): void {
    if (finding.command) return;
    review = finding;
    const defaults: Record<string, boolean> = {};
    for (const f of finding.files) defaults[f.path] = finding.risk === 'low';
    checked = defaults;
    dryRun = false;
  }

  function toggleAll(on: boolean): void {
    if (!review) return;
    const next: Record<string, boolean> = {};
    for (const f of review.files) next[f.path] = on;
    checked = next;
  }

  async function copyCommand(finding: Finding): Promise<void> {
    if (!finding.command) return;
    if (await shelf.copyCommand(finding.command)) {
      copiedId = finding.id;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copiedId = null), 1500);
    }
  }

  async function move(): Promise<void> {
    if (!chosen.length || busy) return;
    busy = true;
    const result = await shelf.trashPaths(chosen.map((f) => f.path));
    busy = false;
    if (result) review = null;
  }
</script>

{#snippet findingCard(finding: Finding)}
  <article class="card" class:report={finding.command}>
    <div class="card-top">
      <span class="card-icon"><Icon name={findingIcon(finding)} size={16} /></span>
      <div class="card-copy">
        <div class="title-row">
          <h2>{finding.title}</h2>
          {#if finding.command}<span class="badge">Report only</span>{/if}
        </div>
        <p class="reason">
          {finding.reason}{#if finding.truncated} Showing the first {finding.count} matches.{/if}
        </p>
      </div>
      <div class="metric">
        {#if finding.command}
          <strong>{bytes(finding.bytes)}</strong>
          <span>Report only</span>
        {:else}
          <strong>{bytes(finding.bytes)}</strong>
          <span>{finding.count} {plural(finding.count, 'file')}</span>
        {/if}
      </div>
    </div>

    {#if finding.command}
      <pre class="command">{finding.command}</pre>
    {:else}
      <div class="evidence">
        {#each finding.files.slice(0, 5) as file (file.path)}
          <span class="ev"><Thumb {file} size={48} /></span>
        {/each}
        {#if finding.files.length > 5}
          <span class="more">+{finding.files.length - 5}</span>
        {/if}
      </div>
    {/if}

    <footer>
      <span
        class="risk"
        class:medium={finding.risk === 'medium'}
        class:high={finding.risk === 'high'}
      >
        <span class="dot" aria-hidden="true"></span>{riskLabel(finding.risk)}
      </span>
      {#if finding.command}
        <span class="action-note">Report only</span>
        <button class="button" onclick={() => copyCommand(finding)}>
          {copiedId === finding.id ? 'Copied' : 'Copy command'}
        </button>
      {:else}
        <button class="button review-button" onclick={() => openReview(finding)}>
          Review <Icon name="chevron-right" size={12} />
        </button>
      {/if}
    </footer>
  </article>
{/snippet}

<div class="cleanup">
  {#if review}
    <div class="review">
      <div class="review-scroll">
        <button class="back" onclick={() => (review = null)}>
          <Icon name="chevron-left" size={14} />All findings
        </button>

        <div class="review-head">
          <div class="title-row">
            <h2>{review.title}</h2>
            <span class="risk" class:medium={review.risk === 'medium'} class:high={review.risk === 'high'}>
              <span class="dot" aria-hidden="true"></span>{riskLabel(review.risk)}
            </span>
          </div>
          <p class="reason">{review.reason}</p>
          <p class="review-meta">
            {review.files.length} {plural(review.files.length, 'file')} · {bytes(review.bytes)}
          </p>
        </div>

        <div class="review-bar">
          <button class="link" onclick={() => toggleAll(true)}>Select all</button>
          <button class="link" onclick={() => toggleAll(false)}>Select none</button>
          <span class="bar-meta">{chosen.length} of {review.files.length} selected</span>
        </div>

        <ul class="file-list">
          {#each review.files as file (file.path)}
            <li>
              <label class="file-row">
                <input
                  type="checkbox"
                  checked={checked[file.path] ?? false}
                  onchange={(e) => (checked[file.path] = (e.currentTarget as HTMLInputElement).checked)}
                />
                <span class="mini"><Thumb {file} size={32} load={review.files.length <= 80} /></span>
                <span class="file-copy">
                  <span class="file-name" title={file.path}>{middleTruncate(file.name, 56)}</span>
                  <span class="file-parent">{middleTruncate(parentPath(file.path), 64)}</span>
                </span>
                <span class="file-size">{bytes(file.size)}</span>
              </label>
            </li>
          {/each}
        </ul>

        <label class="dry">
          <input type="checkbox" bind:checked={dryRun} />
          Show what will happen
        </label>
        {#if dryRun}
          <div class="dry-list">
            {#each chosen as f (f.path)}
              <p>{f.path}</p>
            {:else}
              <p class="quiet">Nothing selected.</p>
            {/each}
            <p class="quiet">Freeing {bytes(chosenBytes)}. Files go to the Trash, not deleted.</p>
          </div>
        {/if}
      </div>

      <footer class="review-foot">
        <div class="foot-copy">
          <strong>Move {chosen.length} {plural(chosen.length, 'file')} to Trash</strong>
          <span>{bytes(chosenBytes)} · Undo stays available for 30 seconds</span>
        </div>
        <button class="button primary" disabled={!chosen.length || busy} onclick={move}>
          {busy ? 'Moving…' : 'Move to Trash'}
        </button>
      </footer>
    </div>
  {:else}
    <div class="scroll">
      <div class="page-head">
        <div>
          <h2>Review before you remove</h2>
          <p>Suggestions from the last scan. Nothing moves until you review it.</p>
        </div>
        {#if !shelf.findingsLoading}
          <span class="finding-count">{shelf.findings.length} {plural(shelf.findings.length, 'finding')}</span>
        {/if}
      </div>

      {#if shelf.findingsLoading}
        <div class="loading" aria-busy="true">
          <span class="loading-dot"></span>
          Reading the index…
        </div>
      {:else if shelf.findings.length === 0}
        <div class="empty">
          <span class="empty-icon"><Icon name="check" size={20} /></span>
          <h2>Nothing to clean up</h2>
          <p>Shelf checks again after each scan.</p>
          <button class="button" onclick={() => shelf.loadFindings()}>Check again</button>
        </div>
      {:else}
        <div class="risk-tabs" role="group" aria-label="Filter findings by risk">
          {#each riskOptions as option (option.value)}
            <button
              class="risk-tab"
              class:active={riskFilter === option.value}
              aria-pressed={riskFilter === option.value}
              onclick={() => (riskFilter = option.value)}
            >
              {option.label}
            </button>
          {/each}
        </div>

        {#if actionable.length > 0}
          <div class="cards">
            {#each actionable as finding (finding.id)}
              {@render findingCard(finding)}
            {/each}
          </div>
        {/if}

        {#if system.length > 0}
          <div class="section-head">
            <h3>System caches</h3>
            <span>Report only · commands are copied, never run</span>
          </div>
          <div class="cards">
            {#each system as finding (finding.id)}
              {@render findingCard(finding)}
            {/each}
          </div>
        {/if}

        {#if visible.length === 0}
          <div class="filtered-empty">
            <p>No {riskFilter} risk findings in this scan.</p>
            <button class="link" onclick={() => (riskFilter = 'all')}>Show all findings</button>
          </div>
        {/if}
      {/if}
    </div>

    <aside class="summary">
      <div class="summary-head">
        <h3>Reclaimable</h3>
        {#if !shelf.findingsLoading}
          <span>{shelf.findings.length} {plural(shelf.findings.length, 'finding')}</span>
        {/if}
      </div>
      <p class="big">{bytes(total)}</p>
      <p class="summary-sub">
        {totalFiles} {plural(totalFiles, 'file')}{#if reportCount} · {reportCount} report-only{/if}
      </p>

      {#if total > 0}
        <div class="risk-bar" aria-hidden="true">
          <span class="low" style:width={riskWidth(riskSplit.low)}></span>
          <span class="medium" style:width={riskWidth(riskSplit.medium)}></span>
          <span class="high" style:width={riskWidth(riskSplit.high)}></span>
        </div>
      {/if}

      <div class="split">
        <p><span class="dot low"></span>Low risk <span>{bytes(riskSplit.low)}</span></p>
        <p><span class="dot medium"></span>Medium risk <span>{bytes(riskSplit.medium)}</span></p>
        <p><span class="dot high"></span>High risk <span>{bytes(riskSplit.high)}</span></p>
      </div>

      <p class="note">
        Sizes cover the files shown here; hardlinks are counted once. Report-only system
        estimates are not included in the reclaimable total.
      </p>
      <button class="button wide" onclick={() => shelf.loadFindings()}>Scan again</button>
    </aside>
  {/if}
</div>

<style>
  .cleanup {
    height: 100%;
    display: flex;
    overflow: hidden;
    background: var(--surface-content);
  }

  .scroll {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .page-head {
    display: flex;
    align-items: flex-start;
    gap: 16px;
  }

  .page-head h2 {
    margin: 0;
    font-size: 18px;
    line-height: 24px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }

  .page-head p {
    margin: 4px 0 0;
    max-width: 54ch;
    font-size: 12px;
    line-height: 17px;
    color: var(--ink-secondary);
  }

  .finding-count {
    margin-left: auto;
    padding-top: 2px;
    color: var(--ink-tertiary);
    font-size: 11.5px;
    white-space: nowrap;
  }

  .risk-tabs {
    display: inline-flex;
    align-self: flex-start;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-window);
  }

  .risk-tab {
    height: 24px;
    padding: 0 10px;
    border-radius: 5px;
    color: var(--ink-tertiary);
    font-size: 11.5px;
  }

  .risk-tab:hover {
    color: var(--ink-primary);
  }

  .risk-tab.active {
    background: var(--surface-raised);
    color: var(--ink-primary);
    box-shadow: 0 0 0 1px var(--line-hairline);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    align-items: start;
    gap: 12px;
    width: 100%;
  }

  .card {
    position: relative;
    overflow: hidden;
    padding: 16px 16px 12px;
    border: var(--hairline);
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
  }

  .card-top {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .card-icon {
    display: grid;
    place-items: center;
    flex: 0 0 28px;
    width: 28px;
    height: 28px;
    margin-top: 1px;
    border-radius: 7px;
    background: var(--surface-hover);
    color: var(--ink-secondary);
  }

  .card-copy {
    min-width: 0;
    flex: 1;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .title-row h2 {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    line-height: 18px;
    font-weight: 600;
  }

  .badge {
    flex: 0 0 auto;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface-hover);
    color: var(--ink-tertiary);
    font-size: 10px;
    line-height: 14px;
  }

  .reason {
    margin: 2px 0 0;
    font-size: 11.5px;
    line-height: 16px;
    color: var(--ink-secondary);
  }

  .metric {
    flex: 0 0 auto;
    margin-left: 8px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .metric strong {
    display: block;
    color: var(--ink-primary);
    font-size: 13px;
    line-height: 18px;
    font-weight: 600;
  }

  .metric span {
    display: block;
    margin-top: 1px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    line-height: 14px;
  }

  .evidence {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 12px;
  }

  .ev {
    display: block;
    width: 48px;
    height: 48px;
    overflow: hidden;
    border-radius: 5px;
    background: var(--surface-hover);
  }

  .more {
    display: grid;
    place-items: center;
    height: 48px;
    padding: 0 8px;
    border-radius: 5px;
    background: var(--surface-hover);
    color: var(--ink-tertiary);
    font-size: 11px;
  }

  .command {
    margin: 12px 0 0;
    padding: 9px 10px;
    overflow-wrap: anywhere;
    border-radius: 6px;
    background: var(--surface-hover);
    color: var(--ink-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 16px;
    white-space: pre-wrap;
  }

  .card footer {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: var(--hairline);
  }

  .risk {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--ink-tertiary);
    font-size: 11px;
    white-space: nowrap;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ink-tertiary);
  }

  .risk.medium .dot,
  .dot.medium {
    background: var(--caution);
  }

  .risk.high .dot,
  .dot.high {
    background: var(--danger);
  }

  .dot.low {
    background: var(--ink-tertiary);
  }

  .action-note {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .review-button {
    gap: 2px;
    margin-left: auto;
    padding-right: 7px;
  }

  .card footer .button:not(.review-button) {
    margin-left: auto;
  }

  .section-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-top: 6px;
    padding-top: 14px;
    border-top: var(--hairline);
  }

  .section-head h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
  }

  .section-head span {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .summary {
    display: flex;
    flex: 0 0 268px;
    flex-direction: column;
    gap: 10px;
    width: 268px;
    min-height: 0;
    overflow-y: auto;
    padding: 22px 22px 20px;
    border-left: var(--hairline);
    background: var(--surface-window);
  }

  .summary::-webkit-scrollbar {
    width: 8px;
  }

  .summary-head {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .summary-head h3 {
    margin: 0;
    color: var(--ink-tertiary);
    font-size: 11.5px;
    font-weight: 500;
  }

  .summary-head span {
    margin-left: auto;
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .big {
    margin: 0;
    font-size: 26px;
    line-height: 32px;
    font-weight: 650;
    letter-spacing: -0.025em;
    font-variant-numeric: tabular-nums;
  }

  .summary-sub {
    margin: -4px 0 2px;
    color: var(--ink-tertiary);
    font-size: 11px;
    line-height: 15px;
  }

  .risk-bar {
    display: flex;
    width: 100%;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--surface-hover);
  }

  .risk-bar .low {
    background: var(--ink-tertiary);
  }

  .risk-bar .medium {
    background: var(--caution);
  }

  .risk-bar .high {
    background: var(--danger);
  }

  .split {
    display: grid;
    gap: 4px;
    padding: 2px 0;
  }

  .split p {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--ink-secondary);
    font-size: 11.5px;
  }

  .split span:last-child {
    margin-left: auto;
    color: var(--ink-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .note {
    margin: 2px 0 6px;
    color: var(--ink-tertiary);
    font-size: 10.5px;
    line-height: 15px;
  }

  .summary .wide {
    width: 100%;
    margin-top: auto;
    justify-content: center;
  }

  .review {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    max-width: 980px;
  }

  .review-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .back {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    gap: 2px;
    margin-bottom: 6px;
    color: var(--accent);
    font-size: 12px;
  }

  .review-head .title-row {
    justify-content: flex-start;
  }

  .review-head h2 {
    overflow: visible;
    margin: 0;
    font-size: 18px;
    line-height: 24px;
    font-weight: 620;
    letter-spacing: -0.015em;
    white-space: normal;
  }

  .review-head .reason {
    margin-top: 4px;
  }

  .review-meta {
    margin: 6px 0 0;
    color: var(--ink-tertiary);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .review-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-bottom: var(--hairline);
  }

  .link {
    color: var(--accent);
    font-size: 12px;
  }

  .bar-meta {
    margin-left: auto;
    color: var(--ink-tertiary);
    font-size: 11.5px;
  }

  .file-list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .file-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 46px;
    padding: 3px 6px;
    border-radius: 6px;
    cursor: pointer;
  }

  .file-row:hover {
    background: var(--surface-hover);
  }

  .mini {
    width: 32px;
    height: 32px;
    flex: 0 0 32px;
    overflow: hidden;
    border-radius: 4px;
  }

  .file-copy {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 1px;
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .file-parent {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .file-size {
    color: var(--ink-tertiary);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  input[type='checkbox'] {
    width: 14px;
    height: 14px;
    accent-color: var(--accent);
  }

  .dry {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink-secondary);
    font-size: 12px;
  }

  .dry-list {
    max-height: 220px;
    overflow-y: auto;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--surface-hover);
    font-family: var(--font-mono);
    font-size: 10.5px;
    line-height: 16px;
  }

  .dry-list p {
    margin: 0 0 4px;
    overflow-wrap: anywhere;
  }

  .dry-list .quiet {
    margin-top: 8px;
    font-family: var(--font-ui);
    font-size: 10.5px;
  }

  .review-foot {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 24px;
    border-top: var(--hairline);
    background: var(--surface-content);
  }

  .foot-copy {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 0;
  }

  .foot-copy strong {
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .foot-copy span {
    color: var(--ink-tertiary);
    font-size: 10.5px;
  }

  .empty {
    display: grid;
    justify-items: center;
    gap: 8px;
    padding: 56px 0;
    text-align: center;
    color: var(--ink-secondary);
  }

  .empty-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
  }

  .empty h2 {
    margin: 4px 0 0;
    color: var(--ink-primary);
    font-size: 15px;
    font-weight: 600;
  }

  .empty p {
    margin: 0 0 6px;
    font-size: 12px;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink-tertiary);
    font-size: 12px;
  }

  .loading-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .filtered-empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px;
    border: var(--hairline);
    border-radius: var(--radius-panel);
    color: var(--ink-secondary);
    font-size: 12px;
  }

  .filtered-empty p {
    margin: 0;
  }

  .quiet {
    color: var(--ink-tertiary);
    font-size: 11.5px;
    line-height: 16px;
  }

  @media (max-width: 1100px) {
    .cleanup {
      flex-direction: column;
    }

    .summary {
      order: -1;
      flex: 0 0 auto;
      width: auto;
      padding: 14px 20px;
      border-bottom: var(--hairline);
      border-left: 0;
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
      gap: 8px 18px;
    }

    .summary-head {
      flex: 0 0 100%;
    }

    .big {
      font-size: 20px;
      line-height: 24px;
    }

    .summary-sub {
      margin: 0;
    }

    .risk-bar {
      flex: 1 0 100px;
      max-width: 240px;
    }

    .split {
      display: flex;
      gap: 12px;
    }

    .note {
      flex: 1 0 100%;
      margin: 0;
    }

    .summary .wide {
      width: auto;
      margin-top: 0;
    }
  }

  @media (max-width: 839px) {
    .scroll,
    .review-scroll {
      padding: 16px;
    }

    .review-foot {
      padding: 12px 16px;
    }

    .page-head {
      display: block;
    }

    .finding-count {
      display: block;
      margin-top: 6px;
    }

    .card-top {
      flex-wrap: wrap;
    }

    .metric {
      margin-left: 38px;
      text-align: left;
    }

    .review-foot {
      flex-wrap: wrap;
    }

    .foot-copy {
      flex: 1 1 100%;
    }

    .review-foot .button {
      width: 100%;
      justify-content: center;
    }
  }
</style>
