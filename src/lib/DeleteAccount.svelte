<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { onDeleted, onCancel }: { onDeleted: () => void; onCancel: () => void } = $props();

  let pin = $state("");
  let error = $state("");
  let loading = $state(false);
  let needsKeyfile = $state(false);
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let fileInput = $state<HTMLInputElement | undefined>();
  const MAX = 8;

  onMount(async () => {
    try {
      needsKeyfile = await invoke<boolean>("vault_needs_keyfile");
    } catch {}
  });

  function press(digit: string) {
    if (pin.length < MAX) pin += digit;
  }

  function backspace() {
    pin = pin.slice(0, -1);
  }

  async function pickKeyfile(e: Event) {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;
    const buf = await file.arrayBuffer();
    keyfileBytes = new Uint8Array(buf);
    keyfileName = file.name;
  }

  function clearKeyfile() {
    keyfileBytes = null;
    keyfileName = "";
    if (fileInput) fileInput.value = "";
  }

  async function confirm() {
    if (pin.length === 0) return;
    if (needsKeyfile && !keyfileBytes) {
      error = "Keyfile required.";
      return;
    }
    error = "";
    loading = true;
    try {
      await invoke("delete_vault", {
        password: pin,
        keyfile: keyfileBytes ? Array.from(keyfileBytes) : null,
      });
      onDeleted();
    } catch (e) {
      error = String(e);
      pin = "";
    } finally {
      loading = false;
    }
  }
</script>

<div class="overlay">
  <div class="modal">
    <h2>Delete account</h2>
    <p class="warn">This will permanently delete your vault and all saved passwords. Enter your PIN to confirm.</p>

    <div class="dots">
      {#each Array(MAX) as _, i}
        <span class="dot" class:filled={i < pin.length}></span>
      {/each}
    </div>

    {#if needsKeyfile}
      <div class="keyfile">
        <input type="file" bind:this={fileInput} onchange={pickKeyfile} style="display:none" />
        {#if !keyfileBytes}
          <button class="keyfile-btn" onclick={() => fileInput?.click()}>📎 Select keyfile</button>
        {:else}
          <div class="keyfile-chip">
            <span>📎 {keyfileName}</span>
            <button onclick={clearKeyfile}>✕</button>
          </div>
        {/if}
      </div>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    <div class="pad">
      {#each ["1","2","3","4","5","6","7","8","9"] as d}
        <button class="key" onclick={() => press(d)} disabled={loading}>{d}</button>
      {/each}
      <button class="key ghost" onclick={backspace} disabled={loading}>⌫</button>
      <button class="key" onclick={() => press("0")} disabled={loading}>0</button>
      <button class="key confirm" onclick={confirm} disabled={loading || pin.length === 0}>
        {loading ? "…" : "✓"}
      </button>
    </div>

    <button class="cancel" onclick={onCancel}>Cancel</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 20;
  }
  .modal {
    background: #1a202c; border: 1px solid #7f1d1d; border-radius: 16px;
    padding: 32px; width: 320px;
    display: flex; flex-direction: column; align-items: center; gap: 16px;
  }
  h2 { font-size: 18px; font-weight: 600; color: #fc8181; }
  .warn { font-size: 13px; color: #9ca3af; text-align: center; }
  .dots { display: flex; gap: 12px; }
  .dot {
    width: 14px; height: 14px; border-radius: 50%;
    background: #2d3748; border: 2px solid #4a5568; transition: background 0.1s;
  }
  .dot.filled { background: #ef4444; border-color: #ef4444; }

  .keyfile { width: 100%; display: flex; flex-direction: column; align-items: center; }
  .keyfile-btn {
    background: transparent; color: #a0aec0; border: 1px dashed #4a5568;
    padding: 8px 14px; border-radius: 8px; font-size: 13px; width: 100%;
  }
  .keyfile-btn:hover { background: #2d3748; color: #e2e8f0; }
  .keyfile-chip {
    display: flex; align-items: center; gap: 8px;
    background: #4a1c1c; border: 1px solid #ef4444; border-radius: 8px;
    padding: 6px 12px; font-size: 13px; width: 100%; justify-content: space-between;
  }
  .keyfile-chip button { background: transparent; color: #a0aec0; padding: 0 4px; font-size: 14px; }
  .keyfile-chip button:hover { color: #fc8181; }

  .pad { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; width: 100%; }
  .key {
    background: #2d3748; color: #e2e8f0;
    font-size: 20px; font-weight: 500;
    padding: 16px; border-radius: 10px;
    transition: background 0.1s; user-select: none;
  }
  .key:hover:not(:disabled) { background: #4a5568; }
  .key:active:not(:disabled) { background: #ef4444; }
  .key:disabled { opacity: 0.4; cursor: not-allowed; }
  .key.ghost { background: transparent; font-size: 22px; }
  .key.ghost:hover:not(:disabled) { background: #2d3748; }
  .key.confirm { background: #ef4444; color: white; font-size: 22px; }
  .key.confirm:hover:not(:disabled) { background: #dc2626; }
  .cancel { background: #374151; color: #d1d5db; padding: 8px 24px; border-radius: 6px; font-size: 14px; }
  .cancel:hover { background: #4b5563; }
  .error { color: #fc8181; font-size: 13px; }
</style>
