<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";
  import type {
    WorkflowEndpoint,
    WorkflowParticipant,
  } from "./workflow-scenario";
  import { endpointsOf } from "./workflow-scenario";
  import EndpointChip from "./EndpointChip.svelte";

  type EndpointState = "idle" | "detected" | "matched" | "mismatch";

  type Props = {
    data: {
      participant: WorkflowParticipant;
      endpointState: (epId: string) => EndpointState;
      onParticipantClick: (id: string) => void;
      onEndpointClick: (epId: string) => void;
      activeSelection: string | null;
      discovered: boolean;
    };
  };

  let { data }: Props = $props();

  let endpoints = $derived<WorkflowEndpoint[]>(endpointsOf(data.participant.id));
</script>

<div
  class="participant"
  class:active={data.activeSelection === data.participant.id}
  class:discovered={data.discovered}
>
  <button
    class="header"
    onclick={() => data.onParticipantClick(data.participant.id)}
    title="Click for participant details"
  >
    <span class="name">{data.participant.name}</span>
    <span class="vendor-pill">{data.participant.vendor}</span>
  </button>
  <div class="prefix">
    <code>{data.participant.guidPrefix.slice(0, 8)}…</code>
  </div>
  <div class="endpoints">
    {#each endpoints as ep (ep.id)}
      <EndpointChip
        endpoint={ep}
        state={data.endpointState(ep.id)}
        active={data.activeSelection === ep.id}
        onClick={() => data.onEndpointClick(ep.id)}
      />
    {/each}
  </div>
  <Handle type="source" position={Position.Bottom} />
  <Handle type="target" position={Position.Top} />
</div>

<style>
  .participant {
    background: #fff;
    border: 1px solid #c8d6f0;
    border-radius: 8px;
    padding: 0.5rem 0.6rem;
    min-width: 180px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }
  .participant.discovered {
    border-color: #5577cc;
  }
  .participant.active {
    border-color: #2c4f9c;
    box-shadow: 0 0 0 2px rgba(85, 119, 204, 0.3);
  }
  .header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: none;
    border: none;
    padding: 0;
    width: 100%;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .header:hover .name {
    color: #2c4f9c;
  }
  .name {
    font-weight: 600;
    font-size: 0.92rem;
    flex: 1;
  }
  .vendor-pill {
    display: inline-block;
    padding: 0.1em 0.45em;
    background: #eef3ff;
    color: #2c4f9c;
    border-radius: 999px;
    font-size: 0.65rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-weight: 500;
  }
  .prefix {
    margin-top: 0.15rem;
    font-size: 0.7rem;
    color: #888;
  }
  .prefix code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .endpoints {
    margin-top: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  @media (prefers-color-scheme: dark) {
    .participant {
      background: #1f1f1f;
      border-color: #2d3a55;
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    }
    .participant.discovered {
      border-color: #5b7bd6;
    }
    .participant.active {
      border-color: #a8c0ff;
      box-shadow: 0 0 0 2px rgba(91, 123, 214, 0.4);
    }
    .header:hover .name {
      color: #a8c0ff;
    }
    .vendor-pill {
      background: #1a2030;
      color: #a8c0ff;
    }
    .prefix {
      color: #999;
    }
  }
</style>
