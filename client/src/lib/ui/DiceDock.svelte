<script lang="ts">
  import { table } from '../table.svelte';
  import DiceBar from './DiceBar.svelte';
  import DiceStage from './DiceStage.svelte';
  import RollLog from './RollLog.svelte';
</script>

<aside class="pointer-events-none fixed top-0 right-0 z-10 flex h-full w-[26rem] max-w-full flex-col gap-2 p-3">
  <div class="min-h-0 flex-1 overflow-hidden">
    <RollLog />
  </div>
  <div class="shrink-0 overflow-hidden {table.activeRoll ? 'h-56' : 'h-0'}">
    <DiceStage />
  </div>
  <DiceBar />
</aside>
