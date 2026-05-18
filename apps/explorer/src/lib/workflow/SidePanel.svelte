<script lang="ts">
  import type { Lang } from "../docs/chapters";
  import type { EndpointPair } from "./workflow-state";
  import type { WorkflowEndpoint } from "./workflow-scenario";
  import {
    PARTICIPANTS,
    SCENARIO_DOMAIN_ID,
    SCENARIO_TOPIC,
    SCENARIO_TYPE,
    participant,
    endpointsOf,
  } from "./workflow-scenario";
  import { fmtNs } from "../qos-matching";

  type Selection =
    | { kind: "none" }
    | { kind: "participant"; id: string }
    | { kind: "endpoint"; id: string }
    | { kind: "bus" }
    | { kind: "pair"; pair: EndpointPair };

  type Props = {
    selection: Selection;
    endpoints: WorkflowEndpoint[];
    lang: Lang;
  };

  let { selection, endpoints, lang }: Props = $props();

  function tr(en: string, fr: string): string {
    return lang === "fr" ? fr : en;
  }

  function endpoint(id: string): WorkflowEndpoint | undefined {
    return endpoints.find((e) => e.id === id);
  }

  const portFormula = `7400 + 250 × ${SCENARIO_DOMAIN_ID} = ${7400 + 250 * SCENARIO_DOMAIN_ID}`;
</script>

<aside class="side">
  <header>
    <h4>{tr("Inspector", "Inspecteur")}</h4>
  </header>
  <div class="body">
    {#if selection.kind === "none"}
      <p class="muted">
        {tr(
          "Click a participant, endpoint, bus, or matched edge for details.",
          "Clique sur un participant, un endpoint, le bus ou un edge matché pour les détails.",
        )}
      </p>
    {:else if selection.kind === "bus"}
      <h5>{tr("SPDP multicast group", "Groupe multicast SPDP")}</h5>
      <table>
        <tbody>
          <tr><th>Domain</th><td>{SCENARIO_DOMAIN_ID}</td></tr>
          <tr><th>Group</th><td>239.255.0.1</td></tr>
          <tr><th>Port</th><td>{portFormula}</td></tr>
          <tr><th>Transport</th><td>UDP / multicast</td></tr>
        </tbody>
      </table>
      <p class="muted small">
        {tr(
          "All participants on the same domain listen here. Each emits a self-announcement every ~5 s.",
          "Tous les participants du même domain écoutent ici. Chacun émet une auto-annonce toutes les ~5 s.",
        )}
        <a href="#docs/discovery">📖 {tr("Discovery chapter", "Chapitre Discovery")}</a>
      </p>
    {:else if selection.kind === "participant"}
      {@const p = participant(selection.id as any)}
      {@const eps = endpointsOf(p.id)}
      <h5>{p.name}</h5>
      <table>
        <tbody>
          <tr><th>Vendor</th><td><code>{p.vendor}</code></td></tr>
          <tr>
            <th>GUID prefix</th>
            <td><code>{p.guidPrefix}</code></td>
          </tr>
          <tr><th>Lease</th><td>{p.leaseSeconds} s</td></tr>
          <tr>
            <th>{tr("Endpoints", "Endpoints")}</th>
            <td>{eps.length}</td>
          </tr>
        </tbody>
      </table>
      <p class="muted small">
        {tr(
          "Owns the endpoints below. The GUID prefix is shared by every entity in this process.",
          "Possède les endpoints ci-dessous. Le préfixe GUID est partagé par toutes les entités de ce process.",
        )}
        <a href="#docs/endpoints">📖 {tr("Endpoints chapter", "Chapitre Endpoints")}</a>
      </p>
    {:else if selection.kind === "endpoint"}
      {@const ep = endpoint(selection.id)}
      {#if ep}
        <h5>
          {ep.kind === "writer" ? "↑" : "↓"}
          {ep.kind}
        </h5>
        <table>
          <tbody>
            <tr><th>Topic</th><td><code>{ep.topic}</code></td></tr>
            <tr><th>Type</th><td><code>{ep.typeName}</code></td></tr>
            <tr><th>Owner</th><td>{ep.ownerId}</td></tr>
            <tr><th>Reliability</th><td>{ep.qos.reliability}</td></tr>
            <tr><th>Durability</th><td>{ep.qos.durability}</td></tr>
            <tr>
              <th>History</th>
              <td>
                {ep.qos.history.kind}{ep.qos.history.depth != null
                  ? `(${ep.qos.history.depth})`
                  : ""}
              </td>
            </tr>
            <tr><th>Deadline</th><td>{fmtNs(ep.qos.deadline_ns)}</td></tr>
          </tbody>
        </table>
        <p class="muted small">
          {tr(
            "Tweak this endpoint's QoS in the panel below to see matching break or recover.",
            "Modifie la QoS de cet endpoint dans le panneau ci-dessous pour voir le matching casser ou revenir.",
          )}
          <a href="#docs/qos">📖 {tr("QoS chapter", "Chapitre QoS")}</a>
        </p>
      {/if}
    {:else if selection.kind === "pair"}
      {@const pair = selection.pair}
      <h5>
        {pair.writer.ownerId} → {pair.reader.ownerId}
        <span class={pair.match.ok ? "ok" : "bad"}>
          {pair.match.ok ? "✓" : "✕"}
        </span>
      </h5>
      {#if pair.match.ok}
        <p>
          {tr(
            "Pair compatible — samples flow from writer to reader.",
            "Paire compatible — les samples circulent du writer au reader.",
          )}
        </p>
      {:else}
        <ul class="issues">
          {#each pair.match.issues as iss}
            <li>
              <strong>{iss.policy}</strong> — {iss.reason}
            </li>
          {/each}
        </ul>
      {/if}
      <table>
        <thead>
          <tr>
            <th></th>
            <th>{tr("Writer", "Writer")}</th>
            <th>{tr("Reader", "Reader")}</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th>Reliability</th>
            <td>{pair.writer.qos.reliability}</td>
            <td>{pair.reader.qos.reliability}</td>
          </tr>
          <tr>
            <th>Durability</th>
            <td>{pair.writer.qos.durability}</td>
            <td>{pair.reader.qos.durability}</td>
          </tr>
          <tr>
            <th>History</th>
            <td>
              {pair.writer.qos.history.kind}{pair.writer.qos.history.depth !=
              null
                ? `(${pair.writer.qos.history.depth})`
                : ""}
            </td>
            <td>
              {pair.reader.qos.history.kind}{pair.reader.qos.history.depth !=
              null
                ? `(${pair.reader.qos.history.depth})`
                : ""}
            </td>
          </tr>
        </tbody>
      </table>
      <p class="muted small">
        <a href="#docs/qos">📖 {tr("QoS matching rules", "Règles de matching QoS")}</a>
      </p>
    {/if}
  </div>
</aside>

<style>
  .side {
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 6px;
    width: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    padding: 0.4rem 0.7rem;
    border-bottom: 1px solid #eee;
    background: #fafbfc;
  }
  h4 {
    margin: 0;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
    font-weight: 600;
  }
  .body {
    padding: 0.6rem 0.8rem;
    font-size: 0.85rem;
    overflow-y: auto;
    max-height: 280px;
  }
  h5 {
    margin: 0 0 0.5rem;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .ok {
    color: #1e6e3e;
  }
  .bad {
    color: #842029;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: 0.5rem 0;
    font-size: 0.82rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.2rem 0.35rem;
    border-bottom: 1px solid #eee;
    vertical-align: top;
  }
  th {
    color: #555;
    font-weight: 500;
    width: 7em;
  }
  td code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
  }
  .issues {
    margin: 0 0 0.5rem;
    padding-left: 1.2rem;
    color: #842029;
    font-size: 0.82rem;
  }
  .muted {
    color: #777;
  }
  .small {
    font-size: 0.85em;
  }
  a {
    color: #2c4f9c;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  @media (prefers-color-scheme: dark) {
    .side {
      background: #1f1f1f;
      border-color: #333;
    }
    header {
      background: #1a1d22;
      border-bottom-color: #2a2a2a;
    }
    h4,
    th {
      color: #aaa;
    }
    th,
    td {
      border-bottom-color: #2a2a2a;
    }
    .muted {
      color: #999;
    }
    .ok {
      color: #8de8a8;
    }
    .bad {
      color: #f4a8b1;
    }
    .issues {
      color: #f4a8b1;
    }
    a {
      color: #a8c0ff;
    }
  }
</style>
