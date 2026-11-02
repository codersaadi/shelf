import { invoke } from '@tauri-apps/api/core';

export type FileKind = 'image' | 'video' | 'music' | 'document' | 'archive' | 'app' | 'other';
export type Sort = 'modified' | 'added' | 'size' | 'name' | 'kind' | 'frecency';
export type Risk = 'low' | 'medium' | 'high';

export interface FileMeta {
  id: number;
  path: string;
  parent: string;
  name: string;
  ext: string;
  kind: FileKind;
  size: number;
  mtime: number;
  added_at: number;
  duration_ms?: number;
  is_screenshot: boolean;
  origin_url?: string;
  origin_app?: string;
  origin_host?: string;
  in_trash: boolean;
  trash_path?: string;
  opens?: number;
}

export interface Page {
  items: FileMeta[];
  next_cursor: string | null;
}

export interface Chip {
  kind: 'kind' | 'date' | 'size' | 'source' | 'place' | 'tag';
  label: string;
  description: string;
}

export interface ParsedQuery {
  chips: Chip[];
  terms: string[];
}

export interface KindStat {
  kind: FileKind;
  count: number;
  bytes: number;
}

export interface HomeShelfData {
  kind: FileKind | 'skill';
  files: FileMeta[];
  count: number;
  bytes: number;
}

export interface HomePayload {
  recent: FileMeta[];
  shelves: HomeShelfData[];
}

export interface Counts {
  kinds: KindStat[];
  total: number;
  bytes: number;
  trash: number;
  trash_bytes: number;
  reclaimable: number;
}

export interface SavedSearch {
  id: number;
  name: string;
  query: string;
  created: number;
}

export interface NativeIcon {
  data_url: string;
}

export interface TextSnippet {
  text: string;
  html?: string;
  truncated: boolean;
}

export interface Inspect {
  file: FileMeta;
  tags: string[];
  on_disk: boolean;
  width?: number;
  height?: number;
  created?: number;
  text_preview?: string;
}

export interface DuplicateGroup {
  size: number;
  wasted: number;
  files: FileMeta[];
}

export interface Finding {
  id: string;
  title: string;
  reason: string;
  bytes: number;
  count: number;
  risk: Risk;
  command?: string;
  truncated: boolean;
  files: FileMeta[];
}

export interface MovedFile {
  path: string;
  trash_path: string;
}

export interface TrashOutcome {
  receipt_id: number;
  moved: MovedFile[];
  bytes: number;
}

export interface TrashItem {
  original: string;
  trash_path: string;
  name: string;
  ext: string;
  kind: FileKind;
  size: number;
  mtime: number;
  is_dir: boolean;
}

export interface FolderEntry {
  name: string;
  path: string;
  mtime: number;
}

export interface KnownLocation {
  name: string;
  path: string;
}

export interface FolderInfo {
  name: string;
  path: string;
  is_codebase: boolean;
  score: number;
  signals: string[];
}

export interface SettingsInfo {
  roots: string[];
  db: string;
  home: string;
}

export interface DefaultApp {
  id: string;
  name: string;
  default: boolean;
}

export interface DefaultCategory {
  mime: string;
  label: string;
  default_app?: string;
  apps: DefaultApp[];
}

export interface OpenWithApp {
  id: string;
  name: string;
  default: boolean;
}

export interface ListRequest {
  view?: string;
  kind?: string;
  exts?: string[];
  mtime_from?: number;
  mtime_to?: number;
  text?: string;
  sort?: Sort;
  limit?: number;
  cursor?: string | null;
}

export const ipc = {
  listShelf: (req: ListRequest) => invoke<Page>('list_shelf', { req }),
  search: (text: string, view: string | null, sort: Sort, limit = 120, cursor: string | null = null) =>
    invoke<Page>('search', { text, view, sort, limit, cursor }),
  parseQuery: (text: string) => invoke<ParsedQuery>('parse_query', { text }),
  inspect: (path: string) => invoke<Inspect>('inspect', { path }),
  textSnippet: (path: string) => invoke<TextSnippet | null>('text_snippet', { path }),
  nativeIcon: (path: string) => invoke<NativeIcon | null>('native_icon', { path }),
  counts: () => invoke<Counts>('counts'),
  home: () => invoke<HomePayload>('home'),
  stats: () => invoke<KindStat[]>('stats'),
  getFiles: (ids: number[]) => invoke<FileMeta[]>('get_files', { ids }),
  trash: (paths: string[]) => invoke<TrashOutcome>('trash', { paths }),
  undoTrash: (receiptId: number) => invoke<number>('undo_trash', { receiptId }),
  cleanupFindings: () => invoke<Finding[]>('cleanup_findings'),
  duplicates: () => invoke<DuplicateGroup[]>('duplicates'),
  showInFiles: (paths: string[]) => invoke<void>('show_in_files', { paths }),
  openFile: (path: string, record = true) => invoke<void>('open_file', { path, record }),
  clearActivity: () => invoke<number>('clear_activity'),
  openWithApps: (path: string) => invoke<OpenWithApp[]>('open_with_apps', { path }),
  openWith: (path: string, app: string) => invoke<void>('open_with', { path, app }),
  setDefaultApp: (path: string, app: string) => invoke<void>('set_default_app', { path, app }),
  openTerminal: (path: string) => invoke<void>('open_terminal', { path }),
  transfer: (paths: string[], dest: string, mode: 'move' | 'copy') =>
    invoke<string[]>('transfer_paths', { paths, dest, mode }),
  settingsInfo: () => invoke<SettingsInfo>('settings_info'),
  defaultApps: () => invoke<DefaultCategory[]>('default_apps'),
  setDefaultMime: (mime: string, app: string) => invoke<void>('set_default_mime', { mime, app }),
  openDataFolder: () => invoke<void>('open_data_folder'),
  pathIsDir: (path: string) => invoke<boolean>('path_is_dir', { path }),
  addIndexRoot: (path: string) => invoke<void>('add_index_root', { path }),
  setUiZoom: (scale: number) => invoke<void>('set_ui_zoom', { scale }),
  closeWindow: () => invoke<void>('close_window'),
  knownLocations: () => invoke<KnownLocation[]>('known_locations'),
  folderInfo: (path: string) => invoke<FolderInfo>('folder_info', { path }),
  themeFile: () => invoke<string>('theme_file'),
  readTheme: () => invoke<string>('read_theme'),
  openThemeFile: () => invoke<void>('open_theme_file'),
  childFolders: (path?: string | null) =>
    invoke<FolderEntry[]>('child_folders', { path: path ?? null }),
  trashEntries: () => invoke<TrashItem[]>('trash_entries'),
  renamePath: (path: string, name: string) => invoke<string>('rename_path', { path, name }),
  restoreTrash: (trashPath: string, original: string) =>
    invoke<void>('restore_trash', { trashPath, original }),
  purgeTrash: (paths: string[]) => invoke<number>('purge_trash', { paths }),
  tagsOf: (path: string) => invoke<string[]>('tags_of', { path }),
  setTags: (path: string, tags: string[]) => invoke<void>('set_tags', { path, tags }),
  savedSearches: () => invoke<SavedSearch[]>('saved_searches'),
  saveSearch: (name: string, query: string) => invoke<number>('save_search', { name, query }),
  deleteSavedSearch: (id: number) => invoke<void>('delete_saved_search', { id }),
  indexRoot: (root: string | null = null) => invoke<void>('index_root', { root }),
};

/** Quantise requests to a few cache sizes so resizing does not re-decode. */
export function thumbBucket(size: number): number {
  const px = Math.round(size);
  if (px <= 128) return 128;
  if (px <= 256) return 256;
  if (px <= 512) return 512;
  return 1024;
}

export function thumbUrl(file: FileMeta, size: number, retry = 0): string {
  return `thumb://localhost/${file.id}?s=${thumbBucket(size)}&v=${file.mtime}&r=${retry}`;
}

/** Full original bytes for Quick Look images; other formats keep thumbnails. */
export function fullUrl(file: FileMeta, retry = 0): string {
  return `thumb://localhost/${file.id}?full=1&v=${file.mtime}&r=${retry}`;
}
