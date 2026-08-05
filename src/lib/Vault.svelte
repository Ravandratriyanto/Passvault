<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import AddEntry from "./AddEntry.svelte";
  import DeleteAccount from "./DeleteAccount.svelte";
  import Settings from "./Settings.svelte";
  let showDelete = $state(false);
  let showSettings = $state(false);

  let { onLock }: { onLock: () => void } = $props();

  const IDLE_MS = 5 * 60 * 1000;
  const CLIPBOARD_CLEAR_MS = 20 * 1000;
  const activityEvents = ["mousemove", "keydown", "click", "touchstart"] as const;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;
  let clipboardTimer: ReturnType<typeof setTimeout> | null = null;
  let lastCopied = "";

  type Entry = {
    id: string; title: string; username: string; password: string;
    url: string | null; notes: string | null; category: string | null;
    totp_secret: string | null; created_at: number; updated_at: number;
  };

  let entries = $state<Entry[]>([]);
  let search = $state("");
  let selected = $state<Entry | null>(null);
  let showPassword = $state(false);
  let showAdd = $state(false);
  let copied = $state("");

  let filtered = $derived(
    entries.filter(e =>
      e.title.toLowerCase().includes(search.toLowerCase()) ||
      e.username.toLowerCase().includes(search.toLowerCase())
    )
  );

  function resetIdle() {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(() => { lock(); }, IDLE_MS);
  }

  onMount(async () => {
    entries = await invoke<Entry[]>("get_entries");
    activityEvents.forEach(e => window.addEventListener(e, resetIdle));
    resetIdle();
  });

  onDestroy(() => {
    if (idleTimer) clearTimeout(idleTimer);
    if (clipboardTimer) clearTimeout(clipboardTimer);
    activityEvents.forEach(e => window.removeEventListener(e, resetIdle));
  });

  async function lock() {
    if (clipboardTimer) {
      clearTimeout(clipboardTimer);
      clipboardTimer = null;
      try { await navigator.clipboard.writeText(""); } catch {}
    }
    await invoke("lock");
    onLock();
  }

  async function deleteEntry(id: string) {
    if (!confirm("Delete this entry?")) return;
    await invoke("delete_entry", { id });
    entries = entries.filter(e => e.id !== id);
    if (selected?.id === id) selected = null;
  }

  async function copyToClipboard(text: string, field: string) {
    await navigator.clipboard.writeText(text);
    copied = field;
    lastCopied = text;
    setTimeout(() => { copied = ""; }, 2000);
    if (clipboardTimer) clearTimeout(clipboardTimer);
    clipboardTimer = setTimeout(async () => {
      try {
        const current = await navigator.clipboard.readText();
        if (current === lastCopied) await navigator.clipboard.writeText("");
      } catch {
        try { await navigator.clipboard.writeText(""); } catch {}
      }
      clipboardTimer = null;
    }, CLIPBOARD_CLEAR_MS);
  }
</script>

<div class="layout">
  <aside>
    <div class="sidebar-top">
      <span class="logo">🔐 Passvault</span>
      <div class="sidebar-actions">
        <button class="icon-btn" title="Settings" onclick={() => showSettings = true}>⚙</button>
        <button class="lock-btn" onclick={lock}>Lock</button>
      </div>
    </div>
    <input placeholder="Search…" bind:value={search} />
    <div class="entry-list">
      {#each filtered as entry}
        <button
          class="entry-item"
          class:active={selected?.id === entry.id}
          onclick={() => { selected = entry; showPassword = false; }}
        >
          <span class="entry-title">{entry.title}</span>
          <span class="entry-user">{entry.username}</span>
        </button>
      {/each}
      {#if filtered.length === 0}
        <p class="empty">No entries found</p>
      {/if}
    </div>
    <button class="add-btn" onclick={() => showAdd = true}>+ Add entry</button>
    <button class="delete-account-btn" onclick={() => showDelete = true}>Delete account</button>
  </aside>

  <main>
    {#if showSettings}
      <Settings onBack={() => showSettings = false} />
    {:else if selected}
      <div class="detail">
        <div class="detail-header">
          <h2>{selected.title}</h2>
          <button class="delete-btn" onclick={() => deleteEntry(selected!.id)}>Delete</button>
        </div>
        <div class="field">
          <label>Username</label>
          <div class="field-row">
            <span>{selected.username}</span>
            <button onclick={() => copyToClipboard(selected!.username, "username")}>
              {copied === "username" ? "Copied!" : "Copy"}
            </button>
          </div>
        </div>
        <div class="field">
          <label>Password</label>
          <div class="field-row">
            <span>{showPassword ? selected.password : "••••••••"}</span>
            <button onclick={() => showPassword = !showPassword}>
              {showPassword ? "Hide" : "Show"}
            </button>
            <button onclick={() => copyToClipboard(selected!.password, "password")}>
              {copied === "password" ? "Copied!" : "Copy"}
            </button>
          </div>
        </div>
        {#if selected.url}
          <div class="field">
            <label>URL</label>
            <span>{selected.url}</span>
          </div>
        {/if}
        {#if selected.notes}
          <div class="field">
            <label>Notes</label>
            <span class="notes">{selected.notes}</span>
          </div>
        {/if}
      </div>
    {:else}
      <div class="empty-state">
        <p>Select an entry or add a new one</p>
      </div>
    {/if}
  </main>
</div>

{#if showDelete}
  <DeleteAccount
    onDeleted={() => { showDelete = false; onLock(); }}
    onCancel={() => showDelete = false}
  />
{/if}

{#if showAdd}
  <AddEntry
    onSave={async (e) => {
      const created = await invoke<Entry>("add_entry", e);
      entries = [...entries, created];
      selected = created;
      showAdd = false;
    }}
    onCancel={() => showAdd = false}
  />
{/if}

<style>
  .layout { display: flex; height: 100vh; }
  aside {
    width: 240px; min-width: 240px;
    background: #111827;
    border-right: 1px solid #1f2937;
    display: flex; flex-direction: column;
    padding: 12px;
    gap: 8px;
  }
  .sidebar-top { display: flex; align-items: center; justify-content: space-between; padding: 4px 0 8px; }
  .logo { font-weight: 700; font-size: 15px; }
  .sidebar-actions { display: flex; align-items: center; gap: 6px; }
  .icon-btn { background: transparent; color: #9ca3af; font-size: 16px; padding: 2px 6px; }
  .icon-btn:hover { color: #e2e8f0; }
  .lock-btn { background: #374151; color: #d1d5db; font-size: 12px; padding: 4px 10px; }
  .lock-btn:hover { background: #4b5563; }
  .entry-list { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; }
  .entry-item {
    background: none; color: inherit; text-align: left;
    padding: 8px 10px; border-radius: 6px;
    display: flex; flex-direction: column; gap: 2px;
    width: 100%;
  }
  .entry-item:hover { background: #1f2937; }
  .entry-item.active { background: #1e1b4b; }
  .entry-title { font-size: 14px; font-weight: 500; }
  .entry-user { font-size: 12px; color: #6b7280; }
  .empty { color: #4b5563; font-size: 13px; padding: 8px 10px; }
  .add-btn { background: #4f46e5; color: white; padding: 8px; font-size: 13px; }
  .add-btn:hover { background: #4338ca; }
  .delete-account-btn { background: #7f1d1d; color: #fca5a5; padding: 8px; font-size: 13px; }
  .delete-account-btn:hover { background: #991b1b; }

  main { flex: 1; padding: 32px; overflow-y: auto; }
  .detail { display: flex; flex-direction: column; gap: 20px; max-width: 500px; }
  .detail-header { display: flex; align-items: center; justify-content: space-between; }
  .detail-header h2 { font-size: 20px; font-weight: 600; }
  .delete-btn { background: #7f1d1d; color: #fca5a5; font-size: 13px; padding: 4px 12px; }
  .delete-btn:hover { background: #991b1b; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 12px; color: #6b7280; text-transform: uppercase; letter-spacing: 0.05em; }
  .field-row { display: flex; align-items: center; gap: 8px; }
  .field-row span { flex: 1; font-size: 15px; font-family: monospace; }
  .field-row button { background: #1f2937; color: #d1d5db; font-size: 12px; padding: 4px 10px; white-space: nowrap; }
  .field-row button:hover { background: #374151; }
  .notes { font-size: 14px; color: #9ca3af; white-space: pre-wrap; }
  .empty-state { display: flex; align-items: center; justify-content: center; height: 100%; color: #374151; }
</style>
