<script lang="ts">
  import { send } from '../../lib/net/socket';
  import { table } from '../../lib/table.svelte';
  const followed = new Set<string>();
  // Combat follow-up remains active when the scene chrome is hidden in focus mode.
  $effect(() => {
    const roll = table.activeRoll;
    if (!roll || roll.rollerId !== table.playerId || followed.has(roll.id)) return;
    const contest = roll.contest;
    if (!contest || contest.verdict !== 'hit' || contest.blow) return;
    followed.add(roll.id);
    send({ type: 'strike', attackerId: contest.attackerId, targetId: contest.targetId, plus: 0, minus: 0 });
  });

  import AdversaryMaker from './AdversaryMaker.svelte';
  import { creation } from './creation.svelte';
  import Creator from './Creator.svelte';
</script>

{#if creation.kind === 'pc'}
  <Creator />
{:else if creation.kind === 'adversary'}
  <AdversaryMaker />
{/if}
