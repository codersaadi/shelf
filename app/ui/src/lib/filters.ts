import type { FileKind } from './ipc';

export type DateFilter = 'any' | 'today' | 'week' | 'month' | 'year';

export const DATE_OPTIONS: { value: DateFilter; label: string }[] = [
  { value: 'any', label: 'Any time' },
  { value: 'today', label: 'Today' },
  { value: 'week', label: 'Last 7 days' },
  { value: 'month', label: 'Last 30 days' },
  { value: 'year', label: 'This year' },
];

export const KIND_OPTIONS: { value: FileKind | null; label: string }[] = [
  { value: null, label: 'All kinds' },
  { value: 'image', label: 'Images' },
  { value: 'video', label: 'Videos' },
  { value: 'music', label: 'Music' },
  { value: 'document', label: 'Documents' },
  { value: 'archive', label: 'Archives' },
  { value: 'app', label: 'Apps' },
  { value: 'other', label: 'Other files' },
];

export interface DocumentType {
  id: string;
  label: string;
  exts: string[];
}

/**
 * Word processing, spreadsheets, slides, Markdown, plain text and e-books are
 * one document family for browsing. These groups let the Type control narrow
 * that family without inventing a second kind taxonomy.
 */
export const DOCUMENT_TYPES: DocumentType[] = [
  { id: 'all', label: 'All types', exts: [] },
  { id: 'pdf', label: 'PDF', exts: ['pdf'] },
  { id: 'word', label: 'Word documents', exts: ['doc', 'docx', 'docm', 'odt', 'rtf', 'pages'] },
  {
    id: 'spreadsheet',
    label: 'Spreadsheets',
    exts: ['xls', 'xlsx', 'xlsm', 'ods', 'csv', 'tsv'],
  },
  { id: 'slides', label: 'Slides', exts: ['ppt', 'pptx', 'pptm', 'odp'] },
  { id: 'markdown', label: 'Markdown', exts: ['md', 'markdown', 'mdx'] },
  { id: 'text', label: 'Plain text', exts: ['txt', 'tex'] },
  { id: 'ebook', label: 'E-books', exts: ['epub', 'djvu'] },
];

export function documentType(id: string): DocumentType {
  return DOCUMENT_TYPES.find((t) => t.id === id) ?? DOCUMENT_TYPES[0];
}

export function dateFrom(filter: DateFilter, now = new Date()): number | undefined {
  if (filter === 'any') return undefined;
  const start = new Date(now);
  if (filter === 'today') {
    start.setHours(0, 0, 0, 0);
  } else if (filter === 'week') {
    start.setTime(now.getTime() - 7 * 86_400_000);
  } else if (filter === 'month') {
    start.setTime(now.getTime() - 30 * 86_400_000);
  } else {
    start.setMonth(0, 1);
    start.setHours(0, 0, 0, 0);
  }
  return Math.floor(start.getTime() / 1000);
}
