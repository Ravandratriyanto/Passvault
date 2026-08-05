<script lang="ts">
  let {
    onSave,
    onCancel,
  }: {
    onSave: (e: {
      title: string; username: string; password: string;
      url: string | null; notes: string | null;
      category: string | null; totp_secret: string | null;
    }) => void;
    onCancel: () => void;
  } = $props();

  let title = $state("");
  let username = $state("");
  let password = $state("");
  let url = $state("");
  let notes = $state("");

  function submit(e: Event) {
    e.preventDefault();
    if (!title || !username || !password) return;
    onSave({
      title,
      username,
      password,
      url: url || null,
      notes: notes || null,
      category: null,
      totp_secret: null,
    });
  }
</script>

<div class="overlay">
  <div class="modal">
    <h2>Add entry</h2>
    <form onsubmit={submit}>
      <label>Title *
        <input bind:value={title} placeholder="e.g. GitHub" />
      </label>
      <label>Username *
        <input bind:value={username} placeholder="email or username" />
      </label>
      <label>Password *
        <input type="password" bind:value={password} placeholder="password" />
      </label>
      <label>URL
        <input bind:value={url} placeholder="https://..." />
      </label>
      <label>Notes
        <textarea bind:value={notes} placeholder="optional notes" rows="3"></textarea>
      </label>
      <div class="actions">
        <button type="button" class="cancel" onclick={onCancel}>Cancel</button>
        <button type="submit">Save</button>
      </div>
    </form>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 10;
  }
  .modal {
    background: #1a202c;
    border: 1px solid #2d3748;
    border-radius: 12px;
    padding: 32px;
    width: 420px;
    display: flex; flex-direction: column; gap: 20px;
  }
  h2 { font-size: 18px; font-weight: 600; }
  form { display: flex; flex-direction: column; gap: 14px; }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; color: #a0aec0; }
  textarea {
    background: #1a202c; color: #e2e8f0;
    border: 1px solid #2d3748; border-radius: 6px;
    padding: 8px 12px; font-size: 14px;
    resize: vertical; outline: none;
  }
  textarea:focus { border-color: #4f46e5; }
  .actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 4px; }
  button[type="submit"] { background: #4f46e5; color: white; padding: 8px 20px; }
  button[type="submit"]:hover { background: #4338ca; }
  .cancel { background: #374151; color: #d1d5db; padding: 8px 20px; }
  .cancel:hover { background: #4b5563; }
</style>
