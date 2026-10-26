// ------------------------------------------------------------
// Module: portal
// Purpose: Lift a menu out of a scrolling panel so it can
//          open above every other layer.
// ------------------------------------------------------------

export interface Anchor {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export function portal(node: HTMLElement) {
  document.body.append(node);
  return {
    destroy() {
      node.remove();
    },
  };
}

export function menuBox(anchor: Anchor, width: number): string {
  const left = Math.max(8, Math.min(anchor.left, window.innerWidth - width - 8));
  const bottom = Math.max(8, window.innerHeight - anchor.top + 8);
  const maxHeight = Math.max(120, anchor.top - 16);
  return `position:fixed;left:${left}px;bottom:${bottom}px;width:${width}px;max-height:${maxHeight}px;z-index:calc(var(--z-menu) + 1)`;
}
