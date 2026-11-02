import { ipc, type FileMeta, type TextSnippet } from './ipc';
import { extensionOf } from './extensions';

const cache = new Map<string, Promise<TextSnippet | null>>();
const MAX_CACHE = 400;

export function isTextPreviewable(file: FileMeta): boolean {
  if (file.name.startsWith('.')) return false;
  return ['txt', 'md', 'markdown', 'mdx'].includes(extensionOf(file.name));
}

/** Deduplicates card reads and invalidates when the file changes. */
export function loadTextSnippet(file: FileMeta): Promise<TextSnippet | null> {
  const key = `${file.path}\u0000${file.mtime}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const request = ipc.textSnippet(file.path).catch(() => null);
  cache.set(key, request);
  if (cache.size > MAX_CACHE) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
  return request;
}
