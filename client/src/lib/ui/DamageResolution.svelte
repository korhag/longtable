<script lang="ts">
  import type { StrikeOffer } from '../net/bindings/StrikeOffer';
  import type { Blow } from '../net/bindings/Blow';
  import { damageApplication, damageSteps, severityName } from '../dice/damageFlow';

  let { offer, pending, receipt, busy, error, readOnly, critical = false, onanswer }: {
    offer: StrikeOffer; pending: boolean; receipt: Blow | null; busy: boolean;
    error: string | null; readOnly: boolean; critical?: boolean;
    onanswer: (useArmor: boolean) => void;
  } = $props();
  let previewArmor = $state(false);
  const canArmor = $derived(offer.armorAvailable > 0 && !offer.direct);
  const effective = $derived(receipt && !pending ? Math.max(0, receipt.marks - (receipt.armorUsed ? 1 : 0)) : previewArmor && canArmor ? offer.withArmor : offer.marks);
  const steps = $derived(damageSteps(offer.majorThreshold, offer.severeThreshold));
</script>

<section class="damage-decision" aria-label={`Damage decision for ${offer.targetName}`}>
  <div class="decision-title">
    <span class="display">{offer.targetName}</span>
    <strong>{offer.damageTotal == null ? 'Damage' : `${offer.damageTotal} damage`}</strong>
  </div>
  <p class="decision-context">{offer.hpMarked}/{offer.hpMax} HP marked before this blow · {offer.armorAvailable} armor slot{offer.armorAvailable === 1 ? '' : 's'} available</p>
  <div class="threshold-track" aria-label="Damage severity from left to right">
    {#each steps as step}
      <div class:current={step.marks === effective} class:original={step.marks === offer.marks && effective !== offer.marks} aria-current={step.marks === effective ? 'step' : undefined}>
        <span>{step.name}</span><strong>{step.range}</strong><small>{step.marks} HP</small>
      </div>
    {/each}
  </div>
  {#if offer.direct}<p class="effect-note">Direct damage bypasses armor.</p>{/if}
  {#if critical}<p class="effect-note">The critical damage bonus is already included in this total.</p>{/if}
  {#if pending}
    <p class="decision-preview" aria-live="polite">{previewArmor && canArmor ? `${offer.step} → ${severityName(offer.withArmor)} · ${offer.withArmor} HP with armor` : `${offer.step} · ${offer.marks} HP without armor`}</p>
    <div class="decision-buttons">
      <button class="btn" type="button" disabled={!canArmor || busy || readOnly}
        onmouseenter={() => previewArmor = true} onmouseleave={() => previewArmor = false}
        onfocus={() => previewArmor = true} onblur={() => previewArmor = false}
        onclick={() => { previewArmor = true; onanswer(true); }}>
        Use armor · 1 slot → {severityName(offer.withArmor)}
      </button>
      <button class="btn quiet" type="button" disabled={busy || readOnly} onclick={() => { previewArmor = false; onanswer(false); }}>Take damage · {offer.marks} HP</button>
    </div>
    {#if !canArmor && !offer.direct}<p class="effect-note">No armor slots remain. Take damage to resolve this blow.</p>{/if}
    {#if readOnly}<p class="effect-note">Return to GM view to resolve damage.</p>{/if}
    {#if busy}<p role="status">Resolving damage…</p>{/if}
  {:else if receipt}
    <p class="decision-complete" role="status">{damageApplication(receipt)}</p>
  {:else}<p class="decision-complete" role="status">This damage decision is no longer pending.</p>{/if}
  {#if error}<p class="decision-error" role="alert">{error}</p>{/if}
</section>

<style>
  .damage-decision { padding: 14px 16px; background: linear-gradient(125deg,#38291a,#211a13); border-top: 1px solid #b58c4d77; font-size: 13px; }
  .decision-title { display:flex; align-items:center; justify-content:space-between; gap:12px; color:#f0d8af; }
  .decision-title strong { white-space:nowrap; }
  .decision-title .display { min-width:0; overflow-wrap:anywhere; }
  .decision-context,.effect-note { margin-top:7px; font-size:11px; color:#c1b197; line-height:1.5; }
  .threshold-track { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:5px; margin-top:12px; }
  .threshold-track>div { display:flex; flex-direction:column; align-items:center; gap:4px; padding:9px 3px; border:1px solid #b38b4a44; border-radius:4px; background:#17120d; color:#bca98a; }
  .threshold-track span { font:11px var(--font-display); }
  .threshold-track strong { font-size:13px; }
  .threshold-track small { font-size:10px; }
  .threshold-track>div.current { border-color:#e0bd77; background:#79582c; color:#fff0cf; box-shadow:inset 0 0 0 2px #d5b06d33; }
  .threshold-track>div.original { border-style:dashed; border-color:#ba925f; }
  .decision-preview { margin-top:10px; min-height:20px; color:#efd3a1; }
  .decision-buttons { display:flex; flex-wrap:wrap; gap:8px; margin-top:8px; }
  .decision-buttons button { flex:1 1 160px; font-size:12px; }
  .decision-complete { margin-top:12px; color:#d3dcae; }
  .decision-error { margin-top:9px; color:#f2a798; }
</style>
