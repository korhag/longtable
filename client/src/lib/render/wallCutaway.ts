import * as THREE from 'three';

/** Clip the upper wall in world space; squeezing the sprite also distorts its ground edge. */
export function setWallCutaway(group: THREE.Group, lowered: boolean) {
  if (group.userData.cutaway === lowered) return;
  group.userData.cutaway = lowered;
  const planes = lowered ? [new THREE.Plane(new THREE.Vector3(0, -1, 0), group.position.y + 0.45)] : null;
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    for (const material of Array.isArray(object.material) ? object.material : [object.material]) {
      material.clippingPlanes = planes;
      material.needsUpdate = true;
    }
  });
}
