<script lang="ts">
  import AdversaryMaker from './AdversaryMaker.svelte';
  import { creation } from './creation.svelte';
  import Creator from './Creator.svelte';
</script>

{#if creation.kind === 'pc'}
  <Creator />
{:else if creation.kind === 'adversary'}
  <AdversaryMaker />
{/if}
