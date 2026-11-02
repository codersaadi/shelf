import { ipc, type NativeIcon } from './ipc';

const cache = new Map<string, Promise<NativeIcon | null>>();
const MAX_CACHE = 200;
const MAX_ACTIVE = 2;
let active = 0;
const pending: Array<() => void> = [];

function schedule<T>(run: () => Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const start = () => {
      active += 1;
      run()
        .then(resolve, reject)
        .finally(() => {
          active -= 1;
          pending.shift()?.();
        });
    };
    if (active < MAX_ACTIVE) start();
    else pending.push(start);
  });
}

/** One lookup per app file, with a small concurrency cap so icon scans do not
 *  burst across the desktop UI. Invalidated when the file changes. */
export function loadNativeIcon(path: string, mtime: number): Promise<NativeIcon | null> {
  const key = `${path}\u0000${mtime}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const request = schedule(() => ipc.nativeIcon(path)).catch(() => null);
  cache.set(key, request);
  if (cache.size > MAX_CACHE) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
  return request;
}
