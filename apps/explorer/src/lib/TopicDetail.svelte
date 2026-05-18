<script lang="ts">
  import type { RegistrySnapshot, TopicDto } from "./types";
  import {
    builtinTopicDescription,
    isBuiltinTopic,
  } from "./builtin-topics";
  import TabSamples from "./TabSamples.svelte";
  import TabEndpoints from "./TabEndpoints.svelte";
  import TabQoS from "./TabQoS.svelte";
  import TabType from "./TabType.svelte";

  type Tab = "samples" | "qos" | "endpoints" | "type";

  type Props = {
    topic: TopicDto;
    registry: RegistrySnapshot;
    onClose: () => void;
  };
  let { topic, registry, onClose }: Props = $props();

  let activeTab = $state<Tab>("samples");

  // Reset to Samples tab whenever the topic changes (or on mount).
  $effect(() => {
    topic.name;
    activeTab = "samples";
  });

  const tabs: { id: Tab; label: string }[] = [
    { id: "samples", label: "Samples" },
    { id: "qos", label: "QoS" },
    { id: "endpoints", label: "Endpoints" },
    { id: "type", label: "Type" },
  ];

  let writerCount = $derived(
    registry.writers.filter((w) => w.topic_name === topic.name).length,
  );
  let readerCount = $derived(
    registry.readers.filter((r) => r.topic_name === topic.name).length,
  );
</script>

<div class="detail">
  <header class="hd">
    <div class="title">
      <div class="name-row">
        <strong>{topic.name}</strong>
        {#if isBuiltinTopic(topic.name)}
          <span class="badge">builtin</span>
          <span
            class="tooltip"
            data-tooltip={builtinTopicDescription(topic.name)}
            aria-label={builtinTopicDescription(topic.name) ?? ""}
          >
            &#9432;
          </span>
        {/if}
      </div>
      <div class="meta">
        <code class="type">{topic.type_name}</code>
        <span class="muted small">·</span>
        <span class="muted small">
          {writerCount} writer{writerCount === 1 ? "" : "s"} ·
          {readerCount} reader{readerCount === 1 ? "" : "s"}
        </span>
      </div>
    </div>
    <button class="close" onclick={onClose} title="Close (Esc)">×</button>
  </header>

  <nav class="tabs">
    {#each tabs as t (t.id)}
      <button
        class:active={activeTab === t.id}
        onclick={() => (activeTab = t.id)}
      >
        {t.label}
      </button>
    {/each}
  </nav>

  <div class="content">
    {#if activeTab === "samples"}
      {#key topic.name}
        <TabSamples topicName={topic.name} typeName={topic.type_name} />
      {/key}
    {:else if activeTab === "qos"}
      <TabQoS topicName={topic.name} {registry} />
    {:else if activeTab === "endpoints"}
      <TabEndpoints topicName={topic.name} {registry} />
    {:else if activeTab === "type"}
      <TabType typeName={topic.type_name} />
    {/if}
  </div>
</div>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: #fff;
  }
  .hd {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 0.6rem 0.9rem;
    border-bottom: 1px solid #eee;
    gap: 0.6rem;
  }
  .title {
    min-width: 0;
    flex: 1;
  }
  .name-row {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .name-row strong {
    font-size: 1.05rem;
    word-break: break-all;
  }
  .meta {
    margin-top: 0.15rem;
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .type {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    color: #666;
  }
  .badge {
    font-size: 0.65em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: #5b6b85;
    color: #fff;
    padding: 0.1em 0.5em;
    border-radius: 3px;
  }
  .tooltip {
    position: relative;
    cursor: help;
    color: #5b6b85;
    font-size: 0.95em;
  }
  .tooltip:hover::after {
    content: attr(data-tooltip);
    position: absolute;
    z-index: 30;
    top: 1.5em;
    left: 0;
    width: max-content;
    max-width: 320px;
    padding: 0.55rem 0.7rem;
    border-radius: 5px;
    background: #1f2430;
    color: #f3f3f3;
    font-size: 0.78rem;
    line-height: 1.45;
    white-space: pre-wrap;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    pointer-events: none;
  }
  .close {
    background: none;
    border: none;
    font-size: 1.4em;
    cursor: pointer;
    color: #888;
    line-height: 1;
    padding: 0 0.3rem;
    align-self: flex-start;
  }
  .close:hover {
    color: #000;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid #eee;
    background: #f9fafc;
    padding: 0 0.5rem;
  }
  .tabs button {
    background: none;
    border: none;
    padding: 0.5rem 0.9rem;
    font-size: 0.88rem;
    cursor: pointer;
    color: #666;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .tabs button:hover {
    color: #2c4f9c;
  }
  .tabs button.active {
    color: #2c4f9c;
    border-bottom-color: #5577cc;
    font-weight: 500;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .content > :global(*) {
    flex: 1;
    min-height: 0;
  }
  .muted {
    color: #888;
  }
  .small {
    font-size: 0.85em;
  }

  @media (prefers-color-scheme: dark) {
    .detail {
      background: #1f1f1f;
    }
    .hd,
    .tabs {
      border-bottom-color: #2a2a2a;
    }
    .tabs {
      background: #1a1d22;
    }
    .tabs button {
      color: #aaa;
    }
    .tabs button:hover,
    .tabs button.active {
      color: #a8c0ff;
    }
    .tabs button.active {
      border-bottom-color: #5b7bd6;
    }
    .type {
      color: #aaa;
    }
    .close {
      color: #aaa;
    }
    .close:hover {
      color: #fff;
    }
    .badge {
      background: #3d4a63;
    }
    .tooltip {
      color: #8a9bb8;
    }
  }
</style>
