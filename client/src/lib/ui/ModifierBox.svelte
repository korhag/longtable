<script lang="ts">
  import { clampModifier } from '../dice/rollText';

  let { value, sign, onchange }: {
    value: number;
    sign: '+' | '−';
    onchange: (next: number) => void;
  } = $props();
</script>

<label class="modifier">
  <span>{sign}</span>
  <input
    type="number"
    min="0"
    max="1000"
    {value}
    aria-label={`${sign} modifier`}
    oninput={(event) => onchange(clampModifier(event.currentTarget.value))}
  />
  <button type="button" aria-label={`Add 1 to the ${sign} modifier`} onclick={() => onchange(Math.min(1000, value + 1))}>+1</button>
</label>

<style>
  .modifier {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    height: 2rem;
    padding: 0 0.35rem;
    border-radius: 0.6rem;
    border: 1px solid rgba(212, 166, 86, 0.35);
    background: rgba(16, 14, 12, 0.45);
    font-size: 0.875rem;
  }
  input {
    width: 2.6rem;
    background: transparent;
    text-align: center;
  }
  button {
    border-radius: 0.4rem;
    padding: 0.1rem 0.35rem;
    background: rgba(255, 255, 255, 0.08);
    font-size: 0.875rem;
  }
</style>
