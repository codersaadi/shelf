/**
 * Mouse/touch drag, momentum and keyboard scrolling for horizontal rails.
 * Native touchpads already send deltaX; this adds grab-to-scroll, shift+wheel
 * and arrow keys without turning the rail into a tab trap.
 */
export function dragScroll(node: HTMLElement) {
  let down = false;
  let moved = false;
  let suppressClick = false;
  let startX = 0;
  let startLeft = 0;
  let pointerId = -1;
  let lastX = 0;
  let lastAt = 0;
  let velocity = 0;
  let frame = 0;

  node.tabIndex = node.tabIndex >= 0 ? node.tabIndex : 0;
  node.setAttribute('role', node.getAttribute('role') ?? 'region');

  const reduceMotion = () =>
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

  function stopMomentum(): void {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }

  function momentum(): void {
    if (reduceMotion() || Math.abs(velocity) < 0.03) return;
    const max = node.scrollWidth - node.clientWidth;
    const step = () => {
      velocity *= 0.94;
      if (Math.abs(velocity) < 0.01) {
        frame = 0;
        return;
      }
      const next = Math.min(max, Math.max(0, node.scrollLeft - velocity * 16));
      node.scrollLeft = next;
      if (next <= 0 || next >= max) {
        frame = 0;
        return;
      }
      frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
  }

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0 || (event.pointerType === 'touch' && event.isPrimary === false)) return;
    stopMomentum();
    down = true;
    moved = false;
    startX = event.clientX;
    startLeft = node.scrollLeft;
    lastX = event.clientX;
    lastAt = performance.now();
    velocity = 0;
    pointerId = event.pointerId;
  }

  function onPointerMove(event: PointerEvent): void {
    if (!down || event.pointerId !== pointerId) return;
    const dx = event.clientX - startX;
    const now = performance.now();
    const dt = now - lastAt;
    if (dt > 0) velocity = (event.clientX - lastX) / dt;
    lastX = event.clientX;
    lastAt = now;
    if (!moved && Math.abs(dx) < 6) return;
    if (!moved) {
      moved = true;
      suppressClick = true;
      node.setPointerCapture?.(pointerId);
      node.classList.add('dragging');
    }
    event.preventDefault();
    node.scrollLeft = startLeft - dx;
  }

  function finish(event?: PointerEvent): void {
    if (!down) return;
    if (event && event.pointerId !== pointerId) return;
    down = false;
    node.classList.remove('dragging');
    if (node.hasPointerCapture?.(pointerId)) node.releasePointerCapture?.(pointerId);
    pointerId = -1;
    if (moved) {
      if (performance.now() - lastAt > 90) velocity = 0;
      setTimeout(() => (suppressClick = false), 0);
      momentum();
    }
  }

  function onClick(event: MouseEvent): void {
    if (!suppressClick) return;
    event.preventDefault();
    event.stopPropagation();
  }

  function onWheel(event: WheelEvent): void {
    stopMomentum();
    if (!event.shiftKey) return;
    const max = node.scrollWidth - node.clientWidth;
    if (max <= 0) return;
    event.preventDefault();
    node.scrollLeft = Math.min(max, Math.max(0, node.scrollLeft + event.deltaY));
  }

  function onKey(event: KeyboardEvent): void {
    const max = node.scrollWidth - node.clientWidth;
    if (max <= 0) return;
    const step = Math.round(node.clientWidth * 0.8);
    const smooth = reduceMotion() ? 'auto' : 'smooth';
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      event.stopPropagation();
      node.scrollBy({ left: step, behavior: smooth });
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      event.stopPropagation();
      node.scrollBy({ left: -step, behavior: smooth });
    } else if (event.key === 'Home') {
      event.preventDefault();
      event.stopPropagation();
      node.scrollTo({ left: 0, behavior: smooth });
    } else if (event.key === 'End') {
      event.preventDefault();
      event.stopPropagation();
      node.scrollTo({ left: max, behavior: smooth });
    }
  }

  node.addEventListener('pointerdown', onPointerDown);
  node.addEventListener('pointermove', onPointerMove);
  node.addEventListener('pointerup', finish);
  node.addEventListener('pointercancel', finish);
  node.addEventListener('wheel', onWheel, { passive: false });
  node.addEventListener('keydown', onKey);
  node.addEventListener('click', onClick, true);

  return {
    destroy() {
      stopMomentum();
      node.removeEventListener('pointerdown', onPointerDown);
      node.removeEventListener('pointermove', onPointerMove);
      node.removeEventListener('pointerup', finish);
      node.removeEventListener('pointercancel', finish);
      node.removeEventListener('wheel', onWheel);
      node.removeEventListener('keydown', onKey);
      node.removeEventListener('click', onClick, true);
    },
  };
}
