<script lang="ts">
  import { gitStatus } from '../stores/git.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'

  let now = $state(Date.now())

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000)
    return () => clearInterval(timer)
  })

  const active = $derived(tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null)
  const badge = $derived.by(() => {
    if (gitStatus.error) return 'offline'
    if (!gitStatus.configured) return 'no repo'
    if (gitStatus.syncing) return 'syncing'
    if (gitStatus.conflicts.length > 0) return 'conflict'
    if (gitStatus.diverged) return 'diverged'
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
  {#if badge === 'conflict'}
    <button class="badge conflict" title="Open merge conflicts" onclick={() => (ui.conflictsHidden = false)}>
      {badge}
    </button>
  {:else}
    <span class="badge {badge}">{badge}</span>
  {/if}
  <span>last push: {relative(gitStatus.lastPush)}</span>
  <span>{ui.cursor.line}:{ui.cursor.column}</span>
  <span>{active?.language ?? 'plaintext'}</span>
  <button class="gear" aria-label="Settings" title="Settings" onclick={() => (ui.settingsOpen = true)}>
    ⚙
  </button>
</footer>

<style>
  .statusbar {
    display: flex;
    flex: 0 0 24px;
    align-items: center;
    gap: 14px;
    padding: 0 10px;
    background: var(--bg-raised);
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 11px;
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
    padding: 0 6px;
    border-radius: 8px;
    background: var(--bg-panel);
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
  .gear {
    padding: 0 4px;
    color: var(--muted);
  }
  .gear:hover {
    color: var(--text);
  }
</style>
