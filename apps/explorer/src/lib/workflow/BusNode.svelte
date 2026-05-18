<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";

  type Props = {
    data: {
      label: string;
      sublabel: string;
      active: boolean;
      onClick: () => void;
    };
  };

  let { data }: Props = $props();
</script>

<button
  type="button"
  class="bus"
  class:active={data.active}
  onclick={data.onClick}
  title="Click for SPDP multicast group details"
>
  <Handle type="source" position={Position.Top} id="top" />
  <Handle type="target" position={Position.Top} id="top-target" />
  <Handle type="source" position={Position.Bottom} id="bottom" />
  <Handle type="target" position={Position.Bottom} id="bottom-target" />

  <span class="label">{data.label}</span>
  <span class="sublabel">{data.sublabel}</span>
</button>

<style>
  .bus {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-width: 360px;
    padding: 0.4rem 0.8rem;
    background: linear-gradient(
      90deg,
      transparent 0%,
      #eef3ff 10%,
      #eef3ff 90%,
      transparent 100%
    );
    border: none;
    border-top: 2px dashed #5577cc;
    border-bottom: 2px dashed #5577cc;
    border-radius: 0;
    color: #2c4f9c;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .bus:hover,
  .bus.active {
    background: linear-gradient(
      90deg,
      transparent 0%,
      #dce5f9 10%,
      #dce5f9 90%,
      transparent 100%
    );
  }
  .label {
    font-weight: 600;
  }
  .sublabel {
    font-size: 0.7rem;
    opacity: 0.75;
  }

  @media (prefers-color-scheme: dark) {
    .bus {
      background: linear-gradient(
        90deg,
        transparent 0%,
        #1a2030 10%,
        #1a2030 90%,
        transparent 100%
      );
      border-color: #5b7bd6;
      color: #a8c0ff;
    }
    .bus:hover,
    .bus.active {
      background: linear-gradient(
        90deg,
        transparent 0%,
        #21304a 10%,
        #21304a 90%,
        transparent 100%
      );
    }
  }
</style>
