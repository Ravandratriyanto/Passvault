<script lang="ts">
  import { scorePassword } from "./passwordStrenght";

  let { password }: { password: string } = $props();
  let result = $derived(scorePassword(password));
</script>

{#if password}
  <div class="wrap">
    <div class="bar">
      <div class="fill" style="width: {result.pct}%; background: {result.color};"></div>
    </div>
    <span class="label" style="color: {result.color};">{result.label}</span>
  </div>
{/if}

<style>
  .wrap  { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .bar   { flex: 1; height: 4px; background: #2d3748; border-radius: 2px; overflow: hidden; }
  .fill  { height: 100%; transition: width 0.2s, background 0.2s; }
  .label { font-size: 11px; font-weight: 500; min-width: 80px; text-align: right; }
</style>
