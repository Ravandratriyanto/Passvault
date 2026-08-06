<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Lock, Paperclip, X, Check, Delete } from "@lucide/svelte";

  let { onDone }: { onDone: () => void } = $props();

  let pin = $state("");
  let error = $state("");
  let loading = $state(false);
  let needsKeyfile = $state(false);
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let lockoutSeconds = $state(0);
  let fileInput = $state<HTMLInputElement | undefined>();
  let countdownTimer: ReturnType<typeof setInterval> | null = null;

  const MAX = 8;

  onMount(async () => {
    try {
      needsKeyfile = await invoke<boolean>("vault_needs_keyfile");
    } catch {}
  });

  onDestroy(() => {
    if (countdownTimer) clearInterval(countdownTimer);
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

  function startCountdown(seconds: number) {
    lockoutSeconds = seconds;
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
    if (pin.length === 0) return;
    if (needsKeyfile && !keyfileBytes) {
      error = "Keyfile required.";
      return;
    }
    if (lockoutSeconds > 0) return;
    error = "";
    loading = true;
    try {
      await invoke("unlock", {
        password: pin,
        keyfile: keyfileBytes ? Array.from(keyfileBytes) : null,
      });
      onDone();
    } catch (e) {
      const msg = String(e);
      const match = msg.match(/wait (\d+)s/);
      if (match) startCountdown(parseInt(match[1], 10));
      error = msg;
      pin = "";
    } finally {
      loading = false;
    }
  }
</script>

<div class="screen">
  <div class="card">
    <div class="icon"><Lock size={40} strokeWidth={1.5} /></div>
    <h1>Passvault</h1>

    <div class="dots">
      {#each Array(MAX) as _, i}
        <span class="dot" class:filled={i < pin.length}></span>
      {/each}
    </div>

    {#if needsKeyfile}
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

    <div class="pad">
      {#each ["1","2","3","4","5","6","7","8","9"] as d}
        <button class="key" onclick={() => press(d)} disabled={loading || lockoutSeconds > 0}>{d}</button>
      {/each}
      <button class="key ghost" onclick={backspace} disabled={loading || lockoutSeconds > 0} aria-label="Backspace">
        <Delete size={22} />
      </button>
      <button class="key" onclick={() => press("0")} disabled={loading || lockoutSeconds > 0}>0</button>
      <button class="key confirm" onclick={submit} disabled={loading || lockoutSeconds > 0 || pin.length === 0} aria-label="Unlock">
        {#if loading}…{:else}<Check size={22} />{/if}
      </button>
    </div>
  </div>
</div>

<style>
  .screen { display: flex; align-items: center; justify-content: center; height: 100vh; }
  .card {
    background: #1a202c; border: 1px solid #2d3748; border-radius: 16px;
    padding: 40px 32px; width: 320px;
    display: flex; flex-direction: column; align-items: center; gap: 20px;
  }
  .icon { color: #4f46e5; display: flex; align-items: center; justify-content: center; }
  h1 { font-size: 24px; font-weight: 700; }

  .dots { display: flex; gap: 12px; }
  .dot {
    width: 14px; height: 14px; border-radius: 50%;
    background: #2d3748; border: 2px solid #4a5568; transition: background 0.1s;
  }
  .dot.filled { background: #4f46e5; border-color: #4f46e5; }

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

  .pad { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; width: 100%; }
  .key {
    background: #2d3748; color: #e2e8f0;
    font-size: 20px; font-weight: 500;
    padding: 16px; border-radius: 10px;
    transition: background 0.1s; user-select: none;
    display: flex; align-items: center; justify-content: center;
  }
  .key:hover:not(:disabled) { background: #4a5568; }
  .key:active:not(:disabled) { background: #4f46e5; }
  .key:disabled { opacity: 0.4; cursor: not-allowed; }
  .key.ghost { background: transparent; }
  .key.ghost:hover:not(:disabled) { background: #2d3748; }
  .key.confirm { background: #4f46e5; color: white; }
  .key.confirm:hover:not(:disabled) { background: #4338ca; }

  .error {
    color: #fc8181; font-size: 13px; text-align: center;
    display: inline-flex; align-items: center; gap: 6px; justify-content: center;
  }
</style>
