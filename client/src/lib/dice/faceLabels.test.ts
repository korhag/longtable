import { describe, expect, it } from 'vitest';

import type { DieMesh } from '../net/bindings/DieMesh.ts';
import { d4CornerLabels } from './faceLabels';

/** The same four vertices the server scales onto a d4, and one face per left-out corner. */
function serverTetrahedron(): DieMesh {
  const raw: [number, number, number][] = [
    [1, 1, 1],
    [1, -1, -1],
    [-1, 1, -1],
    [-1, -1, 1],
  ];
  const vertices = raw.map((vertex) => {
    const length = Math.hypot(vertex[0], vertex[1], vertex[2]);
    const scale = 0.46 / length;
    return [vertex[0] * scale, vertex[1] * scale, vertex[2] * scale] as [number, number, number];
  });
  const faces = [0, 1, 2, 3].map((skip) => ({
    indices: [0, 1, 2, 3].filter((index) => index !== skip),
    value: skip + 1,
    normal: [0, 1, 0] as [number, number, number],
  }));
  return { sides: 4, vertices, faces };
}

describe('d4 corner labels', () => {
  const mesh = serverTetrahedron();
  const labels = d4CornerLabels(mesh);

  it('prints the value of the face that does not touch that corner', () => {
    for (const label of labels) {
      const opposite = mesh.faces.find((face) => !face.indices.includes(label.vertex));
      expect(opposite?.value).toBe(label.value);
    }
  });

  it('gives every face three different corner numbers', () => {
    for (let face = 0; face < mesh.faces.length; face += 1) {
      const values = labels.filter((label) => label.face === face).map((label) => label.value);
      expect(values).toHaveLength(3);
      expect(new Set(values).size).toBe(3);
    }
  });
});
