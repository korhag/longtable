<script lang="ts">
  import { send } from '../net/socket';
  import { table } from '../table.svelte';

  let draft = $state('');
  let scroller = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const count = table.talk.length;
    if (!scroller || count < 0) return;
    scroller.scrollTop = scroller.scrollHeight;
  });

  function submit() {
    const text = draft.trim();
    if (!text) return;
    draft = '';
    send({ type: 'say', text });
  }
</script>

<div class="flex h-full min-h-48 flex-col gap-2">
  <div class="min-h-0 flex-1 overflow-auto" bind:this={scroller}>
    {#if table.talk.length === 0}
      <p class="text-sm text-[var(--color-parchment)]/70">Say something, or roll. It will be said in your character's voice.</p>
    {/if}
    {#each table.talk as line (line.id)}
      <p class="mb-2 text-sm leading-relaxed">
        <span class="display text-[var(--color-gold-bright)]">{line.speaker}</span>
        <span> {line.body}</span>
      </p>
    {/each}
  </div>
  <form class="flex gap-2" onsubmit={(event) => { event.preventDefault(); submit(); }}>
    <input class="field" placeholder="Say it" bind:value={draft} maxlength="280" />
    <button class="btn px-3 py-1" type="submit">Say</button>
  </form>
</div>
