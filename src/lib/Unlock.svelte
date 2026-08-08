<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Lock, Paperclip, X, Check, Delete, Eye, EyeOff } from "@lucide/svelte";

  let { onDone }: { onDone: () => void } = $props();

  type VaultInfo = { version: number; needs_pin: boolean; needs_password: boolean; needs_keyfile: boolean; };

  let info = $state<VaultInfo | null>(null);
  let pin = $state("");
  let password = $state("");
  let showPw = $state(false);
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let fileInput = $state<HTMLInputElement | undefined>();
  let error = $state("");
  let loading = $state(false);
  let lockoutSeconds = $state(0);
  let countdownTimer: ReturnType<typeof setInterval> | null = null;

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
  onDestroy(() => { if (countdownTimer) clearInterval(countdownTimer); });

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

  function startCountdown(sec: number) {
    lockoutSeconds = sec;
    if (countdownTimer) clearInterval(countdownTimer);
    countdownTimer = setInterval(() => {
      lockoutSeconds -= 1;
      if (lockoutSeconds <= 0) {
        if (countdownTimer) clearInterval(countdownTimer);
        countdownTimer = null;
        error = "";
      }
    }, 1000);
  }

  async function submit() {
    if (!canSubmit || lockoutSeconds > 0) return;
    loading = true; error = "";
    try {
      await invoke("unlock", {
        pin: pin || null,
        password: password || null,
        keyfile: keyfileBytes ? Array.from(keyfileBytes) : null,
      });
      onDone();
    } catch (e) {
      const msg = String(e);
      const m = msg.match(/wait (\d+)s/);
      if (m) startCountdown(parseInt(m[1], 10));
      error = msg;
      pin = ""; password = "";
    } finally {
      loading = false;
    }
  }
</script>

<div class="screen">
  <div class="card">
    <div class="icon"><Lock size={40} strokeWidth={1.5} /></div>
    <h1>Onyxlock</h1>

    {#if isV3}
      <p class="sub">Provide any 2 factors to unlock</p>
    {/if}

    {#if !info || info.needs_pin}
      <div class="dots">
        {#each Array(MAX_PIN) as _, i}
          <span class="dot" class:filled={i < pin.length}></span>
        {/each}
      </div>
      <div class="pad">
        {#each ["1","2","3","4","5","6","7","8","9"] as d}
          <button class="key" onclick={() => press(d)} disabled={loading || lockoutSeconds > 0}>{d}</button>
        {/each}
        <button class="key ghost" onclick={backspace} aria-label="Backspace"><Delete size={22} /></button>
        <button class="key" onclick={() => press("0")} disabled={loading || lockoutSeconds > 0}>0</button>
        <button class="key confirm" onclick={submit}
                disabled={loading || lockoutSeconds > 0 || !canSubmit}
                aria-label="Unlock">
          {#if loading}…{:else}<Check size={22} />{/if}
        </button>
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
            disabled={loading || lockoutSeconds > 0}
            onkeydown={(e) => e.key === "Enter" && submit()} />
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

    {#if lockoutSeconds > 0}
      <p class="error"><Lock size={13} /> Locked — try again in {lockoutSeconds}s</p>
    {:else if error}
      <p class="error">{error}</p>
    {/if}
  </div>
</div>

<style>
  .screen { display: flex; align-items: center; justify-content: center; height: 100vh; }
  .card {
    background: #1a202c; border: 1px solid #2d3748; border-radius: 16px;
    padding: 32px; width: 340px;
    display: flex; flex-direction: column; align-items: center; gap: 18px;
  }
  .icon { color: #4f46e5; }
  h1 { font-size: 24px; font-weight: 700; }
  .sub { font-size: 12px; color: #718096; text-align: center; }

  .dots { display: flex; gap: 12px; }
  .dot {
    width: 14px; height: 14px; border-radius: 50%;
    background: #2d3748; border: 2px solid #4a5568;
  }
  .dot.filled { background: #4f46e5; border-color: #4f46e5; }

  .pad { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; width: 100%; }
  .key {
    background: #2d3748; color: #e2e8f0;
    font-size: 20px; font-weight: 500;
    padding: 16px; border-radius: 10px;
    display: flex; align-items: center; justify-content: center;
  }
  .key:hover:not(:disabled) { background: #4a5568; }
  .key:disabled { opacity: 0.4; cursor: not-allowed; }
  .key.ghost { background: transparent; }
  .key.confirm { background: #4f46e5; color: white; }
  .key.confirm:hover:not(:disabled) { background: #4338ca; }

  .field { width: 100%; }
  .field input {
    width: 100%; background: #0f1420; border: 1px solid #2d3748;
    color: #e2e8f0; padding: 10px 12px; border-radius: 8px; font-size: 14px;
  }
  .field input:focus { outline: none; border-color: #4f46e5; }
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
    background: #1e1b4b; border: 1px solid #4f46e5; border-radius: 8px;
    padding: 6px 12px; font-size: 13px; width: 100%; justify-content: space-between;
  }
  .chip-name { display: inline-flex; align-items: center; gap: 6px; }
  .keyfile-chip button {
    background: transparent; color: #a0aec0; padding: 2px;
    display: flex; align-items: center; justify-content: center;
  }
  .keyfile-chip button:hover { color: #fc8181; }

  .error {
    color: #fc8181; font-size: 13px; text-align: center;
    display: inline-flex; align-items: center; gap: 6px; justify-content: center;
  }
</style>
