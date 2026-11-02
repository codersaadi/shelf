<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    value,
    options,
    placeholder = 'System default',
    disabled = false,
    label = 'Choose option',
    onchange,
  }: {
    value: string;
    options: { value: string; label: string }[];
    placeholder?: string;
    disabled?: boolean;
    label?: string;
    onchange: (value: string) => void;
  } = $props();

  let open = $state(false);
  let root: HTMLDivElement | null = $state(null);
  let selected = $derived(options.find((option) => option.value === value));

  function choose(next: string): void {
    open = false;
    if (next !== value) onchange(next);
  }

  onMount(() => {
    const down = (event: PointerEvent): void => {
      if (open && root && !root.contains(event.target as Node)) open = false;
    };
    const key = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') open = false;
    };
    window.addEventListener('pointerdown', down);
    window.addEventListener('keydown', key);
    return () => {
      window.removeEventListener('pointerdown', down);
      window.removeEventListener('keydown', key);
    };
  });
</script>

<div class="select-wrap" bind:this={root}>
  <button
    class="select"
    class:open
    type="button"
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={label}
    onclick={() => (open = !open)}
  >
    <span class="value" class:placeholder={!selected}>{selected?.label ?? placeholder}</span>
    <Icon name="chevron-down" size={12} />
  </button>
  {#if open}
    <div class="menu" role="listbox" aria-label={label}>
      {#each options as option (option.value)}
        <button
          class="option"
          class:selected={option.value === value}
          role="option"
          aria-selected={option.value === value}
          onclick={() => choose(option.value)}
        >
          <span>{option.label}</span>
          {#if option.value === value}<Icon name="check" size={12} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .select-wrap {
    position: relative;
    flex: 0 0 auto;
  }

  .select {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-width: 190px;
    max-width: 260px;
    height: 30px;
    padding: 0 8px 0 10px;
    border: 1px solid var(--line-hairline);
    border-radius: 7px;
    background: var(--surface-content);
    color: var(--ink-primary);
    font-size: 11.5px;
    text-align: left;
    transition: border-color 120ms var(--ease-hover);
  }

  .select:hover:not(:disabled),
  .select.open {
    border-color: color-mix(in srgb, var(--ink-primary) 22%, var(--line-hairline));
  }

  .select:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .select:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .value.placeholder {
    color: var(--ink-tertiary);
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: max(100%, 220px);
    max-height: 260px;
    overflow-y: auto;
    padding: 4px;
    border: 1px solid var(--line-hairline);
    border-radius: 8px;
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }

  .option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 28px;
    padding: 0 8px;
    border-radius: 5px;
    color: var(--ink-secondary);
    font-size: 11.5px;
    text-align: left;
  }

  .option:hover {
    background: var(--surface-hover);
    color: var(--ink-primary);
  }

  .option.selected {
    color: var(--ink-primary);
  }

  .option span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
