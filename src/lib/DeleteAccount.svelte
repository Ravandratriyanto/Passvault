<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { Paperclip, X, Check, Delete, Eye, EyeOff } from "@lucide/svelte";

  let { onDeleted, onCancel }: { onDeleted: () => void; onCancel: () => void } = $props();

  type VaultInfo = { version: number; needs_pin: boolean; needs_password: boolean; needs_keyfile: boolean; };

  let info = $state<VaultInfo | null>(null);
  let pin = $state("");
  let password = $state("");
  let showPw = $state(false);
  let error = $state("");
  let loading = $state(false);
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let fileInput = $state<HTMLInputElement | undefined>();
  const MAX_PIN = 8;

  const isV3 = $derived(info?.version === 3);
  const factorsProvided = $derived(
    (pin.length > 0 ? 1 : 0) + (password.length > 0 ? 1 : 0) + (keyfileBytes ? 1 : 0)
  );
  const canSubmit = $derived.by(() => {
    if (!info) return false;
    if (info.version <= 2) {
      return pin.length > 0 && (!info.needs_keyfile || !!keyfileBytes);
    }
    return factorsProvided >= 2;
  });

  onMount(async () => {
    try { info = await invoke<VaultInfo>("vault_info"); } catch {}
  });

  function press(d: string) { if (pin.length < MAX_PIN) pin += d; }
  function backspace() { pin = pin.slice(0, -1); }

  async function pickKeyfile(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0]; if (!f) return;
    keyfileBytes = new Uint8Array(await f.arrayBuffer());
    keyfileName = f.name;
  }
  function clearKeyfile() {
    keyfileBytes = null; keyfileName = "";
    if (fileInput) fileInput.value = "";
  }

  async function confirm() {
    if (!canSubmit) return;
    error = ""; loading = true;
    try {
      await invoke("delete_vault", {
        pin: pin || null,
        password: password || null,
        keyfile: keyfileBytes ? Array.from(keyfileBytes) : null,
      });
      onDeleted();
    } catch (e) {
      error = String(e);
      pin = ""; password = "";
    } finally {
      loading = false;
    }
  }
</script>

<div class="overlay">
  <div class="modal">
    <h2>Delete account</h2>
    <p class="warn">
      This will permanently delete your vault and all saved passwords.
      {#if isV3}Provide any 2 factors to confirm.{:else}Enter your PIN to confirm.{/if}
    </p>

    {#if !info || info.needs_pin}
      <div class="dots">
        {#each Array(MAX_PIN) as _, i}
          <span class="dot" class:filled={i < pin.length}></span>
        {/each}
      </div>
    {/if}

    {#if isV3 && info?.needs_password}
      <div class="field">
        <div class="pw-row">
          <input
            type={showPw ? "text" : "password"}
            bind:value={password}
            placeholder="Vault password"
            autocomplete="current-password"
            disabled={loading} />
          <button class="eye" onclick={() => showPw = !showPw} aria-label="Toggle visibility">
            {#if showPw}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
          </button>
        </div>
      </div>
    {/if}

    {#if info?.needs_keyfile}
      <div class="keyfile">
        <input type="file" bind:this={fileInput} onchange={pickKeyfile} style="display:none" />
        {#if !keyfileBytes}
          <button class="keyfile-btn" onclick={() => fileInput?.click()}>
            <Paperclip size={14} /> Select keyfile
          </button>
        {:else}
          <div class="keyfile-chip">
            <span class="chip-name"><Paperclip size={14} /> {keyfileName}</span>
            <button onclick={clearKeyfile} aria-label="Remove keyfile"><X size={14} /></button>
          </div>
        {/if}
      </div>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    {#if !info || info.needs_pin}
      <div class="pad">
        {#each ["1","2","3","4","5","6","7","8","9"] as d}
          <button class="key" onclick={() => press(d)} disabled={loading}>{d}</button>
        {/each}
        <button class="key ghost" onclick={backspace} disabled={loading} aria-label="Backspace">
          <Delete size={22} />
        </button>
        <button class="key" onclick={() => press("0")} disabled={loading}>0</button>
        <button class="key confirm" onclick={confirm} disabled={loading || !canSubmit} aria-label="Confirm delete">
          {#if loading}…{:else}<Check size={22} />{/if}
        </button>
      </div>
    {:else}
      <button class="danger" onclick={confirm} disabled={loading || !canSubmit}>
        {#if loading}Deleting…{:else}Delete vault{/if}
      </button>
    {/if}

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
    padding: 32px; width: 340px;
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

  .field { width: 100%; }
  .field input {
    width: 100%; background: #0f1420; border: 1px solid #2d3748;
    color: #e2e8f0; padding: 10px 12px; border-radius: 8px; font-size: 14px;
  }
  .field input:focus { outline: none; border-color: #ef4444; }
  .pw-row { position: relative; }
  .eye {
    position: absolute; right: 8px; top: 50%; transform: translateY(-50%);
    background: transparent; color: #718096; padding: 4px;
    display: flex; align-items: center;
  }
  .eye:hover { color: #e2e8f0; }

  .keyfile { width: 100%; display: flex; flex-direction: column; align-items: center; }
  .keyfile-btn {
    background: transparent; color: #a0aec0; border: 1px dashed #4a5568;
    padding: 8px 14px; border-radius: 8px; font-size: 13px; width: 100%;
    display: flex; align-items: center; justify-content: center; gap: 6px;
  }
  .keyfile-btn:hover { background: #2d3748; color: #e2e8f0; }
  .keyfile-chip {
    display: flex; align-items: center; gap: 8px;
    background: #4a1c1c; border: 1px solid #ef4444; border-radius: 8px;
    padding: 6px 12px; font-size: 13px; width: 100%; justify-content: space-between;
  }
  .chip-name { display: inline-flex; align-items: center; gap: 6px; }
  .keyfile-chip button {
    background: transparent; color: #a0aec0; padding: 2px;
    display: flex; align-items: center; justify-content: center;
  }
  .keyfile-chip button:hover { color: #fc8181; }

  .pad { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; width: 100%; }
  .key {
    background: #2d3748; color: #e2e8f0;
    font-size: 20px; font-weight: 500;
    padding: 16px; border-radius: 10px;
    display: flex; align-items: center; justify-content: center;
  }
  .key:hover:not(:disabled) { background: #4a5568; }
  .key:active:not(:disabled) { background: #ef4444; }
  .key:disabled { opacity: 0.4; cursor: not-allowed; }
  .key.ghost { background: transparent; }
  .key.ghost:hover:not(:disabled) { background: #2d3748; }
  .key.confirm { background: #ef4444; color: white; }
  .key.confirm:hover:not(:disabled) { background: #dc2626; }

  .danger {
    background: #ef4444; color: white; padding: 10px 24px; border-radius: 8px;
    font-size: 14px; font-weight: 500; width: 100%;
  }
  .danger:hover:not(:disabled) { background: #dc2626; }
  .danger:disabled { opacity: 0.4; cursor: not-allowed; }

  .cancel { background: #374151; color: #d1d5db; padding: 8px 24px; border-radius: 6px; font-size: 14px; }
  .cancel:hover { background: #4b5563; }
  .error { color: #fc8181; font-size: 13px; }
</style>
