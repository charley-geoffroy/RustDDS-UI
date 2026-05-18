<script lang="ts">
  import type { Lang } from "../docs/chapters";
  import type {
    Durability,
    Reliability,
  } from "../qos-matching";
  import type { WorkflowEndpoint } from "./workflow-scenario";
  import type { EndpointPair } from "./workflow-state";

  type Props = {
    endpoints: WorkflowEndpoint[];
    pairs: EndpointPair[];
    lang: Lang;
    onQosChange: (
      epId: string,
      update: Partial<{
        reliability: Reliability;
        durability: Durability;
        historyDepth: number | null;
      }>,
    ) => void;
    onReset: () => void;
  };

  let { endpoints, pairs, lang, onQosChange, onReset }: Props = $props();

  function tr(en: string, fr: string): string {
    return lang === "fr" ? fr : en;
  }

  const reliabilityOptions: Reliability[] = ["BestEffort", "Reliable"];
  const durabilityOptions: Durability[] = [
    "Volatile",
    "TransientLocal",
    "Transient",
    "Persistent",
  ];
  const historyDepthOptions: (number | null)[] = [1, 10, 100, null];

  function depthLabel(d: number | null): string {
    return d == null ? "KeepAll" : `KeepLast(${d})`;
  }
</script>

<section class="tinker">
  <header>
    <div class="title-block">
      <h4>{tr("QoS tinker", "QoS tinker")}</h4>
      <p class="hint">
        {tr(
          "Each endpoint carries its own QoS. Matching is evaluated for every (writer, reader) pair sharing a topic — change any cell below to see the green edges turn red.",
          "Chaque endpoint porte sa propre QoS. Le matching est évalué pour chaque paire (writer, reader) partageant un topic — modifie n'importe quelle cellule pour voir les edges verts passer au rouge.",
        )}
      </p>
    </div>
    <button class="reset" onclick={onReset}>
      {tr("Reset", "Réinitialiser")}
    </button>
  </header>

  <table>
    <thead>
      <tr>
        <th>{tr("Endpoint", "Endpoint")}</th>
        <th>Reliability</th>
        <th>Durability</th>
        <th>History</th>
      </tr>
    </thead>
    <tbody>
      {#each endpoints as ep (ep.id)}
        <tr>
          <td>
            <strong>{ep.ownerId}</strong>
            <span class="muted">
              {ep.kind === "writer" ? "↑" : "↓"} {ep.kind}
            </span>
          </td>
          <td>
            <select
              value={ep.qos.reliability}
              onchange={(e) =>
                onQosChange(ep.id, {
                  reliability: (e.target as HTMLSelectElement)
                    .value as Reliability,
                })}
            >
              {#each reliabilityOptions as r (r)}
                <option value={r}>{r}</option>
              {/each}
            </select>
          </td>
          <td>
            <select
              value={ep.qos.durability}
              onchange={(e) =>
                onQosChange(ep.id, {
                  durability: (e.target as HTMLSelectElement)
                    .value as Durability,
                })}
            >
              {#each durabilityOptions as d (d)}
                <option value={d}>{d}</option>
              {/each}
            </select>
          </td>
          <td>
            <select
              value={ep.qos.history.depth == null ? "all" : String(ep.qos.history.depth)}
              onchange={(e) => {
                const v = (e.target as HTMLSelectElement).value;
                onQosChange(ep.id, {
                  historyDepth: v === "all" ? null : Number(v),
                });
              }}
            >
              {#each historyDepthOptions as d (d ?? "all")}
                <option value={d == null ? "all" : String(d)}>
                  {depthLabel(d)}
                </option>
              {/each}
            </select>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="matches">
    <strong>{tr("Matches", "Matches")}:</strong>
    {#each pairs.filter((p) => p.sameTopic) as p (p.writer.id + p.reader.id)}
      <span class="match-pill" class:ok={p.match.ok} class:bad={!p.match.ok}>
        {p.match.ok ? "✓" : "✕"}
        {p.writer.ownerId} → {p.reader.ownerId}
        {#if !p.match.ok && p.match.issues.length > 0}
          <em>· {p.match.issues[0].policy}</em>
        {/if}
      </span>
    {/each}
  </div>
</section>

<style>
  .tinker {
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.8rem;
    margin-bottom: 0.5rem;
  }
  .title-block {
    flex: 1;
  }
  h4 {
    margin: 0;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
    font-weight: 600;
  }
  .hint {
    margin: 0.2rem 0 0;
    font-size: 0.78rem;
    color: #777;
    line-height: 1.4;
    max-width: 65ch;
  }
  .reset {
    background: none;
    border: 1px solid #d8dbe1;
    border-radius: 4px;
    font: inherit;
    font-size: 0.75rem;
    padding: 0.1rem 0.55rem;
    cursor: pointer;
    color: #555;
  }
  .reset:hover {
    border-color: #5577cc;
    color: #2c4f9c;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
    margin-bottom: 0.6rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.25rem 0.4rem;
    border-bottom: 1px solid #eef0f4;
  }
  th {
    color: #555;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    font-size: 0.7rem;
  }
  select {
    font: inherit;
    font-size: 0.8rem;
    padding: 0.1rem 0.3rem;
    border: 1px solid #d8dbe1;
    border-radius: 4px;
    background: #fff;
    color: inherit;
  }
  .muted {
    color: #888;
    margin-left: 0.4rem;
    font-size: 0.78rem;
  }
  .matches {
    font-size: 0.82rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: baseline;
  }
  .match-pill {
    display: inline-flex;
    align-items: baseline;
    gap: 0.25rem;
    padding: 0.1rem 0.55rem;
    border-radius: 999px;
    border: 1px solid transparent;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78rem;
  }
  .match-pill.ok {
    background: #d4edda;
    border-color: #b4dabd;
    color: #1e6e3e;
  }
  .match-pill.bad {
    background: #f8d7da;
    border-color: #f1aeb5;
    color: #842029;
  }
  .match-pill em {
    font-style: normal;
    opacity: 0.7;
    font-size: 0.7rem;
  }

  @media (prefers-color-scheme: dark) {
    .tinker {
      background: #1f1f1f;
      border-color: #333;
    }
    th,
    td {
      border-bottom-color: #2a2a2a;
    }
    h4,
    th {
      color: #aaa;
    }
    .hint {
      color: #999;
    }
    select {
      background: #161616;
      border-color: #353d4a;
      color: #ddd;
    }
    .reset {
      border-color: #353d4a;
      color: #ccc;
    }
    .muted {
      color: #999;
    }
    .match-pill.ok {
      background: #1d3a26;
      border-color: #2a5a38;
      color: #8de8a8;
    }
    .match-pill.bad {
      background: #3a1d22;
      border-color: #5a2a32;
      color: #f4a8b1;
    }
  }
</style>
