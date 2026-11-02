<script lang="ts">
  import { trapFocus } from '../focus';
  import { shelf } from '../stores.svelte';
  import { bytes, middleTruncate, plural } from '../format';
  import Icon from './Icon.svelte';

  let prompt = $derived(shelf.trashPrompt);
  let files = $derived(
    prompt ? shelf.items.filter((f) => prompt.paths.includes(f.path)) : [],
  );
  let more = $derived(prompt ? Math.max(0, prompt.paths.length - files.length) : 0);

  function onKey(event: KeyboardEvent): void {
    if (!prompt) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      shelf.cancelTrash();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      void shelf.confirmTrash();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if prompt}
  <div
    class="backdrop"
    role="presentation"
    onclick={() => shelf.cancelTrash()}
    onkeydown={(e) => e.key === 'Escape' && shelf.cancelTrash()}
  >
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="trash-title"
      tabindex="-1"
    use:trapFocus
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="head">
        <span class="glyph"><Icon name="trash" size={18} /></span>
        <div>
          <h2 id="trash-title">
            Move {prompt.paths.length} {plural(prompt.paths.length, 'file')} to Trash?
          </h2>
          <p>
            {#if prompt.bytes > 0}{bytes(prompt.bytes)} · {/if}Files stay in Trash until
            you empty it. You can undo this.
          </p>
        </div>
      </div>
      {#if files.length}
        <ul>
          {#each files.slice(0, 6) as file (file.path)}
            <li>
              <span>{middleTruncate(file.name, 44)}</span>
              <span class="size">{bytes(file.size)}</span>
            </li>
          {/each}
          {#if more > 0 || prompt.paths.length > 6}
            <li class="more">and {Math.max(more, prompt.paths.length - 6)} more</li>
          {/if}
        </ul>
      {/if}
      <div class="actions">
        <button class="button" onclick={() => shelf.cancelTrash()}>Cancel</button>
        <button class="button danger" onclick={() => shelf.confirmTrash()}>
          Move to Trash
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 65;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, #000 30%, transparent);
  }

  .dialog {
    width: min(440px, calc(100vw - 48px));
    padding: 18px;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    border: 1px solid var(--line-hairline);
    box-shadow: var(--shadow-popover);
  }

  .head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: 0 0 32px;
    border-radius: 8px;
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }

  h2 {
    margin: 1px 0 4px;
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
  }

  p {
    margin: 0;
    font-size: 12px;
    line-height: 17px;
    color: var(--ink-secondary);
  }

  ul {
    list-style: none;
    margin: 14px 0;
    padding: 8px 10px;
    max-height: 180px;
    overflow-y: auto;
    border-radius: 8px;
    background: var(--surface-hover);
  }

  li {
    display: flex;
    gap: 12px;
    font-size: 11.5px;
    line-height: 20px;
    color: var(--ink-secondary);
  }

  li span:first-child {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .size {
    color: var(--ink-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .more {
    color: var(--ink-tertiary);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions .button {
    height: 30px;
  }

  .button.danger {
    background: var(--danger);
    color: var(--on-accent);
  }
</style>
