<script lang="ts">
  type Props = { userTopicCount: number };
  let { userTopicCount }: Props = $props();

  let copied = $state(false);
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch (_) {
      /* clipboard blocked */
    }
  }
</script>

<div class="empty">
  <div class="icon" aria-hidden="true">&#128225;</div>
  <h2>Pick a topic to inspect</h2>

  {#if userTopicCount === 0}
    <p class="lead">No user topics discovered yet on this domain.</p>
    <p>Try running a publisher in another terminal:</p>
    <div class="snippet">
      <code>cargo run -p pub-rustdds</code>
      <button
        onclick={() => copy("cargo run -p pub-rustdds")}
        title="Copy to clipboard"
      >
        {copied ? "copied!" : "copy"}
      </button>
    </div>
    <p class="muted small">
      Discovery typically takes ~2 s. System topics (SPDP/SEDP) are visible
      under "System" in the sidebar.
    </p>
  {:else}
    <p class="lead">
      Select one of the {userTopicCount} user topic{userTopicCount > 1 ? "s" : ""}
      in the sidebar.
    </p>
    <p class="muted small">
      <kbd>/</kbd> to search · <kbd>Esc</kbd> to close detail view
    </p>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    padding: 2rem;
    color: #555;
  }
  .icon {
    font-size: 2.5rem;
    margin-bottom: 0.6rem;
    opacity: 0.7;
  }
  h2 {
    margin: 0 0 1rem;
    font-weight: 500;
    font-size: 1.15rem;
  }
  .lead {
    max-width: 36ch;
    line-height: 1.5;
  }
  .snippet {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: #f3f4f7;
    border: 1px solid #e3e4ea;
    border-radius: 5px;
    padding: 0.4rem 0.6rem;
    margin: 0.6rem 0 1rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
  }
  .snippet button {
    background: none;
    border: 1px solid #c8cad2;
    border-radius: 3px;
    padding: 0.1em 0.6em;
    cursor: pointer;
    font-size: 0.82em;
    color: #555;
  }
  .snippet button:hover {
    border-color: #5577cc;
    color: #2c4f9c;
  }
  .muted {
    color: #888;
  }
  .small {
    font-size: 0.85em;
  }
  kbd {
    background: #eef0f4;
    border: 1px solid #d8dbe1;
    border-bottom-width: 2px;
    border-radius: 3px;
    padding: 0 0.4em;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    color: #444;
  }

  @media (prefers-color-scheme: dark) {
    .empty {
      color: #ccc;
    }
    .snippet {
      background: #1a1f28;
      border-color: #2c3340;
    }
    .snippet button {
      border-color: #3d4a63;
      color: #ccc;
    }
    .snippet button:hover {
      border-color: #6e8cd6;
      color: #a8c0ff;
    }
    kbd {
      background: #1f242c;
      border-color: #353d4a;
      color: #ccc;
    }
    .muted {
      color: #999;
    }
  }
</style>
