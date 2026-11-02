/** Coarse local-date buckets used for grouped shelves. */
export function dateGroup(seconds: number, now = Date.now()): string {
  const then = new Date(seconds * 1000);
  then.setHours(0, 0, 0, 0);
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  const days = Math.round((today.getTime() - then.getTime()) / 86_400_000);
  if (days <= 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days <= 7) return 'This week';
  return 'Earlier';
}
