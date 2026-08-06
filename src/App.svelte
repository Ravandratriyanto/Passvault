<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import Setup from "./lib/Setup.svelte";
  import Unlock from "./lib/Unlock.svelte";
  import Vault from "./lib/Vault.svelte";
  import Import from "./lib/Import.svelte";

  type Screen = "loading" | "setup" | "restore" | "unlock" | "vault";
  let screen = $state<Screen>("loading");
  let locking = $state(false);
  let closeUnlisten: UnlistenFn | null = null;

  onMount(async () => {
    const exists = await invoke<boolean>("vault_exists");
    screen = exists ? "unlock" : "setup";

    closeUnlisten = await listen("close-requested", async () => {
      locking = true;
      try {
        await invoke("lock");
        screen = "unlock";
        await new Promise(r => setTimeout(r, 500));
        const win = getCurrentWebviewWindow();
        await win.hide();
        await win.setSkipTaskbar(true);
      } finally {
        locking = false;
      }
    });
  });

  onDestroy(() => { closeUnlisten?.(); });

  function onSetupDone() { screen = "vault"; }
  function onUnlockDone() { screen = "vault"; }
  function onLock() { screen = "unlock"; }
</script>

{#if screen === "loading"}
  <div class="center"><p>Loading...</p></div>
{:else if screen === "setup"}
  <Setup onDone={onSetupDone} onRestore={() => screen = "restore"} />
{:else if screen === "restore"}
  <Import onDone={onSetupDone} onBack={() => screen = "setup"} />
{:else if screen === "unlock"}
  <Unlock onDone={onUnlockDone} />
{:else}
  <Vault onLock={onLock} />
{/if}

{#if locking}
  <div class="lock-overlay">
    <div class="lock-card">
      <div class="spinner"></div>
      <p class="lock-title">Encrypting vault…</p>
      <p class="lock-sub">Wiping keys from memory</p>
    </div>
  </div>
{/if}

<style>
  .center {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: #718096;
  }

  .lock-overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 20, 25, 0.92);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    backdrop-filter: blur(6px);
  }
  .lock-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #2d3748;
    border-top-color: #4f46e5;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  .lock-title { color: #e2e8f0; font-size: 15px; font-weight: 500; }
  .lock-sub { color: #6b7280; font-size: 12px; }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
