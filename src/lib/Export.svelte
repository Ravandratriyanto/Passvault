<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import QRCode from "qrcode";

  let { onBack }: { onBack: () => void } = $props();

  let blob = $state("");
  let qrDataUrl = $state("");
  let qrError = $state("");
  let loading = $state(true);
  let copied = $state(false);

  onMount(async () => {
    try {
      blob = await invoke<string>("export_vault");
      try {
        qrDataUrl = await QRCode.toDataURL(blob, {
          errorCorrectionLevel: "L",
          margin: 2,
          width: 320,
        });
      } catch {
        qrError = "Vault too large for a single QR code — use copy or file instead.";
      }
    } catch (e) {
      qrError = String(e);
    } finally {
      loading = false;
    }
  });

  async function copy() {
    await navigator.clipboard.writeText(blob);
    copied = true;
    setTimeout(() => { copied = false; }, 1500);
  }

  function saveFile() {
    const b = new Blob([blob], { type: "text/plain" });
    const url = URL.createObjectURL(b);
    const a = document.createElement("a");
    a.href = url;
    a.download = "vault.passbackup";
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="wrap">
  <div class="header">
    <button class="back" onclick={onBack}>← Back</button>
    <h2>Export vault</h2>
  </div>

  <p class="warn">
    ⚠️ Anyone with this backup <em>and</em> your PIN (and keyfile, if set) can decrypt your vault.
    Keep it private.
  </p>

  {#if loading}
    <p class="loading">Preparing backup…</p>
  {:else}
    {#if qrDataUrl}
      <div class="qr">
        <img src={qrDataUrl} alt="Vault backup QR code" />
        <p class="qr-hint">Scan with a QR reader on your other device</p>
      </div>
    {:else if qrError}
      <p class="qr-fallback">{qrError}</p>
    {/if}

    <div class="actions">
      <button class="btn" onclick={copy}>
        {copied ? "Copied!" : "Copy backup text"}
      </button>
      <button class="btn" onclick={saveFile}>Save as .passbackup file</button>
    </div>

    <details class="raw">
      <summary>Show raw backup text ({blob.length} chars)</summary>
      <textarea readonly value={blob}></textarea>
    </details>
  {/if}
</div>

<style>
  .wrap { max-width: 500px; margin: 0 auto; padding: 32px; display: flex; flex-direction: column; gap: 16px; }
  .header { display: flex; align-items: center; gap: 16px; }
  .header h2 { font-size: 20px; font-weight: 600; }
  .back { background: transparent; color: #a0aec0; font-size: 13px; padding: 4px 8px; }
  .back:hover { color: #e2e8f0; }

  .warn {
    background: rgba(251, 191, 36, 0.1); color: #fbbf24;
    border: 1px solid rgba(251, 191, 36, 0.3); border-radius: 8px;
    padding: 12px; font-size: 13px;
  }
  .warn em { font-style: italic; }

  .qr {
    background: white; padding: 16px; border-radius: 12px;
    display: flex; flex-direction: column; align-items: center; gap: 8px;
  }
  .qr img { display: block; }
  .qr-hint { font-size: 12px; color: #4a5568; }
  .qr-fallback {
    background: #1a202c; border: 1px dashed #4a5568; border-radius: 8px;
    padding: 16px; text-align: center; color: #a0aec0; font-size: 13px;
  }

  .actions { display: flex; gap: 8px; }
  .btn {
    flex: 1; background: #374151; color: #d1d5db;
    padding: 10px; font-size: 13px; border-radius: 8px;
  }
  .btn:hover { background: #4b5563; }

  .raw summary { cursor: pointer; font-size: 12px; color: #6b7280; padding: 4px 0; }
  .raw summary:hover { color: #a0aec0; }
  .raw textarea {
    width: 100%; height: 120px; margin-top: 8px;
    background: #1a202c; color: #cbd5e0;
    font-family: monospace; font-size: 11px;
    border: 1px solid #2d3748; border-radius: 6px; padding: 8px;
    resize: vertical;
  }

  .loading { color: #718096; }
</style>
