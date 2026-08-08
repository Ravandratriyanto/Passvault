<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { ArrowLeft, Paperclip, X, Check, Delete, Eye, EyeOff } from "@lucide/svelte";

  let { onDone, onBack }: { onDone: () => void; onBack: () => void } = $props();

  let blob = $state("");
  let keyfileBytes = $state<Uint8Array | null>(null);
  let keyfileName = $state("");
  let pin = $state("");
  let password = $state("");
  let showPw = $state(false);
  let error = $state("");
  let loading = $state(false);
  let backupFileInput = $state<HTMLInputElement | undefined>();
  let keyfileInput = $state<HTMLInputElement | undefined>();

  const MAX_PIN = 8;

  function press(digit: string) {
    if (pin.length < MAX_PIN) pin += digit;
  }
  function backspace() {
    pin = pin.slice(0, -1);
  }

  async function pickBackupFile(e: Event) {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;
    blob = await file.text();
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
    if (keyfileInput) keyfileInput.value = "";
  }

  async function submit() {
    if (!blob.trim()) {
      error = "Paste your backup or upload a .passbackup file.";
      return;
    }
    if (pin.length === 0 && password.length === 0 && !keyfileBytes) {
      error = "Provide at least one factor (PIN, password, or keyfile).";
      return;
    }
    error = "";
    loading = true;
    try {
      await invoke("import_vault", {
        blobB64: blob,
        pin: pin || null,
        password: password || null,
        keyfile: keyfileBytes ? Array.from(keyfileBytes) : null,
      });
      onDone();
    } catch (e) {
      error = String(e);
      pin = ""; password = "";
    } finally {
      loading = false;
    }
  }
</script>

<div class="screen">
  <div class="card">
    <div class="header">
      <button class="back" onclick={onBack}><ArrowLeft size={13} /> Back</button>
      <h1>Restore from backup</h1>
    </div>

    <div class="section">
      <label class="section-label">Backup data</label>
      <textarea
        placeholder="Paste backup text here…"
        bind:value={blob}
        disabled={loading}
      ></textarea>
      <input
        type="file"
        accept=".passbackup,.txt"
        bind:this={backupFileInput}
        onchange={pickBackupFile}
        style="display:none"
      />
      <button class="secondary" onclick={() => backupFileInput?.click()} disabled={loading}>
        Or upload .passbackup file
      </button>
    </div>

    <div class="section">
      <label class="section-label">Vault password (if the backup used one)</label>
      <div class="pw-row">
        <input
          type={showPw ? "text" : "password"}
          bind:value={password}
          placeholder="Password"
          autocomplete="current-password"
          disabled={loading} />
        <button class="eye" onclick={() => showPw = !showPw} aria-label="Toggle visibility">
          {#if showPw}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
        </button>
      </div>
    </div>

    <div class="section">
      <label class="section-label">Keyfile (if the backup used one)</label>
      <input
        type="file"
        bind:this={keyfileInput}
        onchange={pickKeyfile}
        style="display:none"
      />
      {#if !keyfileBytes}
        <button class="secondary" onclick={() => keyfileInput?.click()} disabled={loading}>
          <Paperclip size={14} /> Select keyfile
        </button>
      {:else}
        <div class="chip">
          <span class="chip-name"><Paperclip size={14} /> {keyfileName}</span>
          <button onclick={clearKeyfile} aria-label="Remove keyfile"><X size={14} /></button>
        </div>
      {/if}
    </div>

    <div class="section">
      <label class="section-label">PIN</label>
      <div class="dots">
        {#each Array(MAX_PIN) as _, i}
          <span class="dot" class:filled={i < pin.length}></span>
        {/each}
      </div>
      <div class="pad">
        {#each ["1","2","3","4","5","6","7","8","9"] as d}
          <button class="key" onclick={() => press(d)} disabled={loading}>{d}</button>
        {/each}
        <button class="key ghost" onclick={backspace} disabled={loading} aria-label="Backspace">
          <Delete size={20} />
        </button>
        <button class="key" onclick={() => press("0")} disabled={loading}>0</button>
        <button class="key confirm" onclick={submit} disabled={loading} aria-label="Restore">
          {#if loading}…{:else}<Check size={20} />{/if}
        </button>
      </div>
    </div>

    <p class="hint">v3 backups need any 2 factors. Legacy backups just need the PIN (+ keyfile if it had one).</p>
    {#if error}<p class="error">{error}</p>{/if}
  </div>
</div>

<style>
  .screen { display: flex; align-items: center; justify-content: center; min-height: 100vh; padding: 32px 0; }
  .card {
    background: #1a202c; border: 1px solid #2d3748; border-radius: 16px;
    padding: 24px; width: 380px;
    display: flex; flex-direction: column; gap: 16px;
  }
  .header { display: flex; align-items: center; gap: 12px; }
  .back { background: transparent; color: #a0aec0; font-size: 13px; padding: 4px 8px; display: inline-flex; align-items: center; gap: 4px; }
  .back:hover { color: #e2e8f0; }
  h1 { font-size: 18px; font-weight: 600; }

  .section { display: flex; flex-direction: column; gap: 6px; }
  .section-label {
    font-size: 11px; color: #6b7280; text-transform: uppercase; letter-spacing: 0.05em;
  }
  textarea {
    background: #111827; color: #cbd5e0;
    border: 1px solid #2d3748; border-radius: 6px; padding: 8px;
    font-family: monospace; font-size: 11px; height: 70px; resize: vertical;
  }
  .secondary {
    background: transparent; color: #a0aec0; border: 1px dashed #4a5568;
    padding: 8px; border-radius: 6px; font-size: 12px;
    display: inline-flex; align-items: center; justify-content: center; gap: 6px;
  }
  .secondary:hover { background: #2d3748; color: #e2e8f0; }
  .chip {
    display: flex; align-items: center; justify-content: space-between;
    background: #1e1b4b; border: 1px solid #4f46e5; border-radius: 6px;
    padding: 6px 10px; font-size: 12px;
  }
  .chip-name { display: inline-flex; align-items: center; gap: 6px; }
  .chip button {
    background: transparent; color: #a0aec0; padding: 2px;
    display: flex; align-items: center; justify-content: center;
  }
  .chip button:hover { color: #fc8181; }

  .pw-row { position: relative; }
  .pw-row input {
    width: 100%; background: #111827; color: #cbd5e0;
    border: 1px solid #2d3748; border-radius: 6px; padding: 8px 30px 8px 8px;
    font-size: 13px;
  }
  .pw-row input:focus { outline: none; border-color: #4f46e5; }
  .eye {
    position: absolute; right: 6px; top: 50%; transform: translateY(-50%);
    background: transparent; color: #718096; padding: 4px;
    display: flex; align-items: center;
  }
  .eye:hover { color: #e2e8f0; }

  .dots { display: flex; gap: 10px; justify-content: center; padding: 4px 0; }
  .dot {
    width: 12px; height: 12px; border-radius: 50%;
    background: #2d3748; border: 2px solid #4a5568;
  }
  .dot.filled { background: #4f46e5; border-color: #4f46e5; }

  .pad { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .key {
    background: #2d3748; color: #e2e8f0;
    font-size: 18px; font-weight: 500;
    padding: 12px; border-radius: 8px;
    display: flex; align-items: center; justify-content: center;
  }
  .key:hover:not(:disabled) { background: #4a5568; }
  .key:disabled { opacity: 0.4; cursor: not-allowed; }
  .key.ghost { background: transparent; }
  .key.ghost:hover:not(:disabled) { background: #2d3748; }
  .key.confirm { background: #4f46e5; color: white; }
  .key.confirm:hover:not(:disabled) { background: #4338ca; }

  .hint { font-size: 11px; color: #6b7280; text-align: center; line-height: 1.4; }
  .error { color: #fc8181; font-size: 13px; text-align: center; }
</style>
