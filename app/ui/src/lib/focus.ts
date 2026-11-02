/** Keep Tab inside a dialog and restore focus to the element that opened it. */
export function trapFocus(node: HTMLElement) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;

  function items(): HTMLElement[] {
    return [...node.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])',
    )].filter((el) => el.getClientRects().length > 0);
  }

  const first = items()[0];
  if (first) first.focus();
  else node.focus();

  function onKey(event: KeyboardEvent): void {
    if (event.key !== 'Tab') return;
    const list = items();
    if (!list.length) return;
    const head = list[0];
    const tail = list[list.length - 1];
    if (!list.includes(document.activeElement as HTMLElement)) {
      event.preventDefault();
      (event.shiftKey ? tail : head).focus();
    } else if (event.shiftKey && document.activeElement === head) {
      event.preventDefault();
      tail.focus();
    } else if (!event.shiftKey && document.activeElement === tail) {
      event.preventDefault();
      head.focus();
    }
  }

  node.addEventListener('keydown', onKey);
  return {
    destroy() {
      node.removeEventListener('keydown', onKey);
      previous?.focus();
    },
  };
}
