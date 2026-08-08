<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Shield, Paperclip, X, Check, ArrowLeft, ArrowRight, Delete, Eye, EyeOff } from "@lucide/svelte";
  import PasswordStrength from "./Passwordstrenght.svelte";
  import { scorePassword } from "./passwordStrenght";

  let { onDone, onRestore }: { onDone: () => void; onRestore: () => void } = $props();

  type Step = "pin" | "confirm-pin" | "password" | "extras";
  let step = $state<Step>("pin");
  let pin = $state("");
  let confirmPin = $state("");
  let password = $state("");
  let confirmPassword = $state("");
  let showPw = $state(false);
  let error = $state("");
  let loading = $state(false);
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let fileInput = $state<HTMLInputElement | undefined>();
  let autostart = $state(true);

  const MIN_PIN = 4;
  const MAX_PIN = 8;
  const currentPin = $derived(step === "pin" ? pin : confirmPin);
  const pwStrong = $derived(scorePassword(password).ok);
  const pwMatches = $derived(password.length > 0 && password === confirmPassword);

  function press(digit: string) {
    if (currentPin.length >= MAX_PIN) return;
    if (step === "pin") pin += digit;
    else confirmPin += digit;
  }
  function backspace() {
    if (step === "pin") pin = pin.slice(0, -1);
    else confirmPin = confirmPin.slice(0, -1);
  }

  function nextFromPin() {
    if (pin.length < MIN_PIN) { error = `PIN must be at least ${MIN_PIN} digits.`; return; }
    error = ""; step = "confirm-pin";
  }
  function nextFromConfirm() {
    if (confirmPin !== pin) { error = "PINs do not match."; confirmPin = ""; return; }
    error = ""; step = "password";
  }
  function nextFromPassword() {
    if (!pwStrong)  { error = "Password is too weak."; return; }
    if (!pwMatches) { error = "Passwords do not match."; return; }
    error = ""; step = "extras";
  }

  async function pickKeyfile(e: Event) {
    const t = e.target as HTMLInputElement;
    const f = t.files?.[0]; if (!f) return;
    keyfileBytes = new Uint8Array(await f.arrayBuffer());
    keyfileName = f.name;
  }
  function clearKeyfile() {
    keyfileBytes = null; keyfileName = "";
    if (fileInput) fileInput.value = "";
  }

  async function submit() {
    loading = true; error = "";
    try {
      await invoke("setup", {
        pin,
        password,
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

    {#if step === "pin" || step === "confirm-pin"}
      <p class="sub">
        {step === "pin" ? "Choose a PIN (4–8 digits)" : "Re-enter your PIN to confirm"}
      </p>
      <div class="dots">
        {#each Array(MAX_PIN) as _, i}
          <span class="dot" class:filled={i < currentPin.length}></span>
        {/each}
      </div>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="pad">
        {#each ["1","2","3","4","5","6","7","8","9"] as d}
          <button class="key" onclick={() => press(d)} disabled={loading}>{d}</button>
        {/each}
        <button class="key ghost" onclick={backspace} aria-label="Backspace"><Delete size={22} /></button>
        <button class="key" onclick={() => press("0")}>0</button>
        {#if step === "pin"}
          <button class="key confirm" onclick={nextFromPin} disabled={pin.length < MIN_PIN} aria-label="Next">
            <ArrowRight size={22} />
          </button>
        {:else}
          <button class="key confirm" onclick={nextFromConfirm} disabled={confirmPin.length === 0} aria-label="Next">
            <ArrowRight size={22} />
          </button>
        {/if}
      </div>
      {#if step === "pin"}
        <button class="restore-link" onclick={onRestore} disabled={loading}>
          Restore from backup instead
        </button>
      {:else}
        <button class="text-btn" onclick={() => { step = "pin"; confirmPin = ""; error = ""; }}>
          <ArrowLeft size={12} /> Back
        </button>
      {/if}
    {:else if step === "password"}
      <p class="sub">Set a vault password (recovery factor)</p>
      <div class="field">
        <div class="pw-row">
          <input
            type={showPw ? "text" : "password"}
            bind:value={password}
            placeholder="Password"
            autocomplete="new-password" />
          <button class="eye" onclick={() => showPw = !showPw} aria-label="Toggle visibility">
            {#if showPw}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
          </button>
        </div>
        <PasswordStrength {password} />
      </div>
      <div class="field">
        <input
          type={showPw ? "text" : "password"}
          bind:value={confirmPassword}
          placeholder="Confirm password"
          autocomplete="new-password" />
      </div>
      {#if error}<p class="error">{error}</p>{/if}
      <p class="hint">
        With PIN + password + optional keyfile, any two can unlock the vault. Recovery works if you forget one.
      </p>
      <div class="row">
        <button class="text-btn" onclick={() => { step = "confirm-pin"; error = ""; }}>
          <ArrowLeft size={12} /> Back
        </button>
        <button class="primary" onclick={nextFromPassword} disabled={!pwStrong || !pwMatches}>
          Next <ArrowRight size={14} />
        </button>
      </div>
    {:else}
      <p class="sub">Optional: add a keyfile & startup preference</p>
      <div class="keyfile">
        <input type="file" bind:this={fileInput} onchange={pickKeyfile} style="display:none" />
        {#if !keyfileBytes}
          <button class="keyfile-btn" onclick={() => fileInput?.click()}>
            <Paperclip size={14} /> Add keyfile (optional 3rd factor)
          </button>
          <p class="keyfile-hint">Boosts recovery: with 3 factors, any 2 unlock. Without it, both PIN and password are required.</p>
        {:else}
          <div class="keyfile-chip">
            <span class="chip-name"><Paperclip size={14} /> {keyfileName}</span>
            <button onclick={clearKeyfile} aria-label="Remove keyfile"><X size={14} /></button>
          </div>
        {/if}
      </div>
      <label class="autostart">
        <input type="checkbox" bind:checked={autostart} disabled={loading} />
        <div class="autostart-text">
          <span class="autostart-title">Start with Windows</span>
          <span class="autostart-hint">Runs hidden in the tray. Press your hotkey to summon it.</span>
        </div>
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="row">
        <button class="text-btn" onclick={() => { step = "password"; error = ""; }} disabled={loading}>
          <ArrowLeft size={12} /> Back
        </button>
        <button class="primary" onclick={submit} disabled={loading}>
          {#if loading}Creating…{:else}<Check size={14} /> Create vault{/if}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .screen { display: flex; align-items: center; justify-content: center; height: 100vh; }
  .card {
    background: #1a202c; border: 1px solid #2d3748; border-radius: 16px;
    padding: 32px; width: 360px;
    display: flex; flex-direction: column; align-items: center; gap: 18px;
  }
  .icon { color: #4f46e5; display: flex; align-items: center; justify-content: center; }
  h1 { font-size: 22px; font-weight: 700; }
  .sub { font-size: 13px; color: #718096; text-align: center; }
  .hint { font-size: 11px; color: #6b7280; text-align: center; line-height: 1.4; }

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

  .row { display: flex; justify-content: space-between; align-items: center; width: 100%; }
  .primary {
    background: #4f46e5; color: white; padding: 8px 14px; border-radius: 8px;
    display: inline-flex; align-items: center; gap: 6px; font-size: 13px;
  }
  .primary:disabled { opacity: 0.4; cursor: not-allowed; }
  .primary:hover:not(:disabled) { background: #4338ca; }
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
