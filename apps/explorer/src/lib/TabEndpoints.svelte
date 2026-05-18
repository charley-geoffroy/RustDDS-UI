<script lang="ts">
  import type { RegistrySnapshot, EndpointDto } from "./types";

  type Props = { topicName: string; registry: RegistrySnapshot };
  let { topicName, registry }: Props = $props();

  let writers = $derived(
    registry.writers.filter((w) => w.topic_name === topicName),
  );
  let readers = $derived(
    registry.readers.filter((r) => r.topic_name === topicName),
  );

  function participantLabel(guid: string): string {
    const p = registry.participants.find((p) => p.guid === guid);
    if (p) {
      return p.entity_name ?? `(${guid.slice(0, 8)})`;
    }
    return `unknown (${guid.slice(0, 8)})`;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (_) {
      /* clipboard blocked */
    }
  }
</script>

<div class="endpoints">
  <section>
    <h3>Writers <span class="count">{writers.length}</span></h3>
    {#if writers.length === 0}
      <p class="muted small">No writers detected on this topic.</p>
    {/if}
    {#each writers as w (w.guid)}
      <article class="ep">
        <div class="row">
          <strong>{participantLabel(w.participant_guid)}</strong>
        </div>
        <div class="row">
          <button
            class="guid"
            onclick={() => copy(w.guid)}
            title="Click to copy"
          >
            {w.guid}
          </button>
        </div>
      </article>
    {/each}
  </section>

  <section>
    <h3>Readers <span class="count">{readers.length}</span></h3>
    {#if readers.length === 0}
      <p class="muted small">No readers detected on this topic.</p>
    {/if}
    {#each readers as r (r.guid)}
      <article class="ep">
        <div class="row">
          <strong>{participantLabel(r.participant_guid)}</strong>
        </div>
        <div class="row">
          <button
            class="guid"
            onclick={() => copy(r.guid)}
            title="Click to copy"
          >
            {r.guid}
          </button>
        </div>
      </article>
    {/each}
  </section>
</div>

<style>
  .endpoints {
    padding: 0.8rem 1rem;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    overflow: auto;
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
    font-weight: 600;
  }
  .count {
    display: inline-block;
    min-width: 1.4em;
    padding: 0 0.4em;
    margin-left: 0.3em;
    border-radius: 999px;
    background: #eee;
    color: #444;
    font-size: 0.78em;
    text-align: center;
    font-weight: 500;
  }
  .ep {
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 5px;
    padding: 0.5rem 0.7rem;
    margin-bottom: 0.5rem;
  }
  .row + .row {
    margin-top: 0.2rem;
  }
  .guid {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78em;
    color: #777;
    background: none;
    border: none;
    padding: 0;
    cursor: copy;
    text-align: left;
  }
  .guid:hover {
    color: #2c4f9c;
  }
  .muted {
    color: #888;
  }
  .small {
    font-size: 0.85em;
  }

  @media (prefers-color-scheme: dark) {
    h3 {
      color: #aaa;
    }
    .count {
      background: #333;
      color: #ccc;
    }
    .ep {
      background: #1f1f1f;
      border-color: #333;
    }
    .guid {
      color: #888;
    }
    .guid:hover {
      color: #a8c0ff;
    }
  }
</style>
