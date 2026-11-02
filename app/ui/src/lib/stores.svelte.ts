import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { loadThemeFile } from './theme';
import {
  ipc,
  type Chip,
  type Counts,
  type DuplicateGroup,
  type FileKind,
  type FileMeta,
  type Finding,
  type FolderEntry,
  type FolderInfo,
  type Inspect,
  type SavedSearch,
  type Sort,
  type TrashItem,
  type TrashOutcome,
} from './ipc';
import { kindPlural, plural, removeChipText, viewTitle } from './format';
import { DATE_OPTIONS, dateFrom as computeDateFrom, documentType, type DateFilter } from './filters';
import { DEFAULT_SHORTCUTS, type ShortcutAction } from './shortcuts';

export interface Toast {
  id: number;
  message: string;
  undo?: number;
}

export interface Location {
  name: string;
  path: string;
}

export interface HomeShelf {
  kind: FileKind | 'skill';
  files: FileMeta[];
  count: number;
  bytes: number;
}

export interface HomeData {
  recent: FileMeta[];
  shelves: HomeShelf[];
}

interface HistoryEntry {
  view: string;
  text: string;
}

export interface ListColumns {
  date: boolean;
  size: boolean;
  kind: boolean;
  from: boolean;
}

interface Prefs {
  viewMode: 'grid' | 'list';
  cell: number;
  sort: Sort;
  inspectorOpen: boolean;
  columns?: ListColumns;
  shortcuts?: Record<string, string>;
  theme?: 'system' | 'light' | 'dark';
  zoom?: number;
  trackOpens?: boolean;
}

const LOCATIONS_KEY = 'shelf.locations.v1';

const EMPTY_COUNTS: Counts = {
  kinds: [],
  total: 0,
  bytes: 0,
  trash: 0,
  trash_bytes: 0,
  reclaimable: 0,
};
const PREFS_KEY = 'shelf.prefs.v1';
const HOME_CACHE_KEY = 'shelf.home.v1';

export class ShelfStore {
  view = $state('home');
  text = $state('');
  chips = $state<Chip[]>([]);
  terms = $state<string[]>([]);
  items = $state.raw<FileMeta[]>([]);
  cursor = $state<string | null>(null);
  loading = $state(true);
  loadingMore = $state(false);
  counts = $state<Counts>(EMPTY_COUNTS);
  saved = $state<SavedSearch[]>([]);
  selection = $state<string[]>([]);
  viewMode = $state<'grid' | 'list'>('grid');
  cell = $state(176);
  sort = $state<Sort>('modified');
  inspectorOpen = $state(true);
  listColumns = $state<ListColumns>({ date: true, size: true, kind: true, from: true });
  shortcuts = $state<Record<ShortcutAction, string>>({ ...DEFAULT_SHORTCUTS });
  theme = $state<'system' | 'light' | 'dark'>('system');
  zoom = $state(1);
  trackOpens = $state(true);
  locations = $state<Location[]>([]);
  filterKind = $state<FileKind | null>(null);
  filterDate = $state<DateFilter>('any');
  filterType = $state('all');
  inspector = $state<Inspect | null>(null);
  home = $state.raw<HomeData | null>(null);
  findings = $state.raw<Finding[]>([]);
  duplicates = $state.raw<DuplicateGroup[]>([]);
  duplicatesLoading = $state(false);
  folders = $state.raw<FolderEntry[]>([]);
  folderInfo = $state<FolderInfo | null>(null);
  findingsLoading = $state(false);
  indexing = $state<string | null>(null);
  toast = $state<Toast | null>(null);
  trashPrompt = $state<{ paths: string[]; bytes: number } | null>(null);
  renamePrompt = $state<{ path: string; name: string } | null>(null);
  purgePrompt = $state<string[] | null>(null);
  transferPrompt = $state<{ paths: string[]; mode: 'move' | 'copy' } | null>(null);
  paletteOpen = $state(false);
  quickSearchOpen = $state(false);
  quickLookOpen = $state(false);
  sidebarOpen = $state(true);
  online = $state(true);

  history = $state<HistoryEntry[]>([{ view: 'home', text: '' }]);
  historyIndex = $state(0);

  private loadSeq = 0;
  private inspectTimer: ReturnType<typeof setTimeout> | null = null;
  private parseTimer: ReturnType<typeof setTimeout> | null = null;
  private reloadTimer: ReturnType<typeof setTimeout> | null = null;
  private toastTimer: ReturnType<typeof setTimeout> | null = null;
  private started = false;
  private unlisten: UnlistenFn[] = [];

  get selectedFiles(): FileMeta[] {
    const chosen = new Set(this.selection);
    return this.items.filter((f) => chosen.has(f.path));
  }

  get currentFolder(): string | null {
    return this.view.startsWith('place:') ? this.view.slice(6) : null;
  }

  get selectedFile(): FileMeta | null {
    return this.selectedFiles[0] ?? this.inspector?.file ?? null;
  }

  get title(): string {
    if (this.text.trim() && !this.view.startsWith('saved:')) return 'Search';
    return viewTitle(this.view, { saved: this.saved });
  }

  get totalCount(): number {
    return this.counts.total;
  }

  get fixedKind(): FileKind | null {
    if (this.view === 'screenshots') return 'image';
    if (!this.view.startsWith('kind:')) return null;
    const kind = this.view.slice(5);
    return ['image', 'video', 'music', 'document', 'archive', 'app', 'other'].includes(kind)
      ? (kind as FileKind)
      : null;
  }

  get activeKind(): FileKind | null {
    return this.fixedKind ?? this.filterKind;
  }

  get showKindFilter(): boolean {
    return this.fixedKind === null;
  }

  get showTypeFilter(): boolean {
    return this.activeKind === 'document';
  }

  get filterExts(): string[] {
    return this.filterType === 'all' ? [] : documentType(this.filterType).exts;
  }

  get dateFrom(): number | undefined {
    return computeDateFrom(this.filterDate);
  }

  get filtersActive(): boolean {
    const kindActive = this.showKindFilter && this.filterKind !== null;
    const typeActive = this.showTypeFilter && this.filterType !== 'all';
    return kindActive || this.filterDate !== 'any' || typeActive;
  }

  get filterCount(): number {
    return (
      (this.showKindFilter && this.filterKind !== null ? 1 : 0) +
      (this.filterDate !== 'any' ? 1 : 0) +
      (this.showTypeFilter && this.filterType !== 'all' ? 1 : 0)
    );
  }

  get filterSummary(): string {
    const parts: string[] = [];
    if (this.showKindFilter && this.filterKind) parts.push(kindPlural(this.filterKind));
    const date = DATE_OPTIONS.find((d) => d.value === this.filterDate);
    if (date && date.value !== 'any') parts.push(date.label);
    if (this.showTypeFilter && this.filterType !== 'all') {
      parts.push(documentType(this.filterType).label);
    }
    return parts.join(' · ');
  }

  countOf(kind: FileKind): number {
    return this.counts.kinds.find((k) => k.kind === kind)?.count ?? 0;
  }

  bytesOf(kind: FileKind): number {
    return this.counts.kinds.find((k) => k.kind === kind)?.bytes ?? 0;
  }

  async init(): Promise<void> {
    if (this.started) return;
    this.started = true;
    this.loadPrefs();
    this.applyTheme();
    void loadThemeFile().catch(() => {});
    this.loadLocations();
    void ipc.setUiZoom(this.zoom).catch(() => {});
    this.loadHomeCache();
    await Promise.all([this.loadCounts(), this.loadSaved()]);
    await this.refresh();
    await this.listen();
  }

  async listen(): Promise<void> {
    try {
      this.unlisten.push(
        await listen<{ phase: string; done?: number; files?: number; message?: string; first?: boolean }>(
          'shelf://index',
          (event) => {
            const p = event.payload;
            if (p.phase === 'start') {
              this.indexing = 'Indexing files';
            } else if (p.phase === 'progress') {
              this.indexing = `Indexing ${(p.done ?? 0).toLocaleString('en-US')} files`;
            } else if (p.phase === 'done') {
              this.indexing = null;
              if (p.first) this.showToast(`Indexed ${(p.files ?? 0).toLocaleString('en-US')} files.`);
              void this.loadCounts();
              void this.refresh();
            } else {
              this.indexing = null;
              this.showToast(p.message ?? 'Indexing stopped unexpectedly.');
            }
          },
        ),
        await listen('shelf://files-changed', () => {
          if (this.reloadTimer) clearTimeout(this.reloadTimer);
          this.reloadTimer = setTimeout(() => {
            void this.loadCounts();
            void this.refresh();
            void this.loadSaved();
          }, 350);
        }),
      );
    } catch {
      // Browser preview: no Tauri event bridge. The app still renders empty states.
    }
  }

  destroy(): void {
    for (const off of this.unlisten) off();
    this.unlisten = [];
  }

  async refresh(): Promise<void> {
    if (this.text.trim() || !['home', 'cleanup', 'settings', 'duplicates'].includes(this.view)) {
      await this.loadPage(true);
      return;
    }
    if (this.view === 'cleanup') {
      await this.loadFindings();
      return;
    }
    if (this.view === 'duplicates') {
      await this.loadDuplicates();
      return;
    }
    await this.loadHome();
  }

  async loadPage(reset: boolean): Promise<void> {
    if (this.view === 'trash' && !this.text.trim()) {
      if (reset) await this.loadTrash();
      return;
    }
    if (this.view !== 'downloads' && !this.view.startsWith('place:')) this.folders = [];
    if (reset && !this.text.trim()) {
      if (this.view === 'downloads') void this.loadFolders(null);
      else if (this.view.startsWith('place:')) {
        void this.loadFolders(this.view.slice(6));
        void this.loadFolderInfo(this.view.slice(6));
      }
    }
    if (!this.view.startsWith('place:')) this.folderInfo = null;
    if (reset) {
      this.loading = true;
      this.cursor = null;
      this.selection = [];
      this.inspector = null;
    } else {
      if (!this.cursor || this.loadingMore) return;
      this.loadingMore = true;
    }
    const seq = ++this.loadSeq;
    try {
      const page = await ipc.listShelf({
        view: this.view,
        kind: this.activeKind ?? undefined,
        exts: this.filterExts.length ? this.filterExts : undefined,
        mtime_from: this.dateFrom,
        text: this.text.trim() || undefined,
        sort: this.sort,
        limit: 200,
        cursor: reset ? null : this.cursor,
      });
      if (seq !== this.loadSeq) return;
      this.items = reset ? page.items : [...this.items, ...page.items];
      this.cursor = page.next_cursor;
    } catch (e) {
      if (seq === this.loadSeq) this.showToast(errorText(e));
    } finally {
      if (seq === this.loadSeq) {
        this.loading = false;
        this.loadingMore = false;
      }
    }
  }

  async loadMore(): Promise<void> {
    await this.loadPage(false);
  }

  async loadFolderInfo(path: string): Promise<void> {
    try {
      const info = await ipc.folderInfo(path);
      if (this.view === `place:${path}`) this.folderInfo = info;
    } catch {
      if (this.view === `place:${path}`) this.folderInfo = null;
    }
  }

  async openTerminal(path: string): Promise<void> {
    try {
      await ipc.openTerminal(path);
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async loadFolders(path: string | null): Promise<void> {
    try {
      this.folders = await ipc.childFolders(path);
    } catch {
      this.folders = [];
    }
  }

  async loadTrash(): Promise<void> {
    const seq = ++this.loadSeq;
    this.loading = true;
    this.cursor = null;
    this.selection = [];
    this.inspector = null;
    try {
      const entries: TrashItem[] = await ipc.trashEntries();
      if (seq !== this.loadSeq) return;
      this.items = entries.map((entry, i) => ({
        id: -(i + 1),
        path: entry.original,
        parent: entry.original.slice(0, Math.max(0, entry.original.lastIndexOf('/'))),
        name: entry.name,
        ext: entry.ext,
        kind: entry.kind,
        size: entry.size,
        mtime: entry.mtime,
        added_at: entry.mtime,
        is_screenshot: false,
        in_trash: true,
        trash_path: entry.trash_path,
      }));
      this.counts = {
        ...this.counts,
        trash: entries.length,
        trash_bytes: entries.reduce((n, entry) => n + entry.size, 0),
      };
    } catch (e) {
      if (seq === this.loadSeq) this.showToast(errorText(e));
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  async loadHome(): Promise<void> {
    this.loading = true;
    this.selection = [];
    this.inspector = null;
    const seq = ++this.loadSeq;
    try {
      const payload = await ipc.home();
      if (seq !== this.loadSeq) return;
      this.home = { recent: payload.recent, shelves: payload.shelves };
      this.saveHomeCache();
    } catch (e) {
      if (seq === this.loadSeq) {
        this.showToast(errorText(e));
        this.home = { recent: [], shelves: [] };
      }
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
  }

  async loadCounts(): Promise<void> {
    try {
      this.counts = await ipc.counts();
    } catch {
      // Keep the last known counts while the index is unavailable.
    }
  }

  async loadSaved(): Promise<void> {
    try {
      this.saved = await ipc.savedSearches();
    } catch {
      this.saved = [];
    }
  }

  async loadDuplicates(): Promise<void> {
    if (this.duplicatesLoading) return;
    this.duplicatesLoading = true;
    try {
      this.duplicates = await ipc.duplicates();
    } catch (e) {
      this.showToast(errorText(e));
      this.duplicates = [];
    } finally {
      this.duplicatesLoading = false;
    }
  }

  async loadFindings(): Promise<void> {
    this.findingsLoading = true;
    try {
      this.findings = await ipc.cleanupFindings();
    } catch (e) {
      this.showToast(errorText(e));
      this.findings = [];
    } finally {
      this.findingsLoading = false;
    }
  }

  async setView(view: string): Promise<void> {
    if (this.view === view && !this.text) return;
    this.view = view;
    this.sort = view === 'recent' || view === 'downloads' ? 'added' : view.startsWith('place:') || view === 'skills' ? 'name' : 'modified';
    this.text = '';
    this.chips = [];
    this.terms = [];
    this.selection = [];
    this.inspector = null;
    this.folders = [];
    this.folderInfo = null;
    this.resetFilters();
    this.pushHistory();
    await this.refresh();
  }

  async openSaved(search: SavedSearch): Promise<void> {
    this.view = `saved:${search.id}`;
    this.text = search.query;
    this.selection = [];
    this.inspector = null;
    this.resetFilters();
    this.pushHistory();
    await this.reparse();
    await this.refresh();
  }

  setText(text: string): void {
    const wasSearching = this.text.trim().length > 0;
    const searching = text.trim().length > 0;
    if (!wasSearching && searching && this.sort === 'modified') {
      // Results rank by what the user actually opens.
      this.sort = 'frecency';
    } else if (wasSearching && !searching && this.sort === 'frecency') {
      this.sort = 'modified';
    }
    this.text = text;
    if (this.parseTimer) clearTimeout(this.parseTimer);
    if (!text.trim()) {
      this.chips = [];
      this.terms = [];
      this.selection = [];
      if (this.view.startsWith('saved:')) this.view = 'home';
      if (this.view === 'home') this.resetFilters();
      void this.refresh();
      return;
    }
    this.parseTimer = setTimeout(() => {
      void this.reparse();
      void this.loadPage(true);
    }, 120);
  }

  async reparse(): Promise<void> {
    if (!this.text.trim()) {
      this.chips = [];
      this.terms = [];
      return;
    }
    try {
      const parsed = await ipc.parseQuery(this.text);
      this.chips = parsed.chips;
      this.terms = parsed.terms;
    } catch {
      // Keep chips from the last successful parse; results still load.
    }
  }

  async searchNow(): Promise<void> {
    if (!this.text.trim()) return;
    await this.reparse();
    await this.loadPage(true);
    this.pushHistory();
  }

  removeChip(index: number): void {
    const chip = this.chips[index];
    if (!chip) return;
    this.text = removeChipText(this.text, chip);
    this.chips = this.chips.filter((_, i) => i !== index);
    void this.reparse();
    void this.loadPage(true);
  }

  setFilterKind(kind: FileKind | null): void {
    if (this.filterKind === kind) return;
    this.filterKind = kind;
    if (kind !== 'document') this.filterType = 'all';
    void this.loadPage(true);
  }

  setDateFilter(filter: DateFilter): void {
    if (this.filterDate === filter) return;
    this.filterDate = filter;
    void this.loadPage(true);
  }

  setFilterType(type: string): void {
    if (this.filterType === type) return;
    this.filterType = type;
    void this.loadPage(true);
  }

  clearFilters(): void {
    this.resetFilters();
    void this.loadPage(true);
  }

  private resetFilters(): void {
    this.filterKind = null;
    this.filterDate = 'any';
    this.filterType = 'all';
  }

  async setSort(sort: Sort): Promise<void> {
    if (this.sort === sort) return;
    this.sort = sort;
    this.savePrefs();
    if (this.text.trim() || !['home', 'cleanup', 'settings', 'duplicates'].includes(this.view)) {
      await this.loadPage(true);
    } else {
      await this.loadHome();
    }
  }

  setViewMode(mode: 'grid' | 'list'): void {
    this.viewMode = mode;
    this.savePrefs();
  }

  async closeWindow(): Promise<void> {
    try {
      await ipc.closeWindow();
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  setTrackOpens(on: boolean): void {
    this.trackOpens = on;
    this.savePrefs();
  }

  async clearActivity(): Promise<void> {
    try {
      await ipc.clearActivity();
      this.showToast('Open activity cleared.');
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  setZoom(zoom: number): void {
    this.zoom = Math.min(1.5, Math.max(0.8, zoom));
    void ipc.setUiZoom(this.zoom).catch(() => {});
    this.savePrefs();
  }

  addLocation(path: string): boolean {
    const clean = path.trim().replace(/\/+$/, '');
    if (!clean || this.locations.some((l) => l.path === clean)) return false;
    const name = clean.split('/').filter(Boolean).pop() ?? clean;
    this.locations = [...this.locations, { name, path: clean }];
    this.saveLocations();
    return true;
  }

  removeLocation(path: string): void {
    this.locations = this.locations.filter((l) => l.path !== path);
    this.saveLocations();
    if (this.view === `place:${path}`) void this.setView('home');
  }

  openLocation(path: string): void {
    void this.setView(`place:${path}`);
  }

  setTheme(theme: 'system' | 'light' | 'dark'): void {
    this.theme = theme;
    this.applyTheme();
    this.savePrefs();
  }

  private applyTheme(): void {
    const root = document.documentElement;
    if (this.theme === 'system') delete root.dataset.theme;
    else root.dataset.theme = this.theme;
  }

  setShortcut(action: ShortcutAction, combo: string): void {
    if (!combo) return;
    this.shortcuts = { ...this.shortcuts, [action]: combo };
    this.savePrefs();
  }

  resetShortcuts(): void {
    this.shortcuts = { ...DEFAULT_SHORTCUTS };
    this.savePrefs();
  }

  toggleColumn(key: keyof ListColumns): void {
    this.listColumns = { ...this.listColumns, [key]: !this.listColumns[key] };
    this.savePrefs();
  }

  setCell(size: number, persist = true): void {
    this.cell = Math.min(320, Math.max(96, Math.round(size / 8) * 8));
    if (persist) this.savePrefs();
  }

  toggleInspector(force?: boolean): void {
    this.inspectorOpen = force ?? !this.inspectorOpen;
    if (this.inspectorOpen) void this.inspectSelection();
    this.savePrefs();
  }

  async saveTags(path: string, tags: string[]): Promise<void> {
    try {
      await ipc.setTags(path, tags);
      this.inspector = await ipc.inspect(path);
      this.showToast('Tags updated.');
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async inspectSelection(): Promise<void> {
    const path = this.selection[0];
    if (!path) {
      this.inspector = null;
      return;
    }
    try {
      this.inspector = await ipc.inspect(path);
    } catch {
      this.inspector = null;
    }
  }

  select(paths: string[]): void {
    this.selection = paths;
    if (paths.length && !this.inspectorOpen) {
      this.inspectorOpen = true;
      this.savePrefs();
    }
    if (!this.inspectorOpen) return;
    if (this.inspectTimer) clearTimeout(this.inspectTimer);
    this.inspectTimer = setTimeout(() => void this.inspectSelection(), 50);
  }

  async openFile(file: FileMeta): Promise<void> {
    try {
      await ipc.openFile(file.path, this.trackOpens);
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async showInFiles(paths: string[]): Promise<void> {
    if (!paths.length) return;
    try {
      await ipc.showInFiles(paths);
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async copyPath(path: string): Promise<void> {
    try {
      await copyText(path);
      this.showToast('Path copied.');
    } catch {
      this.showToast("Couldn't copy the path.");
    }
  }

  async copyCommand(command: string): Promise<boolean> {
    try {
      await copyText(command);
      return true;
    } catch {
      this.showToast("Couldn't copy the command.");
      return false;
    }
  }

  async trashPaths(paths: string[]): Promise<TrashOutcome | null> {
    if (!paths.length) return null;
    const previous = this.items;
    this.items = this.items.filter((f) => !paths.includes(f.path));
    this.selection = this.selection.filter((p) => !paths.includes(p));
    if (!this.selection.length) this.inspector = null;
    try {
      const outcome = await ipc.trash(paths);
      const n = outcome.moved.length;
      this.showToast(`Moved ${n} ${plural(n, 'file')} to Trash.`, outcome.receipt_id);
      await Promise.all([this.loadCounts(), this.refresh()]);
      if (this.view === 'duplicates') void this.loadDuplicates();
      return outcome;
    } catch (e) {
      this.items = previous;
      this.showToast(errorText(e));
      return null;
    }
  }

  requestTrash(paths: string[], knownBytes?: number): void {
    if (!paths.length || this.view === 'trash') return;
    const chosen = new Set(paths);
    const inItems = this.items.filter((f) => chosen.has(f.path));
    let bytes = knownBytes ?? inItems.reduce((n, f) => n + f.size, 0);
    if (
      this.inspector &&
      chosen.has(this.inspector.file.path) &&
      !inItems.some((f) => f.path === this.inspector?.file.path)
    ) {
      bytes += this.inspector.file.size;
    }
    this.trashPrompt = { paths, bytes };
  }

  requestRename(path: string, name: string): void {
    this.renamePrompt = { path, name };
  }

  cancelRename(): void {
    this.renamePrompt = null;
  }

  async renameWithName(name: string): Promise<void> {
    const prompt = this.renamePrompt;
    if (!prompt || !name.trim() || name.trim() === prompt.name) return;
    try {
      const next = await ipc.renamePath(prompt.path, name.trim());
      this.renamePrompt = null;
      if (this.view === `place:${prompt.path}`) this.view = `place:${next}`;
      this.select([next]);
      this.showToast('Renamed.');
      await this.refresh();
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  requestTransfer(paths: string[], mode: 'move' | 'copy'): void {
    if (!paths.length) return;
    this.transferPrompt = { paths, mode };
  }

  cancelTransfer(): void {
    this.transferPrompt = null;
  }

  async transfer(paths: string[], dest: string, mode: 'move' | 'copy'): Promise<boolean> {
    if (!paths.length || !dest.trim()) return false;
    try {
      const changed = await ipc.transfer(paths, dest.trim(), mode);
      this.transferPrompt = null;
      this.showToast(
        `${mode === 'move' ? 'Moved' : 'Copied'} ${changed.length || paths.length} ${plural(paths.length, 'file')}.`,
      );
      await Promise.all([this.loadCounts(), this.refresh()]);
      return true;
    } catch (e) {
      this.showToast(errorText(e));
      return false;
    }
  }

  requestPurge(paths: string[]): void {
    if (paths.length) this.purgePrompt = paths;
  }

  cancelPurge(): void {
    this.purgePrompt = null;
  }

  async purgeTrash(paths: string[]): Promise<void> {
    if (!paths.length) return;
    try {
      const n = await ipc.purgeTrash(paths);
      this.purgePrompt = null;
      this.showToast(`Permanently deleted ${n} ${plural(n, 'file')}.`);
      await Promise.all([this.loadCounts(), this.refresh()]);
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async confirmTrash(): Promise<void> {
    const prompt = this.trashPrompt;
    this.trashPrompt = null;
    if (prompt) await this.trashPaths(prompt.paths);
  }

  cancelTrash(): void {
    this.trashPrompt = null;
  }

  async trashSelection(): Promise<void> {
    if (this.view === 'trash') return;
    this.requestTrash(this.selection);
  }

  async undo(receiptId: number): Promise<void> {
    try {
      const n = await ipc.undoTrash(receiptId);
      this.toast = null;
      this.showToast(`Restored ${n} ${plural(n, 'file')}.`);
      await Promise.all([this.loadCounts(), this.refresh()]);
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async saveCurrentSearch(name: string): Promise<void> {
    if (!this.text.trim()) return;
    try {
      await ipc.saveSearch(name, this.text.trim());
      await this.loadSaved();
      this.showToast(`Saved “${name}”.`);
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async deleteSaved(id: number): Promise<void> {
    try {
      await ipc.deleteSavedSearch(id);
      await this.loadSaved();
      if (this.view === `saved:${id}`) await this.setView('home');
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  async reindex(): Promise<void> {
    try {
      await ipc.indexRoot(null);
      this.showToast('Reindexing in the background.');
    } catch (e) {
      this.showToast(errorText(e));
    }
  }

  showToast(message: string, undo?: number): void {
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toast = { id: Date.now(), message, undo };
    const ms = undo ? 30_000 : 8_000;
    this.toastTimer = setTimeout(() => {
      this.toast = null;
    }, ms);
  }

  dismissToast(): void {
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toast = null;
  }

  back(): void {
    if (this.historyIndex <= 0) return;
    this.historyIndex -= 1;
    this.applyHistory();
  }

  forward(): void {
    if (this.historyIndex >= this.history.length - 1) return;
    this.historyIndex += 1;
    this.applyHistory();
  }

  get canBack(): boolean {
    return this.historyIndex > 0;
  }

  get canForward(): boolean {
    return this.historyIndex < this.history.length - 1;
  }

  private applyHistory(): void {
    const entry = this.history[this.historyIndex];
    if (!entry) return;
    this.view = entry.view;
    this.sort = entry.view === 'recent' || entry.view === 'downloads' ? 'added' : entry.view.startsWith('place:') ? 'name' : 'modified';
    this.text = entry.text;
    this.selection = [];
    this.inspector = null;
    this.folders = [];
    this.folderInfo = null;
    this.resetFilters();
    void this.reparse();
    void this.refresh();
  }

  private pushHistory(): void {
    const last = this.history[this.history.length - 1];
    if (last && last.view === this.view && last.text === this.text) return;
    this.history = [...this.history.slice(0, this.historyIndex + 1), { view: this.view, text: this.text }];
    this.historyIndex = this.history.length - 1;
  }

  private loadLocations(): void {
    try {
      const raw = localStorage.getItem(LOCATIONS_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as Location[];
      if (Array.isArray(parsed)) {
        this.locations = parsed.filter(
          (l) => typeof l?.name === 'string' && typeof l?.path === 'string',
        );
      }
    } catch {
      this.locations = [];
    }
  }

  private saveLocations(): void {
    try {
      localStorage.setItem(LOCATIONS_KEY, JSON.stringify(this.locations));
    } catch {
      // Storage is optional.
    }
  }

  private loadHomeCache(): void {
    try {
      const raw = localStorage.getItem(HOME_CACHE_KEY);
      if (!raw) return;
      const cached = JSON.parse(raw) as HomeData;
      if (Array.isArray(cached.recent) && Array.isArray(cached.shelves)) this.home = cached;
    } catch {
      // A corrupt cache only costs one repaint.
    }
  }

  private saveHomeCache(): void {
    try {
      if (this.home) localStorage.setItem(HOME_CACHE_KEY, JSON.stringify(this.home));
    } catch {
      // Storage can be full or unavailable; the live response is still applied.
    }
  }

  private loadPrefs(): void {
    try {
      const raw = localStorage.getItem(PREFS_KEY);
      if (!raw) return;
      const p = JSON.parse(raw) as Partial<Prefs>;
      if (p.viewMode === 'grid' || p.viewMode === 'list') this.viewMode = p.viewMode;
      if (typeof p.cell === 'number') this.cell = Math.min(320, Math.max(96, p.cell));
      if (p.sort) this.sort = p.sort;
      if (typeof p.inspectorOpen === 'boolean') this.inspectorOpen = p.inspectorOpen;
      if (p.columns) this.listColumns = { ...this.listColumns, ...p.columns };
      if (p.shortcuts) {
        this.shortcuts = { ...DEFAULT_SHORTCUTS, ...(p.shortcuts as Record<ShortcutAction, string>) };
      }
      if (p.theme === 'light' || p.theme === 'dark' || p.theme === 'system') {
        this.theme = p.theme;
      }
      if (typeof p.zoom === 'number') this.zoom = Math.min(1.5, Math.max(0.8, p.zoom));
      if (typeof p.trackOpens === 'boolean') this.trackOpens = p.trackOpens;
    } catch {
      // Corrupt preferences fall back to defaults.
    }
  }

  private savePrefs(): void {
    try {
      const p: Prefs = {
        viewMode: this.viewMode,
        cell: this.cell,
        sort: this.sort,
        inspectorOpen: this.inspectorOpen,
        columns: this.listColumns,
        shortcuts: this.shortcuts,
        theme: this.theme,
        zoom: this.zoom,
        trackOpens: this.trackOpens,
      };
      localStorage.setItem(PREFS_KEY, JSON.stringify(p));
    } catch {
      // Storage can be unavailable in private contexts; defaults still work.
    }
  }
}

async function copyText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    // WebKitGTK can deny Clipboard API without a trusted gesture; fall back.
  }
  const area = document.createElement('textarea');
  area.value = text;
  area.style.position = 'fixed';
  area.style.opacity = '0';
  document.body.appendChild(area);
  area.select();
  const ok = document.execCommand('copy');
  area.remove();
  if (!ok) throw new Error('copy failed');
}

function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return 'Something went wrong.';
}

export const shelf = new ShelfStore();
