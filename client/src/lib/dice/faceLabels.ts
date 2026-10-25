// ------------------------------------------------------------
// Module: faceLabels
// Purpose: Decide which number sits at each corner of a d4.
//          A d4 is read from the face it lands on, so that
//          number is printed at the top point of the three
//          faces you can still see.
// ------------------------------------------------------------

import type { DieMesh } from '../net/bindings/DieMesh.ts';

export interface CornerLabel {
  /** Face the number is painted on. */
  face: number;
  /** Vertex of that face the number sits beside. */
  vertex: number;
  value: number;
}

/**
 * For every corner of every face, the value of the one face that does not touch that corner.
 * On a tetrahedron that face is the one opposite the corner.
 */
export function d4CornerLabels(mesh: DieMesh): CornerLabel[] {
  const labels: CornerLabel[] = [];
  mesh.faces.forEach((face, faceIndex) => {
    for (const vertex of face.indices) {
      const opposite = mesh.faces.findIndex(
        (candidate, index) => index !== faceIndex && !candidate.indices.includes(vertex),
      );
      if (opposite < 0) continue;
      labels.push({ face: faceIndex, vertex, value: mesh.faces[opposite]?.value ?? 0 });
    }
  });
  return labels;
}
