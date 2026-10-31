import * as THREE from 'three';

const baseColors = new WeakMap<THREE.Material, THREE.Color>();

/** Upright scenery extends above the ground fog. Dim its retained materials too. */
export function shadeScenery(group: THREE.Object3D, brightness: number) {
  group.userData.fogBrightness = brightness;
  group.traverse(node => {
    if (!(node instanceof THREE.Mesh) && !(node instanceof THREE.Sprite)) return;
    const materials = Array.isArray(node.material) ? node.material : [node.material];
    for (const material of materials) {
      if (!('color' in material) || !(material.color instanceof THREE.Color)) continue;
      const base: THREE.Color = baseColors.get(material) ?? material.color.clone();
      baseColors.set(material, base);
      material.color.copy(base).multiplyScalar(brightness);
    }
  });
}

/** Texture loads may replace the fallback tint after the fog was applied. */
export function setSceneryColor(material: THREE.MeshBasicMaterial, color: THREE.ColorRepresentation) {
  material.color.set(color);
  baseColors.set(material, material.color.clone());
}
