export interface PanelBox { x: number; y: number; w: number; h: number }

/** Keep both the title and all content reachable after a resize or restored layout. */
export function panelBounds(box: PanelBox, viewportWidth: number, viewportHeight: number): PanelBox {
  const margin = 8;
  const w = Math.min(Math.max(240, box.w), Math.max(0, viewportWidth - margin * 2));
  const h = Math.min(Math.max(160, box.h), Math.max(0, viewportHeight - margin * 2));
  return {
    w, h,
    x: Math.max(margin, Math.min(box.x, viewportWidth - w - margin)),
    y: Math.max(margin, Math.min(box.y, viewportHeight - h - margin)),
  };
}
