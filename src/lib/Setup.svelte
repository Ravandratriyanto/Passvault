<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Shield, Paperclip, X, Check, ArrowLeft, ArrowRight, Delete } from "@lucide/svelte";

  let { onDone, onRestore }: { onDone: () => void; onRestore: () => void } = $props();

  type Step = "pin" | "confirm";
  let step = $state<Step>("pin");
  let pin = $state("");
  let confirmPin = $state("");
  let error = $state("");
  let loading = $state(false);
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let fileInput = $state<HTMLInputElement | undefined>();
  let autostart = $state(true);

  const MIN = 4;
  const MAX = 8;

  const current = $derived(step === "pin" ? pin : confirmPin);

  function press(digit: string) {
    if (current.length >= MAX) return;
    if (step === "pin") pin += digit;
    else confirmPin += digit;
  }

  function backspace() {
    if (step === "pin") pin = pin.slice(0, -1);
    else confirmPin = confirmPin.slice(0, -1);
  }

  function next() {
    if (pin.length < MIN) {
      error = `PIN must be at least ${MIN} digits.`;
      return;
    }
    error = "";
    step = "confirm";
  }

  function back() {
    step = "pin";
    confirmPin = "";
    error = "";
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

  async function submit() {
    if (confirmPin !== pin) {
      error = "PINs do not match.";
      confirmPin = "";
      return;
    }
    loading = true;
    error = "";
    try {
      await invoke("setup", {
        password: pin,
        keyfile: keyfileBytes ? Array.from(keyfileBytes) : null,
        autostart,
      });
      onDone();
    } catch (e) {
      error = String(e);
      loading = false;
    }
  }
</script>

<div class="screen">
  <div class="card">
    <div class="icon"><Shield size={40} strokeWidth={1.5} /></div>
    <h1>Create your vault</h1>
    <p class="sub">
      {step === "pin" ? "Choose a PIN (4–8 digits)" : "Re-enter your PIN to confirm"}
    </p>

    <div class="dots">
      {#each Array(MAX) as _, i}
        <span class="dot" class:filled={i < current.length}></span>
      {/each}
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="pad">
      {#each ["1","2","3","4","5","6","7","8","9"] as d}
        <button class="key" onclick={() => press(d)} disabled={loading}>{d}</button>
      {/each}
      <button class="key ghost" onclick={backspace} disabled={loading} aria-label="Backspace">
        <Delete size={22} />
      </button>
      <button class="key" onclick={() => press("0")} disabled={loading}>0</button>
      {#if step === "pin"}
        <button class="key confirm" onclick={next} disabled={pin.length < MIN} aria-label="Next">
          <ArrowRight size={22} />
        </button>
      {:else}
        <button class="key confirm" onclick={submit} disabled={loading || confirmPin.length === 0} aria-label="Confirm">
          {#if loading}…{:else}<Check size={22} />{/if}
        </button>
      {/if}
    </div>

    {#if step === "pin"}
      <div class="keyfile">
        <input type="file" bind:this={fileInput} onchange={pickKeyfile} style="display:none" />
        {#if !keyfileBytes}
          <button class="keyfile-btn" onclick={() => fileInput?.click()}>
            <Paperclip size={14} /> Add keyfile (optional 2FA)
          </button>
          <p class="keyfile-hint">A file you'll need every time you unlock. Photo, doc, anything.</p>
        {:else}
          <div class="keyfile-chip">
            <span class="chip-name"><Paperclip size={14} /> {keyfileName}</span>
            <button onclick={clearKeyfile} aria-label="Remove keyfile"><X size={14} /></button>
          </div>
          <p class="keyfile-hint">Keep this file safe. Without it, the vault cannot be unlocked.</p>
        {/if}
      </div>
      <button class="restore-link" onclick={onRestore} disabled={loading}>
        Restore from backup instead
      </button>
    {:else}
      <label class="autostart">
        <input type="checkbox" bind:checked={autostart} disabled={loading} />
        <div class="autostart-text">
          <span class="autostart-title">Start with Windows</span>
          <span class="autostart-hint">Runs hidden in the tray. Press your hotkey to summon it.</span>
        </div>
      </label>
      <button class="text-btn" onclick={back} disabled={loading}>
        <ArrowLeft size={12} /> Back
      </button>
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
  .icon { color: #4f46e5; display: flex; align-items: center; justify-content: center; }
  h1 { font-size: 22px; font-weight: 700; }
  .sub { font-size: 13px; color: #718096; text-align: center; }

  .dots { display: flex; gap: 12px; }
  .dot {
    width: 14px; height: 14px; border-radius: 50%;
    background: #2d3748; border: 2px solid #4a5568; transition: background 0.1s;
  }
  .dot.filled { background: #4f46e5; border-color: #4f46e5; }

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

  .keyfile { width: 100%; display: flex; flex-direction: column; gap: 6px; align-items: center; }
  .keyfile-btn {
    background: transparent; color: #a0aec0; border: 1px dashed #4a5568;
    padding: 8px 14px; border-radius: 8px; font-size: 13px; width: 100%;
    display: flex; align-items: center; justify-content: center; gap: 6px;
  }
  .keyfile-btn:hover { background: #2d3748; color: #e2e8f0; }
  .keyfile-hint { font-size: 11px; color: #6b7280; text-align: center; }
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

  .text-btn {
    background: transparent; color: #a0aec0; font-size: 13px; padding: 4px 8px;
    display: inline-flex; align-items: center; gap: 4px;
  }
  .text-btn:hover { color: #e2e8f0; }

  .restore-link {
    background: transparent; color: #6b7280; font-size: 12px;
    padding: 4px 8px; text-decoration: underline;
  }
  .restore-link:hover:not(:disabled) { color: #a0aec0; }

  .autostart {
    display: flex; align-items: flex-start; gap: 10px;
    width: 100%; padding: 12px; border-radius: 8px;
    background: #1e293b; border: 1px solid #2d3748; cursor: pointer;
  }
  .autostart input { margin-top: 3px; accent-color: #4f46e5; }
  .autostart-text { display: flex; flex-direction: column; gap: 2px; }
  .autostart-title { font-size: 13px; color: #e2e8f0; font-weight: 500; }
  .autostart-hint { font-size: 11px; color: #6b7280; }

  .error { color: #fc8181; font-size: 13px; text-align: center; }
</style>
