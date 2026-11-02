import type { FileKind } from './ipc';

export type FileFamily =
  | 'document'
  | 'spreadsheet'
  | 'presentation'
  | 'code'
  | 'archive'
  | 'image'
  | 'video'
  | 'audio'
  | 'font'
  | 'database'
  | 'application'
  | 'unknown';

export type FileGlyph =
  | 'document'
  | 'spreadsheet'
  | 'presentation'
  | 'code'
  | 'archive'
  | 'image'
  | 'video'
  | 'audio'
  | 'font'
  | 'database'
  | 'application'
  | 'file';

export interface FileTypeInfo {
  family: FileFamily;
  glyph: FileGlyph;
  /** Small secondary label, max four characters plus compound dots. */
  label: string;
  /** Full extension when it differs from the label, for tooltips. */
  extension: string;
  description: string;
}

const DOCS = new Set([
  'doc',
  'docx',
  'odt',
  'rtf',
  'txt',
  'md',
  'mdx',
  'markdown',
  'tex',
  'epub',
  'djvu',
  'docm',
  'pages',
]);
const SHEETS = new Set(['xls', 'xlsx', 'ods', 'csv', 'tsv', 'numbers', 'xlsm']);
const SLIDES = new Set(['ppt', 'pptx', 'odp', 'key', 'pptm']);
const ARCHIVES = new Set([
  'zip',
  'tar',
  'gz',
  'bz2',
  'xz',
  'zst',
  '7z',
  'rar',
  'tgz',
  'tbz2',
  'txz',
  'tzst',
  'tar.gz',
  'tar.bz2',
  'tar.xz',
  'tar.zst',
  'tar.lz',
  'tar.lz4',
]);
const APPS = new Set(['deb', 'rpm', 'appimage', 'flatpak', 'snap', 'exe', 'msi']);
const CODE = new Set([
  'py',
  'pyc',
  'pyo',
  'js',
  'mjs',
  'cjs',
  'ts',
  'tsx',
  'jsx',
  'rs',
  'go',
  'java',
  'kt',
  'kts',
  'c',
  'h',
  'cc',
  'cpp',
  'hpp',
  'cs',
  'rb',
  'php',
  'sh',
  'bash',
  'zsh',
  'fish',
  'lua',
  'pl',
  'swift',
  'scala',
  'sql',
  'json',
  'jsonl',
  'toml',
  'yaml',
  'yml',
  'xml',
  'html',
  'htm',
  'css',
  'scss',
  'sass',
  'less',
  'vue',
  'svelte',
  'astro',
  'ipynb',
  'lock',
  'ini',
  'cfg',
  'conf',
  'env',
  'gitignore',
  'npmrc',
  'editorconfig',
  'bashrc',
  'zshrc',
  'profile',
]);
const FONTS = new Set(['ttf', 'otf', 'woff', 'woff2', 'ttc']);
const DATABASES = new Set(['db', 'sqlite', 'sqlite3', 'mdb']);
const IMAGES = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tif', 'tiff', 'heic', 'heif', 'avif', 'jfif', 'svg']);
const VIDEOS = new Set(['mp4', 'mkv', 'mov', 'avi', 'webm', 'm4v', 'mpg', 'mpeg', 'wmv', 'flv', 'm2ts', '3gp', 'ogv']);
const AUDIO = new Set(['mp3', 'flac', 'wav', 'ogg', 'oga', 'm4a', 'aac', 'opus', 'wma', 'aiff', 'mid']);
const COMPOUND = ['tar.gz', 'tar.bz2', 'tar.xz', 'tar.zst', 'tar.lz', 'tar.lz4'];

const SPECIAL_LABELS: Record<string, string> = {
  appimage: 'APP',
  flatpak: 'APP',
  sqlite3: 'DB',
  sqlite: 'DB',
  markdown: 'MD',
  jpeg: 'JPG',
  tiff: 'TIF',
  'tar.gz': 'TAR.GZ',
  'tar.bz2': 'TAR.BZ2',
  'tar.xz': 'TAR.XZ',
  'tar.zst': 'TAR.ZST',
  'tar.lz': 'TAR.LZ',
  'tar.lz4': 'TAR.LZ4',
};

/** Return a compound archive suffix, a dotfile's suffix, or the last suffix. */
export function extensionOf(name: string): string {
  const base = name.split('/').pop() ?? name;
  const lower = base.toLowerCase();
  for (const compound of COMPOUND) {
    if (lower.endsWith(`.${compound}`)) return compound;
  }
  const dot = lower.lastIndexOf('.');
  if (dot === -1 || dot === lower.length - 1) return '';
  if (dot === 0) {
    const rest = lower.slice(1);
    const inner = rest.lastIndexOf('.');
    return inner === -1 ? rest : rest.slice(inner + 1);
  }
  return lower.slice(dot + 1);
}

function labelFor(ext: string): string {
  if (!ext) return '';
  const special = SPECIAL_LABELS[ext];
  if (special) return special;
  if (ext.length <= 4) return ext.toUpperCase();
  return ext.slice(0, 4).toUpperCase();
}

/** Semantic family for a file. Colour follows the family, never the extension. */
export function fileTypeInfo(name: string, kind: FileKind): FileTypeInfo {
  const ext = extensionOf(name);
  const make = (family: FileFamily, glyph: FileGlyph, text: string): FileTypeInfo => ({
    family,
    glyph,
    label: labelFor(ext),
    extension: ext,
    description: text,
  });

  if (ext === 'pdf') return make('document', 'document', 'PDF document');
  if (DOCS.has(ext)) return make('document', 'document', 'Document');
  if (SHEETS.has(ext)) return make('spreadsheet', 'spreadsheet', 'Spreadsheet');
  if (SLIDES.has(ext)) return make('presentation', 'presentation', 'Presentation');
  if (ARCHIVES.has(ext)) return make('archive', 'archive', 'Archive');
  if (APPS.has(ext)) return make('application', 'application', 'Application package');
  if (CODE.has(ext)) {
    return make('code', 'code', 'Code file');
  }
  if (FONTS.has(ext)) return make('font', 'font', 'Font');
  if (DATABASES.has(ext)) return make('database', 'database', 'Database');
  if (IMAGES.has(ext)) return make('image', 'image', 'Image');
  if (VIDEOS.has(ext)) return make('video', 'video', 'Video');
  if (AUDIO.has(ext)) return make('audio', 'audio', 'Audio');

  if (kind === 'image') return make('image', 'image', 'Image');
  if (kind === 'video') return make('video', 'video', 'Video');
  if (kind === 'music') return make('audio', 'audio', 'Audio');
  if (kind === 'archive') return make('archive', 'archive', 'Archive');
  if (kind === 'app') return make('application', 'application', 'Application');
  return make('unknown', 'file', 'File');
}
