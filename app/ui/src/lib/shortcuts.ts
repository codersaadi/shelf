export type ShortcutAction =
  | 'search'
  | 'quickLook'
  | 'inspector'
  | 'grid'
  | 'list'
  | 'delete'
  | 'selectAll'
  | 'openFolder'
  | 'copyPath';

export const SHORTCUT_LABELS: Record<ShortcutAction, string> = {
  search: 'Search',
  quickLook: 'Quick Look',
  inspector: 'Inspector',
  grid: 'Grid view',
  list: 'List view',
  delete: 'Move to Trash',
  selectAll: 'Select all',
  openFolder: 'Open in folder',
  copyPath: 'Copy path',
};

export const DEFAULT_SHORTCUTS: Record<ShortcutAction, string> = {
  search: 'ctrl+f',
  quickLook: 'space',
  inspector: 'ctrl+i',
  grid: 'ctrl+1',
  list: 'ctrl+2',
  delete: 'delete',
  selectAll: 'ctrl+a',
  openFolder: 'ctrl+enter',
  copyPath: 'ctrl+shift+c',
};

/** Normalise a key event to a stable, comparable chord string. */
export function comboOf(event: KeyboardEvent): string {
  const key = event.key.toLowerCase();
  if (['control', 'meta', 'alt', 'shift'].includes(key)) return '';
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('ctrl');
  if (event.metaKey) parts.push('meta');
  if (event.altKey) parts.push('alt');
  if (event.shiftKey) parts.push('shift');
  parts.push(key === ' ' ? 'space' : key);
  return parts.join('+');
}

export function prettyCombo(combo: string): string {
  if (!combo) return '—';
  return combo
    .split('+')
    .map((part) => {
      if (part === 'ctrl') return 'Ctrl';
      if (part === 'meta') return '⌘';
      if (part === 'alt') return 'Alt';
      if (part === 'shift') return 'Shift';
      if (part === 'space') return 'Space';
      if (part === 'enter') return 'Enter';
      if (part === 'delete') return 'Delete';
      if (part.length === 1) return part.toUpperCase();
      return part[0].toUpperCase() + part.slice(1);
    })
    .join(' + ');
}

export function shortcutKeys(): ShortcutAction[] {
  return Object.keys(DEFAULT_SHORTCUTS) as ShortcutAction[];
}
