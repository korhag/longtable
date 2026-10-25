<script lang="ts">
  import DiceBar from './DiceBar.svelte';
  import RollLog from './RollLog.svelte';
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
  <div class="min-h-0 flex-1 overflow-hidden">
    <RollLog />
  </div>
  <DiceBar />
</div>
