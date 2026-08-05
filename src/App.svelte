<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Setup from "./lib/Setup.svelte";
  import Unlock from "./lib/Unlock.svelte";
  import Vault from "./lib/Vault.svelte";
  import Import from "./lib/Import.svelte";

  type Screen = "loading" | "setup" | "restore" | "unlock" | "vault";
  let screen = $state<Screen>("loading");

  onMount(async () => {
    const exists = await invoke<boolean>("vault_exists");
    screen = exists ? "unlock" : "setup";
  });

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

<style>
  .center {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: #718096;
  }
</style>
