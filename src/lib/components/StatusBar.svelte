<script lang="ts">
  import { gitStatus, isOfflineError } from '../stores/git.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
  import { tip } from '../utils/tooltip'

  let now = $state(Date.now())

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000)
    return () => clearInterval(timer)
  })

  const active = $derived(tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null)
  const badge = $derived.by(() => {
    if (gitStatus.error) return isOfflineError(gitStatus.error) ? 'offline' : 'error'
    if (!gitStatus.configured) return 'no repo'
    if (gitStatus.syncing) return 'syncing'
    if (gitStatus.conflicts.length > 0) return 'conflict'
    if (gitStatus.diverged) return 'diverged'
    if (!gitStatus.hasRemote) return 'local only'
    if (gitStatus.ahead > 0) return 'queued'
    return 'synced'
  })

  function relative(date: Date | null): string {
    if (!date) return 'never'
    const seconds = Math.floor((now - date.getTime()) / 1000)
    if (seconds < 60) return 'just now'
    const minutes = Math.floor(seconds / 60)
    if (minutes < 60) return `${minutes} min ago`
    const hours = Math.floor(minutes / 60)
    if (hours < 24) return `${hours} h ago`
    return `${Math.floor(hours / 24)} d ago`
  }
</script>

<footer class="statusbar">
  <span class="path">{active?.path ?? 'No file'}</span>
  <span class="spacer"></span>
  {#if gitStatus.syncing || ui.importing || ui.deleting}
    <svg
      class="spinner"
      viewBox="25 25 50 50"
      role="img"
      aria-label={ui.deleting
        ? `Deleting ${ui.deleting}`
        : ui.importCommitting
          ? 'Committing import'
          : ui.importing
            ? 'Importing'
            : 'Syncing'}
    >
      <circle cx="50" cy="50" r="20"></circle>
    </svg>
  {/if}
  {#if ui.deleting}
    <span>deleting…</span>
  {:else if ui.importCommitting}
    <span class="pct">committing…</span>
  {:else if ui.importPct !== null}
    <span class="pct">{ui.importPct}%</span>
  {/if}
  {#if badge === 'conflict'}
    <button class="badge conflict" use:tip={'Open merge conflicts'} onclick={() => (ui.conflictsHidden = false)}>
      {badge}
    </button>
  {:else}
    <span class="badge {badge}">{badge}</span>
  {/if}
  <span>last push: {relative(gitStatus.lastPush)}</span>
  <span>{ui.cursor.line}:{ui.cursor.column}</span>
  <span>{active?.language ?? 'plaintext'}</span>
</footer>

<style>
  .statusbar {
    display: flex;
    flex: 0 0 var(--h-statusbar);
    align-items: center;
    gap: var(--sp-3);
    padding: 0 var(--sp-25);
    background: var(--surface-1);
    color: var(--muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spacer {
    flex: 1;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    padding: 0 var(--sp-1);
    border-radius: var(--r-pill);
    background: var(--surface-1);
    color: var(--ok);
  }
  .badge.queued,
  .badge.syncing {
    color: var(--warn);
  }
  .badge.diverged,
  .badge.conflict,
  .badge.offline {
    color: var(--err);
  }
  .badge.no {
    color: var(--muted);
  }
  .badge.local {
    color: var(--muted);
  }
  .spinner {
    flex: 0 0 auto;
    width: 12px;
    height: 12px;
    fill: none;
    stroke: var(--accent-fg);
    transform-origin: center;
    animation: spin 2s linear infinite;
  }
  .spinner circle {
    stroke-width: 6;
    stroke-linecap: round;
    stroke-dasharray: 1, 200;
    stroke-dashoffset: 0;
    animation: dash 1.5s ease-in-out infinite;
  }
  @keyframes dash {
    0% {
      stroke-dasharray: 1, 200;
      stroke-dashoffset: 0;
    }
    50% {
      stroke-dasharray: 90, 200;
      stroke-dashoffset: -35px;
    }
    100% {
      stroke-dashoffset: -125px;
    }
  }
  .pct {
    min-width: 3ch;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }</style>
