import type { Chip, FileKind, Risk, SavedSearch, Sort } from './ipc';

const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
];

/** Middle truncation keeps the extension visible: `Screenshot 2026-…-14.32.png`. */
export function middleTruncate(name: string, max = 34): string {
  const chars = [...name];
  if (chars.length <= max) return name;
  const dot = name.lastIndexOf('.');
  const ext = dot > 0 && name.length - dot <= 9 ? name.slice(dot) : '';
  const keep = Math.max(3, max - [...ext].length - 1);
  const head = [...name.slice(0, name.length - ext.length)].slice(0, keep).join('').trimEnd();
  return `${head}…${ext}`;
}

export function bytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return '—';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = n;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  if (unit === 0) return `${Math.round(value)} B`;
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const s = total % 60;
  const m = Math.floor(total / 60) % 60;
  const h = Math.floor(total / 3600);
  return h > 0
    ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
    : `${m}:${String(s).padStart(2, '0')}`;
}

export function count(n: number): string {
  return n.toLocaleString('en-US');
}

export function plural(n: number, one: string, many = `${one}s`): string {
  return n === 1 ? one : many;
}

export function relativeDate(unix: number): string {
  const diff = Date.now() / 1000 - unix;
  if (diff < 60) return 'Just now';
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) {
    const h = Math.floor(diff / 3600);
    return `${h} ${plural(h, 'hour')} ago`;
  }
  if (diff < 7 * 86400) {
    const d = Math.floor(diff / 86400);
    return `${d} ${plural(d, 'day')} ago`;
  }
  return shortDate(unix);
}

export function shortDate(unix: number): string {
  const d = new Date(unix * 1000);
  return `${d.getDate()} ${MONTHS[d.getMonth()]} ${d.getFullYear()}`;
}

export function dateTime(unix: number): string {
  const d = new Date(unix * 1000);
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  return `${shortDate(unix)}, ${hh}:${mm}`;
}

export function kindLabel(kind: FileKind): string {
  const map: Record<FileKind, string> = {
    image: 'Image',
    video: 'Video',
    music: 'Audio',
    document: 'Document',
    archive: 'Archive',
    app: 'App',
    other: 'File',
  };
  return map[kind] ?? 'File';
}

export function kindPlural(kind: FileKind | 'skill'): string {
  const map: Record<FileKind | 'skill', string> = {
    image: 'Images',
    video: 'Videos',
    music: 'Music',
    document: 'Documents',
    archive: 'Archives',
    app: 'Apps',
    other: 'Other',
    skill: 'Skills',
  };
  return map[kind] ?? 'Files';
}

export function kindIcon(
  kind: FileKind,
): 'image' | 'video' | 'music' | 'document' | 'archive' | 'app' | 'document' {
  switch (kind) {
    case 'image':
      return 'image';
    case 'video':
      return 'video';
    case 'music':
      return 'music';
    case 'archive':
      return 'archive';
    case 'app':
      return 'app';
    default:
      return 'document';
  }
}

export function riskLabel(risk: Risk): string {
  return risk === 'low' ? 'Low risk' : risk === 'medium' ? 'Medium risk' : 'High risk';
}

export function sortLabel(sort: Sort): string {
  const map: Record<Sort, string> = {
    modified: 'Date modified',
    added: 'Date added',
    size: 'Size',
    name: 'Name',
    kind: 'Kind',
    frecency: 'Often opened',
  };
  return map[sort] ?? sort;
}

export function viewTitle(view: string, opts: { saved?: SavedSearch[] } = {}): string {
  if (view.startsWith('kind:')) {
    return kindPlural(view.slice(5) as FileKind);
  }
  if (view.startsWith('place:')) {
    const parts = view.slice(6).split('/').filter(Boolean);
    return parts.length ? parts[parts.length - 1] : 'Location';
  }
  switch (view) {
    case 'home':
      return 'Home';
    case 'downloads':
      return 'Downloads';
    case 'screenshots':
      return 'Screenshots';
    case 'recent':
      return 'Recent';
    case 'cleanup':
      return 'Clean Up';
    case 'trash':
      return 'Trash';
    case 'settings':
      return 'Settings';
    case 'search':
      return 'Search';
    default: {
      const saved = opts.saved?.find((s) => `saved:${s.id}` === view);
      return saved ? saved.name : 'Shelf';
    }
  }
}

export function folderName(path: string): string {
  const parts = path.split('/').filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
}

export function parentPath(path: string): string {
  const idx = path.lastIndexOf('/');
  return idx > 0 ? path.slice(0, idx) : '/';
}

/** Remove one interpreted chip from the raw query, keeping the rest of the text. */
export function removeChipText(text: string, chip: Chip): string {
  const lower = text.toLowerCase();
  const label = chip.label.toLowerCase();
  let index = lower.indexOf(label);
  let end = index + label.length;
  if (index === -1 && label.endsWith('s')) {
    index = lower.indexOf(label.slice(0, -1));
    end = index + label.length - 1;
  }
  if (index === -1) {
    // Parser labels and input can disagree on inflection ("screenshots").
    const stem = label.replace(/e?s$/, '');
    index = lower.indexOf(stem);
    end = index + stem.length;
  }
  if (index === -1) return text;
  if (text[end]?.toLowerCase() === 's') end += 1;
  return `${text.slice(0, index)} ${text.slice(end)}`.replace(/\s+/g, ' ').trim();
}

export function isImage(file: { kind: FileKind }): boolean {
  return file.kind === 'image';
}

export function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return 'Good night';
  if (h < 12) return 'Good morning';
  if (h < 18) return 'Good afternoon';
  return 'Good evening';
}
