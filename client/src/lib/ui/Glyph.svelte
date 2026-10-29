<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
  {#if name === 'heart'}
    <path d="M12 19s-7-4.4-7-9a4 4 0 0 1 7-2 4 4 0 0 1 7 2c0 4.6-7 9-7 9z" fill="currentColor" stroke="none" />
  {:else if name === 'shield'}
    <path d="M12 3 5 6v6c0 4.2 2.8 7 7 9 4.2-2 7-4.8 7-9V6z" />
    <path d="M9 12.2 11 14l4-4.2" />
  {:else if name === 'stress'}
    <path d="M12 3.5 13.6 8h4.8l-3.9 2.8 1.5 4.7L12 12.8 7.9 15.5l1.5-4.7L5.6 8h4.8z" />
  {:else if name === 'hope'}
    <circle cx="12" cy="12" r="3.2" fill="currentColor" stroke="none" />
    <path d="M12 3.2v2.2M12 18.6v2.2M3.2 12h2.2M18.6 12h2.2M5.6 5.6l1.6 1.6M16.8 16.8l1.6 1.6M18.4 5.6l-1.6 1.6M7.2 16.8l-1.6 1.6" />
  {:else if name === 'agility'}
    <path d="M5 16c2.2-6 4-8 7-8 2 0 3.2 1.2 4.2 3" />
    <path d="M14 8.5 19 7l-1.2 5" />
  {:else if name === 'strength'}
    <path d="M7 10.5v6M17 10.5v6M7 13.5h10" />
    <path d="M8.5 7.5a3.5 3.5 0 0 1 7 0" />
    <path d="M6 10.5h2.2M15.8 10.5H18" />
  {:else if name === 'finesse'}
    <path d="M12 19c4-2.4 6-6.2 6-10.2C18 5.6 15.2 4 12 4S6 5.6 6 8.8C6 12.8 8 16.6 12 19z" />
    <path d="M12 4v15" />
  {:else if name === 'instinct'}
    <circle cx="12" cy="12" r="3" />
    <path d="M3 12h4M17 12h4M12 3v4M12 17v4" />
  {:else if name === 'presence'}
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2.8v2.4M12 18.8v2.4M2.8 12h2.4M18.8 12h2.4M5.2 5.2l1.7 1.7M17.1 17.1l1.7 1.7M18.8 5.2l-1.7 1.7M6.9 17.1l-1.7 1.7" />
  {:else if name === 'knowledge'}
    <path d="M5 5.5h6.2A2.8 2.8 0 0 1 14 8.3V19H7.2A2.2 2.2 0 0 0 5 21.2z" />
    <path d="M19 5.5h-6.2A2.8 2.8 0 0 0 10 8.3V19h6.8A2.2 2.2 0 0 1 19 21.2z" />
  {/if}
</svg>
