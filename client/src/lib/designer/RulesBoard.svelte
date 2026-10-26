<script lang="ts">
  import type { CharacterKind } from '../net/bindings/CharacterKind.ts';
  import { designer, type DesignerView } from './designer.svelte';
  import RulesOverview from './RulesOverview.svelte';
  import SheetBuilder from './SheetBuilder.svelte';

  const tabs: { id: DesignerView; label: string }[] = [
    { id: 'overview', label: 'Overview' },
    { id: 'pc', label: 'PC sheet' },
    { id: 'npc', label: 'NPC sheet' },
    { id: 'adversary', label: 'Adversary card' },
  ];

  function show(id: DesignerView) {
    designer.view = id;
  }

  const kind = $derived(designer.view === 'overview' ? null : designer.view as CharacterKind);
</script>

<section class="rules-board">
  {#if designer.draft}
    <div class="flex flex-wrap gap-1 border-b border-white/10 p-2">
      {#each tabs as tab (tab.id)}
        <button class="btn quiet px-2" class:active={designer.view === tab.id} type="button" onclick={() => show(tab.id)}>{tab.label}</button>
      {/each}
    </div>
    <div class="min-h-0 flex-1 overflow-auto p-3">
      {#if designer.view === 'overview'}
        <RulesOverview />
      {:else if kind}
        <SheetBuilder />
      {/if}
    </div>
  {:else}
    <div class="grid flex-1 place-items-center p-6 text-center text-[12px]">
      {#if designer.notice?.tone === 'error'}
        <p class="text-[#e7a090]">{designer.notice.text}</p>
      {:else}
        <p class="text-[var(--color-parchment)]/70">Load a rule set from the left, or start a new one. Its things, rolls, and sheets show here.</p>
      {/if}
    </div>
  {/if}
</section>
