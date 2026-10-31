// ------------------------------------------------------------
// Module: fogStamp
// Purpose: Decide whether the fog sheet changed without copying
//          the lit shape into a string.
// ------------------------------------------------------------

import type { FogView } from '../net/bindings/FogView.ts';

export type FogPolygons = FogView['visiblePolygons'];

export type FogStamp = {
  explored: string;
  visible: string;
  mode: string;
  fogWidth: number;
  fogHeight: number;
  cellsPerSquare: number;
  mapWidth: number;
  mapHeight: number;
  levels: string;
  polygons: FogPolygons;
};

export function samePolygons(left: FogPolygons | undefined, right: FogPolygons | undefined) {
  const a = left ?? [];
  const b = right ?? [];
  if (a === b) return true;
  if (a.length !== b.length) return false;
  return a.every((polygon, index) => {
    const other = b[index];
    if (!other || polygon.length !== other.length) return false;
    return polygon.every((point, pointIndex) => point[0] === other[pointIndex]?.[0] && point[1] === other[pointIndex]?.[1]);
  });
}

export function fogStamp(fog: FogView, mapWidth: number, levels: string, mapHeight = mapWidth): FogStamp {
  return {
    explored: fog.explored,
    visible: fog.visible ?? '',
    mode: fog.mode,
    fogWidth: fog.width,
    fogHeight: fog.height,
    cellsPerSquare: fog.cellsPerSquare,
    mapWidth,
    mapHeight,
    levels,
    polygons: fog.visiblePolygons,
  };
}

export function sameFogPicture(left: FogView | null | undefined, right: FogView | null | undefined) {
  if (left === right) return true;
  if (!left || !right) return false;
  return left.mode === right.mode
    && left.width === right.width
    && left.height === right.height
    && left.cellsPerSquare === right.cellsPerSquare
    && left.explored === right.explored
    && (left.visible ?? '') === (right.visible ?? '')
    && samePolygons(left.visiblePolygons, right.visiblePolygons);
}

export function sameFogStamp(stamp: FogStamp, fog: FogView, mapWidth: number, levels: string, mapHeight = mapWidth) {
  return stamp.explored === fog.explored
    && stamp.visible === (fog.visible ?? '')
    && stamp.mode === fog.mode
    && stamp.fogWidth === fog.width
    && stamp.fogHeight === fog.height
    && stamp.cellsPerSquare === fog.cellsPerSquare
    && stamp.mapWidth === mapWidth
    && stamp.mapHeight === mapHeight
    && stamp.levels === levels
    && samePolygons(stamp.polygons, fog.visiblePolygons);
}
