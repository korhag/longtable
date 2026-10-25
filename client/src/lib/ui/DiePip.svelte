<script lang="ts">
  let { sides, body, ink, onclick }: {
    sides: number;
    body: string;
    ink: string;
    onclick: () => void;
  } = $props();

  const points = $derived(shape(sides));
  const labelY = $derived(sides === 4 ? 68 : 56);

  function shape(count: number): string {
    if (count === 4) return '50,8 92,90 8,90';
    if (count === 6) return '16,16 84,16 84,84 16,84';
    if (count === 8) return '50,6 94,50 50,94 6,50';
    if (count === 10) return '50,4 90,34 74,94 26,94 10,34';
    if (count === 12) return polygon(5, 50, 52, 44);
    return polygon(6, 50, 50, 44);
  }

  function polygon(count: number, cx: number, cy: number, radius: number) {
    return Array.from({ length: count }, (_, index) => {
      const angle = (-Math.PI / 2) + (index * 2 * Math.PI) / count;
      return `${(cx + Math.cos(angle) * radius).toFixed(1)},${(cy + Math.sin(angle) * radius).toFixed(1)}`;
    }).join(' ');
  }
</script>

<button class="die-shape" type="button" aria-label={`Add a d${sides}`} onclick={onclick}>
  <svg viewBox="0 0 100 100" aria-hidden="true">
    <polygon {points} fill={body} stroke="rgba(255,236,210,0.45)" stroke-width="3" stroke-linejoin="round" />
    <text x="50" y={labelY} text-anchor="middle" fill={ink} font-weight="700" font-size="22" font-family="Cinzel, serif">d{sides}</text>
  </svg>
</button>

<style>
  .die-shape {
    width: 2.7rem;
    height: 2.7rem;
    background: none;
    border: 0;
    padding: 0;
  }
  .die-shape svg {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
