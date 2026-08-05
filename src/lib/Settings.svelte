<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Export from "./Export.svelte";

  let { onBack }: { onBack: () => void } = $props();
  let showExport = $state(false);

  type Settings = { hotkey: string; autostart: boolean | null };

  let settings = $state<Settings | null>(null);
  let capturing = $state(false);
  let error = $state("");
  let saved = $state("");

  onMount(async () => {
    settings = await invoke<Settings>("get_settings");
  });

  function flash(msg: string) {
    saved = msg;
    setTimeout(() => { saved = ""; }, 1500);
  }

  async function captureKey(e: KeyboardEvent) {
    e.preventDefault();
    if (["Control", "Alt", "Shift", "Meta", "OS"].includes(e.key)) return;
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (mods.length === 0) {
      error = "Include at least one modifier (Ctrl, Alt, Shift).";
      return;
    }
    const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
    const combo = [...mods, key].join("+");
    try {
      await invoke("set_hotkey", { newHotkey: combo });
      if (settings) settings.hotkey = combo;
      error = "";
      capturing = false;
      flash("Hotkey saved");
    } catch (err) {
      error = String(err);
    }
  }

  async function toggleAutostart() {
    if (!settings) return;
    const next = !(settings.autostart ?? false);
    try {
      await invoke("set_autostart", { enabled: next });
      settings.autostart = next;
      flash(next ? "Autostart enabled" : "Autostart disabled");
    } catch (err) {
      error = String(err);
    }
  }
</script>

<div class="wrap">
  <div class="header">
    <button class="back" onclick={onBack}>← Back</button>
    <h2>Settings</h2>
  </div>

  {#if settings}
    <div class="row">
      <div class="label">
        <div class="name">Global hotkey</div>
        <div class="hint">Press this from anywhere to open Passvault</div>
      </div>
      <div class="control">
        {#if capturing}
          <input
            type="text"
            readonly
            value="Press a combo…"
            onkeydown={captureKey}
            class="capture"
          />
          <button onclick={() => { capturing = false; error = ""; }}>Cancel</button>
        {:else}
          <span class="hotkey">{settings.hotkey}</span>
          <button onclick={() => { capturing = true; error = ""; }}>Change</button>
        {/if}
      </div>
    </div>

    <div class="row">
      <div class="label">
        <div class="name">Start with Windows</div>
        <div class="hint">Launches hidden to tray on boot</div>
      </div>
      <div class="control">
        <label class="switch">
          <input type="checkbox" checked={!!settings.autostart} onchange={toggleAutostart} />
          <span class="slider"></span>
        </label>
      </div>
    </div>

    <div class="row">
      <div class="label">
        <div class="name">Export vault</div>
        <div class="hint">Back up your vault as a QR code or file to restore on another device</div>
      </div>
      <div class="control">
        <button onclick={() => showExport = true}>Export…</button>
      </div>
    </div>

    {#if error}<p class="msg error">{error}</p>{/if}
    {#if saved}<p class="msg saved">{saved}</p>{/if}
  {:else}
    <p class="loading">Loading…</p>
  {/if}
</div>

{#if showExport}
  <div class="overlay">
    <div class="modal">
      <Export onBack={() => showExport = false} />
    </div>
  </div>
{/if}

<style>
  .wrap { max-width: 500px; margin: 0 auto; padding: 32px; display: flex; flex-direction: column; gap: 16px; }
  .header { display: flex; align-items: center; gap: 16px; }
  .header h2 { font-size: 20px; font-weight: 600; }
  .back { background: transparent; color: #a0aec0; font-size: 13px; padding: 4px 8px; }
  .back:hover { color: #e2e8f0; }

  .row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px; background: #1a202c; border: 1px solid #2d3748; border-radius: 10px;
    gap: 16px;
  }
  .label { flex: 1; }
  .name { font-size: 14px; font-weight: 500; color: #e2e8f0; }
  .hint { font-size: 12px; color: #6b7280; margin-top: 2px; }
  .control { display: flex; align-items: center; gap: 8px; }
  .hotkey {
    font-family: monospace; font-size: 13px;
    background: #2d3748; color: #cbd5e0;
    padding: 4px 10px; border-radius: 6px;
  }
  .control button {
    background: #374151; color: #d1d5db; font-size: 12px; padding: 4px 10px;
  }
  .control button:hover { background: #4b5563; }
  .capture {
    background: #2d3748; color: #e2e8f0; border: 1px solid #4f46e5;
    padding: 4px 10px; border-radius: 6px; font-family: monospace; font-size: 13px;
    width: 180px; text-align: center; outline: none;
  }

  .switch { position: relative; display: inline-block; width: 40px; height: 22px; }
  .switch input { opacity: 0; width: 0; height: 0; }
  .slider {
    position: absolute; inset: 0; cursor: pointer;
    background: #4a5568; border-radius: 22px; transition: 0.15s;
  }
  .slider::before {
    content: ""; position: absolute; height: 16px; width: 16px;
    left: 3px; top: 3px; background: white; border-radius: 50%; transition: 0.15s;
  }
  .switch input:checked + .slider { background: #4f46e5; }
  .switch input:checked + .slider::before { transform: translateX(18px); }

  .msg { font-size: 13px; padding: 8px 12px; border-radius: 6px; }
  .error { color: #fc8181; background: rgba(252, 129, 129, 0.1); }
  .saved { color: #68d391; background: rgba(104, 211, 145, 0.1); }
  .loading { color: #718096; }

  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 20;
    overflow-y: auto;
  }
  .modal {
    background: #0f1419; border: 1px solid #2d3748; border-radius: 16px;
    max-height: 90vh; overflow-y: auto;
  }
</style>
