<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { confirmDialog } from '../stores/confirm.svelte'
  import {
    errorMessage,
    gitStatus,
    applyGitStatus,
    syncNow,
    pullOnly,
    pushOnly,
    type FileChange,
    type GitStatus,
  } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'
  import { tip } from '../utils/tooltip'
  import { discardUnder } from '../stores/tabs.svelte'
  import { openFileInTab } from '../utils/open-file'
  import FileIcon from './FileIcon.svelte'

  let commitMessage = $state('')
  let cloneUrl = $state('')
  let cloning = $state(false)

  const blocked = $derived(gitStatus.syncing || gitStatus.conflicts.length > 0)
  const conflicted = $derived(new Set(gitStatus.conflicts.map((c) => c.path)))

  async function commit() {
    if (!commitMessage.trim()) return
    if (await syncNow(commitMessage.trim())) commitMessage = ''
  }

  async function openChange(change: FileChange) {
    if (change.status === 'D') return
    try {
      await openFileInTab(change.path)
    } catch (e) {
      gitStatus.error = errorMessage(e)
    }
  }

  async function discard(change: FileChange) {
    const ok = await confirmDialog({
      title: 'Discard changes',
      message: `Discard changes to ${change.path}?`,
      kind: 'warning',
      confirmLabel: 'Discard',
    })
    if (!ok) return
    try {
      applyGitStatus(await invoke<GitStatus>('git_discard_file', { path: change.path }))
      discardUnder(change.path)
    } catch (e) {
      gitStatus.error = errorMessage(e)
    }
  }

  async function clone() {
    if (!cloneUrl.trim()) return
    cloning = true
    gitStatus.error = null
    try {
      ui.repoPath = await invoke<string>('git_clone', { url: cloneUrl.trim() })
      ui.treeVersion += 1
      cloneUrl = ''
      applyGitStatus(await invoke<GitStatus>('git_status'))
    } catch (e) {
      gitStatus.error = errorMessage(e)
    } finally {
      cloning = false
    }
  }
</script>

<div class="panel">
  {#if !gitStatus.configured}
    <form
      class="clone"
      onsubmit={(e) => {
        e.preventDefault()
        void clone()
      }}
    >
      <input
        bind:value={cloneUrl}
        aria-label="Repository URL to clone"
        placeholder="https://github.com/user/repo.git"
      />
      <button type="submit" class="commit" disabled={cloning}>
        {cloning ? 'Cloning…' : 'Clone'}
      </button>
    </form>
  {:else}
    <div class="head">
      <p class="branch" use:tip={'Current branch: ' + (gitStatus.branch || 'unknown')}>
        ⎇ {gitStatus.branch || '—'}
      </p>
      <div class="acts">
        <button disabled={blocked} aria-label="Pull" use:tip={'Pull'} onclick={() => pullOnly()}>
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 2.5v7.2m0 0 2.5-2.5M8 9.7 5.5 7.2" />
            <path d="M3.5 12.5h9" />
          </svg>
        </button>
        <button disabled={blocked} aria-label="Push" use:tip={'Push'} onclick={() => pushOnly()}>
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 10.5V3.3m0 0 2.5 2.5M8 3.3 5.5 5.8" />
            <path d="M3.5 12.5h9" />
          </svg>
        </button>
        <button
          class="sync"
          class:spinning={gitStatus.syncing}
          disabled={blocked}
          aria-label="Sync"
          use:tip={'Sync'}
          onclick={() => syncNow(null)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M13 8a5 5 0 0 1-9.2 2.7" />
            <path d="M3 8a5 5 0 0 1 9.2-2.7" />
            <path d="M12.7 2.7v2.8h-2.8" />
            <path d="M3.3 13.3v-2.8h2.8" />
          </svg>
        </button>
      </div>
    </div>

    <form
      class="commit-form"
      onsubmit={(e) => {
        e.preventDefault()
        void commit()
      }}
    >
      <textarea
        bind:value={commitMessage}
        aria-label="Commit message"
        placeholder="Commit message"
        rows="3"
      ></textarea>
      <button
        type="submit"
        class="commit"
        disabled={blocked || !commitMessage.trim()}
      >
        Commit
      </button>
    </form>

    {#if gitStatus.conflicts.length > 0}
      <p class="conflicts">⚠ {gitStatus.conflicts.length} conflict(s)</p>
    {/if}
    {#if gitStatus.error}
      <p class="error" role="alert">{gitStatus.error}</p>
    {/if}

    <div class="section">Changes {gitStatus.changes.length}</div>
    {#if gitStatus.changes.length === 0}
      <p class="none">No changes</p>
    {:else}
      <ul class="changes">
        {#each gitStatus.changes as change (change.path)}
          <li>
            <button
              class="change"
              disabled={change.status === 'D'}
              title={change.status === 'D' ? `${change.path} (deleted)` : undefined}
              use:tip={change.status === 'D' ? `${change.path} (deleted)` : change.path}
              onclick={() => openChange(change)}
            >
              <span class="badge {change.status}">{change.status}</span>
              <FileIcon name={change.path} />
              <span class="path">{change.path}</span>
            </button>
            <button
              class="discard"
              aria-label={conflicted.has(change.path)
                ? 'Resolve the conflict first'
                : 'Discard changes'}
              disabled={conflicted.has(change.path)}
              use:tip={conflicted.has(change.path)
                ? 'Resolve the conflict first'
                : 'Discard changes'}
              onclick={() => discard(change)}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M4.5 4.5l7 7m0-7-7 7" />
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    <p class="meta">
      {gitStatus.modifiedFiles.length} changed · {gitStatus.ahead} ahead · {gitStatus.behind} behind
    </p>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-2) var(--sp-2);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    min-width: 0;
  }
  .branch {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: var(--fs-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acts {
    display: flex;
    flex: 0 0 auto;
    gap: 2px;
  }
  .acts button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .acts button:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }
  .acts button:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .acts button.spinning svg {
    animation: spin 900ms linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  /* Same stroke language as FileIcon, so the panel reads as one icon set. */
  svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .commit-form,
  .clone {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }
  textarea,
  .clone input {
    box-sizing: border-box;
    width: 100%;
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: var(--text);
    font-family: inherit;
    font-size: var(--fs-sm);
  }
  textarea {
    /* Three lines: this is the panel's main input, not a filter box. */
    min-height: 64px;
    resize: none;
  }
  textarea::placeholder,
  .clone input::placeholder {
    color: var(--muted);
  }
  .commit {
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-md);
    background: var(--accent);
    color: var(--on-accent);
    font-family: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .commit:hover:not(:disabled) {
    filter: brightness(1.1);
  }
  .commit:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .section {
    padding-top: var(--sp-1);
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: var(--fs-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .none {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .changes {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .changes li {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: var(--r-sm);
  }
  .changes li:hover {
    background: var(--surface-2);
  }
  .change {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--sp-1);
    min-width: 0;
    height: 22px;
    padding: 0 var(--sp-1);
    border: 0;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }
  .change:disabled {
    color: var(--muted);
    cursor: default;
    text-decoration: line-through;
  }
  .path {
    overflow: hidden;
    font-size: var(--fs-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: 0 0 auto;
    width: 10px;
    color: var(--muted);
    font-size: var(--fs-xs);
    font-weight: 600;
  }
  .badge.M {
    color: var(--accent-fg);
  }
  .badge.A {
    color: var(--ok);
  }
  .badge.D {
    color: var(--err);
  }
  .discard {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    opacity: 0;
  }
  /* Hidden until the row is hovered, but not from the keyboard. */
  .changes li:hover .discard,
  .discard:focus-visible {
    opacity: 1;
  }
  .changes li:hover .discard:disabled {
    opacity: 0.35;
  }
  .discard:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--err);
  }
  .discard:disabled {
    cursor: default;
    opacity: 0.35;
  }
  .conflicts,
  .error {
    margin: 0;
    color: var(--err);
    font-size: var(--fs-xs);
    overflow-wrap: anywhere;
  }
  .meta {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-xs);
  }
</style>
