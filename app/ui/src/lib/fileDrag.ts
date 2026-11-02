/** Build a standards-compliant file URI for a local absolute path. */
export function fileUri(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const absolute = normalized.startsWith('/') ? normalized : `/${normalized}`;
  return `file://${absolute.split('/').map(encodeURIComponent).join('/')}`;
}

/**
 * Put both freedesktop URI-list data and Chromium's DownloadURL data on the
 * drag. WebKitGTK cannot fabricate a native file-promise drag, but external
 * file managers and browsers accept these URI payloads.
 */
export function startFileDrag(event: DragEvent, paths: string[]): void {
  const transfer = event.dataTransfer;
  if (!transfer || paths.length === 0) return;

  const uris = paths.map(fileUri);
  const set = (type: string, value: string): void => {
    try {
      transfer.setData(type, value);
    } catch {
      // Some WebKit builds reject custom MIME types; keep the other entries.
    }
  };

  set('text/uri-list', uris.join('\r\n'));
  set('text/plain', paths.join('\n'));
  set('x-special/gnome-copied-files', `copy\n${uris.join('\n')}`);
  set(
    'DownloadURL',
    paths
      .map((path, i) => {
        const name = path.split('/').pop() ?? 'file';
        return `application/octet-stream:${encodeURIComponent(name)}:${uris[i]}`;
      })
      .join('\n'),
  );
  set('application/x-shelf-paths', JSON.stringify(paths));

  transfer.effectAllowed = 'copy';
  const source = (event.currentTarget as HTMLElement | null)?.closest(
    '.tile, .row, .rail-card, .shelf-card',
  );
  if (source instanceof HTMLElement) transfer.setDragImage(source, 24, 24);
}
