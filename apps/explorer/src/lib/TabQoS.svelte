<script lang="ts">
  import type { EndpointDto, RegistrySnapshot } from "./types";
  import {
    fmtNs,
    matchQos,
    parseQos,
    type MatchResult,
    type NormalizedQos,
  } from "./qos-matching";

  type Props = { topicName: string; registry: RegistrySnapshot };
  let { topicName, registry }: Props = $props();

  let writers = $derived(
    registry.writers.filter((w) => w.topic_name === topicName),
  );
  let readers = $derived(
    registry.readers.filter((r) => r.topic_name === topicName),
  );

  type NormalizedEndpoint = { ep: EndpointDto; qos: NormalizedQos };

  let normWriters = $derived<NormalizedEndpoint[]>(
    writers.map((w) => ({ ep: w, qos: parseQos(w.qos) })),
  );
  let normReaders = $derived<NormalizedEndpoint[]>(
    readers.map((r) => ({ ep: r, qos: parseQos(r.qos) })),
  );

  // Matrix: matrix[wi][ri] = MatchResult for writers[wi] vs readers[ri]
  let matrix = $derived<MatchResult[][]>(
    normWriters.map((w) => normReaders.map((r) => matchQos(w.qos, r.qos))),
  );

  let totalMatches = $derived(
    matrix.flat().filter((m) => m.ok).length,
  );
  let totalPairs = $derived(normWriters.length * normReaders.length);

  function shortGuid(g: string): string {
    // Show the entity portion (after the dot) — that's the unique tail
    // inside a participant.
    const dot = g.indexOf(".");
    return dot >= 0 ? g.slice(dot + 1) : g.slice(0, 8);
  }

  function participantLabel(participantGuid: string): string {
    const p = registry.participants.find((p) => p.guid === participantGuid);
    return p?.entity_name ?? participantGuid.slice(0, 8);
  }

  // Selected cell for the detail explainer below the matrix.
  let selected = $state<{ wi: number; ri: number } | null>(null);
</script>

<div class="qos">
  {#if normWriters.length === 0 || normReaders.length === 0}
    <p class="muted small empty">
      Need at least one writer and one reader on this topic to compute
      matches. Currently: {normWriters.length} writer{normWriters.length === 1
        ? ""
        : "s"}, {normReaders.length} reader{normReaders.length === 1 ? "" : "s"}.
    </p>
  {:else}
    <header class="summary">
      <span class="badge ok">{totalMatches}</span> /
      <span class="badge total">{totalPairs}</span>
      <span class="muted small">writer × reader pairs match</span>
    </header>

    <div class="matrix-wrap">
      <table class="matrix">
        <thead>
          <tr>
            <th></th>
            {#each normReaders as r (r.ep.guid)}
              <th class="col-head" title={r.ep.guid}>
                <div class="col-head-inner">
                  <code>{shortGuid(r.ep.guid)}</code>
                  <span class="muted small">
                    {participantLabel(r.ep.participant_guid)}
                  </span>
                </div>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each normWriters as w, wi (w.ep.guid)}
            <tr>
              <th class="row-head" title={w.ep.guid}>
                <div class="row-head-inner">
                  <code>{shortGuid(w.ep.guid)}</code>
                  <span class="muted small">
                    {participantLabel(w.ep.participant_guid)}
                  </span>
                </div>
              </th>
              {#each normReaders as r, ri (r.ep.guid)}
                {@const m = matrix[wi][ri]}
                <td>
                  <button
                    class:ok={m.ok}
                    class:bad={!m.ok}
                    class:active={selected?.wi === wi && selected?.ri === ri}
                    onclick={() => (selected = { wi, ri })}
                    title={m.ok
                      ? "Compatible"
                      : m.issues.map((i) => `${i.policy}: ${i.reason}`).join("\n")}
                  >
                    {#if m.ok}
                      ✓
                    {:else}
                      <span class="x">✕</span>
                      <span class="reason-list">
                        {#each m.issues.slice(0, 2) as iss}
                          <span class="reason-chip">{iss.policy}</span>
                        {/each}
                      </span>
                    {/if}
                  </button>
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if selected}
      {@const w = normWriters[selected.wi]}
      {@const r = normReaders[selected.ri]}
      {@const m = matrix[selected.wi][selected.ri]}
      <section class="detail">
        <header>
          <strong>
            Writer <code>{shortGuid(w.ep.guid)}</code>
            ↔ Reader <code>{shortGuid(r.ep.guid)}</code>
          </strong>
          <span class:ok={m.ok} class:bad={!m.ok}>
            {m.ok ? "✓ Compatible" : `✕ ${m.issues.length} issue(s)`}
          </span>
        </header>

        {#if !m.ok}
          <ul class="issues">
            {#each m.issues as iss}
              <li>
                <strong>{iss.policy}</strong> — {iss.reason}
              </li>
            {/each}
          </ul>
        {/if}

        <div class="side-by-side">
          {@render qosTable("Writer", w.qos)}
          {@render qosTable("Reader", r.qos)}
        </div>
      </section>
    {:else}
      <p class="muted small hint">
        Click a cell to see the writer ↔ reader QoS side by side.
      </p>
    {/if}
  {/if}
</div>

{#snippet qosTable(label: string, q: NormalizedQos)}
  <table class="qos-table">
    <caption>{label}</caption>
    <tbody>
      <tr>
        <th>Reliability</th>
        <td>{q.reliability}</td>
      </tr>
      <tr>
        <th>Durability</th>
        <td>{q.durability}</td>
      </tr>
      <tr>
        <th>History</th>
        <td>
          {q.history.kind}{q.history.depth != null
            ? `(${q.history.depth})`
            : ""}
        </td>
      </tr>
      <tr>
        <th>Deadline</th>
        <td>{fmtNs(q.deadline_ns)}</td>
      </tr>
      <tr>
        <th>Liveliness</th>
        <td>
          {#if q.liveliness}
            {q.liveliness.kind} / {fmtNs(q.liveliness.lease_duration_ns)}
          {:else}
            —
          {/if}
        </td>
      </tr>
      <tr>
        <th>Ownership</th>
        <td>{q.ownership}</td>
      </tr>
    </tbody>
  </table>
{/snippet}

<style>
  .qos {
    padding: 0.8rem 1rem;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .empty {
    padding: 2rem 0;
    text-align: center;
  }
  .summary {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    font-size: 0.95rem;
  }
  .badge {
    display: inline-block;
    padding: 0.1em 0.55em;
    border-radius: 4px;
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.92em;
  }
  .badge.ok {
    background: #d4edda;
    color: #1e6e3e;
  }
  .badge.total {
    background: #eee;
    color: #555;
  }

  .matrix-wrap {
    overflow-x: auto;
  }
  table.matrix {
    border-collapse: separate;
    border-spacing: 4px;
  }
  table.matrix th {
    font-weight: normal;
    text-align: left;
    padding: 0.3rem 0.45rem;
    vertical-align: bottom;
  }
  .col-head {
    text-align: center !important;
    vertical-align: bottom;
  }
  .col-head-inner,
  .row-head-inner {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .col-head-inner code,
  .row-head-inner code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78em;
  }
  table.matrix td {
    padding: 0;
  }
  table.matrix td button {
    width: 100%;
    min-width: 64px;
    min-height: 44px;
    padding: 0.3rem 0.5rem;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.2rem;
  }
  table.matrix td button.ok {
    background: #d4edda;
    color: #1e6e3e;
    border-color: #b4dabd;
    font-size: 1.1rem;
  }
  table.matrix td button.bad {
    background: #f8d7da;
    color: #842029;
    border-color: #f1aeb5;
  }
  table.matrix td button.active {
    box-shadow: 0 0 0 2px rgba(85, 119, 204, 0.5);
  }
  .x {
    font-size: 1rem;
  }
  .reason-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.15rem;
    justify-content: center;
  }
  .reason-chip {
    background: rgba(132, 32, 41, 0.15);
    border-radius: 3px;
    padding: 0 0.35em;
    font-size: 0.7rem;
    font-weight: 500;
  }

  .detail {
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 6px;
    padding: 0.8rem 1rem;
  }
  .detail header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .detail header .ok {
    color: #1e6e3e;
    font-weight: 600;
  }
  .detail header .bad {
    color: #842029;
    font-weight: 600;
  }
  .issues {
    margin: 0 0 0.8rem;
    padding-left: 1.2rem;
    color: #842029;
  }
  .issues li {
    margin: 0.25rem 0;
  }
  .side-by-side {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }
  table.qos-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  table.qos-table caption {
    text-align: left;
    font-weight: 600;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
    padding-bottom: 0.3rem;
  }
  table.qos-table th,
  table.qos-table td {
    text-align: left;
    padding: 0.25rem 0.45rem;
    border-bottom: 1px solid #eee;
  }
  table.qos-table th {
    color: #555;
    font-weight: 500;
    width: 8em;
  }
  table.qos-table td {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .muted {
    color: #777;
  }
  .small {
    font-size: 0.85em;
  }
  .hint {
    padding: 0.5rem 0;
  }

  @media (prefers-color-scheme: dark) {
    .badge.ok {
      background: #1d3a26;
      color: #8de8a8;
    }
    .badge.total {
      background: #2a2a2a;
      color: #ccc;
    }
    table.matrix td button.ok {
      background: #1d3a26;
      color: #8de8a8;
      border-color: #2a5a38;
    }
    table.matrix td button.bad {
      background: #3a1d22;
      color: #f4a8b1;
      border-color: #5a2a32;
    }
    table.matrix td button.active {
      box-shadow: 0 0 0 2px rgba(91, 123, 214, 0.6);
    }
    .reason-chip {
      background: rgba(244, 168, 177, 0.15);
    }
    .detail {
      background: #1f1f1f;
      border-color: #333;
    }
    .detail header .ok {
      color: #8de8a8;
    }
    .detail header .bad {
      color: #f4a8b1;
    }
    .issues {
      color: #f4a8b1;
    }
    table.qos-table caption,
    table.qos-table th {
      color: #aaa;
    }
    table.qos-table th,
    table.qos-table td {
      border-bottom-color: #2a2a2a;
    }
  }
</style>
