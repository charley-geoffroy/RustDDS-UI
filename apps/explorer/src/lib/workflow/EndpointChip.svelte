<script lang="ts">
  import type { WorkflowEndpoint } from "./workflow-scenario";

  type Props = {
    endpoint: WorkflowEndpoint;
    state: "idle" | "detected" | "matched" | "mismatch";
    active: boolean;
    onClick: () => void;
  };

  let { endpoint, state, active, onClick }: Props = $props();
</script>

<button
  type="button"
  class="chip"
  class:active
  data-state={state}
  onclick={onClick}
  title="Click for endpoint details"
>
  <span class="arrow">{endpoint.kind === "writer" ? "↑" : "↓"}</span>
  <span class="topic">{endpoint.topic}</span>
  <span class="kind muted">{endpoint.kind}</span>
</button>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    width: 100%;
    padding: 0.2rem 0.5rem;
    border-radius: 5px;
    border: 1px solid #d8dde7;
    background: #f6f8fc;
    color: #555;
    font: inherit;
    font-size: 0.78rem;
    cursor: pointer;
    text-align: left;
    transition:
      background 0.15s,
      border-color 0.15s,
      color 0.15s;
  }
  .chip:hover {
    background: #eef3ff;
  }
  .chip.active {
    box-shadow: 0 0 0 2px rgba(85, 119, 204, 0.35);
  }
  .chip[data-state="detected"] {
    background: #e9f1ff;
    border-color: #a8c0ff;
    color: #2c4f9c;
  }
  .chip[data-state="matched"] {
    background: #d4edda;
    border-color: #6cc079;
    color: #1e6e3e;
  }
  .chip[data-state="mismatch"] {
    background: #f8d7da;
    border-color: #ec9faa;
    color: #842029;
  }
  .arrow {
    font-weight: 700;
  }
  .topic {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-size: 0.7rem;
  }
  .muted {
    color: inherit;
    opacity: 0.65;
  }

  @media (prefers-color-scheme: dark) {
    .chip {
      background: #232830;
      border-color: #353d4a;
      color: #ccc;
    }
    .chip:hover {
      background: #2a3340;
    }
    .chip[data-state="detected"] {
      background: #1f2a3f;
      border-color: #3d4a63;
      color: #a8c0ff;
    }
    .chip[data-state="matched"] {
      background: #1d3a26;
      border-color: #2a5a38;
      color: #8de8a8;
    }
    .chip[data-state="mismatch"] {
      background: #3a1d22;
      border-color: #5a2a32;
      color: #f4a8b1;
    }
  }
</style>
