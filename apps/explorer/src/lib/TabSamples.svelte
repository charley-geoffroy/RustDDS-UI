<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import type { SampleBatchDto, SampleDto } from "./types";

  type Props = {
    topicName: string;
    typeName: string;
  };
  let { topicName, typeName }: Props = $props();

  const MAX_KEPT = 200;

  let samples = $state<SampleDto[]>([]);
  let totalReceived = $state(0);
  let lastBatchAt = $state<number | null>(null);
  let rateHz = $state(0);
  let error = $state<string | null>(null);
  let subscribed = $state(false);
  let selected = $state<SampleDto | null>(null);
  let unlisten: UnlistenFn | null = null;

  // Pause state: the IPC keeps flowing (we still receive batches), but
  // we don't append to the displayed list or move the rate. We just
  // count how many samples slipped by so the user knows what they're
  // missing.
  let paused = $state(false);
  let pausedReceivedCount = $state(0);

  function updateRate(receivedSinceLast: number) {
    const now = performance.now();
    if (lastBatchAt == null) {
      lastBatchAt = now;
      return;
    }
    const dt_s = Math.max((now - lastBatchAt) / 1000, 1e-3);
    const instant = receivedSinceLast / dt_s;
    const alpha = 0.3;
    rateHz = alpha * instant + (1 - alpha) * rateHz;
    lastBatchAt = now;
  }

  async function start() {
    try {
      await invoke("subscribe_topic", { topicName, typeName });
      subscribed = true;
    } catch (e) {
      error = String(e);
      return;
    }
    unlisten = await listen<SampleBatchDto>("dds:samples", (e) => {
      if (e.payload.topic !== topicName) return;
      if (paused) {
        pausedReceivedCount += e.payload.received_since_last;
        return;
      }
      updateRate(e.payload.received_since_last);
      totalReceived += e.payload.received_since_last;
      if (e.payload.samples.length > 0) {
        samples = [...e.payload.samples.reverse(), ...samples].slice(0, MAX_KEPT);
      }
    });
  }

  function togglePause() {
    if (paused) {
      // Resuming — reset rate tracking so the gap doesn't depress the EMA.
      lastBatchAt = null;
      pausedReceivedCount = 0;
    }
    paused = !paused;
  }

  function clearSamples() {
    samples = [];
    selected = null;
  }

  async function stop() {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    if (subscribed) {
      try {
        await invoke("unsubscribe_topic", { topicName });
      } catch (e) {
        console.warn("unsubscribe failed:", e);
      }
      subscribed = false;
    }
  }

  onMount(start);
  onDestroy(stop);

  function fmtTime(ns: number): string {
    const d = new Date(ns / 1e6);
    return (
      d.toLocaleTimeString([], { hour12: false }) +
      "." +
      String(d.getMilliseconds()).padStart(3, "0")
    );
  }

  // 1 byte per group, space-separated, 16 bytes per line — matches
  // the format used in the CDR chapter and standard hex dumps (xxd).
  function chunkHex(hex: string, bytesPerLine = 16): string {
    const out: string[] = [];
    let line: string[] = [];
    for (let i = 0; i < hex.length; i += 2) {
      line.push(hex.slice(i, i + 2));
      if (line.length >= bytesPerLine) {
        out.push(line.join(" "));
        line = [];
      }
    }
    if (line.length) out.push(line.join(" "));
    return out.join("\n");
  }

  function asciiDump(hex: string): string {
    const out: string[] = [];
    for (let i = 0; i < hex.length; i += 2) {
      const code = parseInt(hex.slice(i, i + 2), 16);
      out.push(code >= 0x20 && code < 0x7f ? String.fromCharCode(code) : ".");
    }
    return out.join("");
  }

  function fmtSize(n: number): string {
    return n < 1024 ? `${n} B` : `${(n / 1024).toFixed(1)} KiB`;
  }

  let viewMode = $state<"hex" | "ascii">("hex");
</script>

<div class="tab-samples">
  <header class="stats">
    <button
      class="ctrl"
      onclick={togglePause}
      title={paused ? "Resume sample stream" : "Pause sample stream"}
      aria-label={paused ? "Resume" : "Pause"}
    >
      {paused ? "▶" : "⏸"}
    </button>
    <button
      class="ctrl"
      onclick={clearSamples}
      disabled={samples.length === 0}
      title="Clear the displayed sample list"
      aria-label="Clear"
    >
      🗑
    </button>
    <span class="rate" class:paused>
      <span class="dot" class:live={!paused && rateHz > 0.05}></span>
      {rateHz.toFixed(1)} Hz
    </span>
    <span class="muted small">
      {totalReceived} total · {samples.length} kept
    </span>
    {#if paused}
      <span class="paused-pill">
        ⏸ paused · {pausedReceivedCount} received while paused
      </span>
    {/if}
    {#if !subscribed && !error}
      <span class="muted small">subscribing…</span>
    {/if}
    <a
      href="#docs/cdr"
      class="phase4-hint"
      title="Type-aware decoded view is in development. See the CDR chapter for the manual decoding procedure today."
    >
      <span class="dot pulse"></span>
      raw CDR · decoded view <em>Phase 4</em>
    </a>
  </header>

  {#if error}
    <p class="err">{error}</p>
  {/if}

  <div class="layout">
    <ul class="sample-list">
      {#each samples as s (s.recv_ns + "-" + s.size)}
        <li>
          <button
            class:active={selected === s}
            onclick={() => (selected = s)}
          >
            <span class="time">{fmtTime(s.recv_ns)}</span>
            <span class="size">{fmtSize(s.size)}</span>
          </button>
        </li>
      {/each}
      {#if samples.length === 0 && subscribed}
        <li class="muted small empty">Waiting for samples…</li>
      {/if}
    </ul>

    <div class="detail">
      {#if selected}
        <header class="detail-head">
          <span class="muted small">
            {fmtTime(selected.recv_ns)} · {fmtSize(selected.size)}
          </span>
          <div class="view-toggle">
            <button
              class:active={viewMode === "hex"}
              onclick={() => (viewMode = "hex")}
            >hex</button>
            <button
              class:active={viewMode === "ascii"}
              onclick={() => (viewMode = "ascii")}
            >ascii</button>
          </div>
        </header>
        {#if viewMode === "hex"}
          <pre class="hex">{chunkHex(selected.bytes_hex)}</pre>
        {:else}
          <pre class="hex">{asciiDump(selected.bytes_hex)}</pre>
        {/if}
      {:else}
        <p class="muted small placeholder">
          Click a sample on the left to view its bytes.
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .tab-samples {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .stats {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.4rem 0.9rem;
    border-bottom: 1px solid #eee;
    background: #fafbfc;
  }
  .ctrl {
    background: #fff;
    border: 1px solid #d8dbe1;
    border-radius: 5px;
    padding: 0.2rem 0.55rem;
    font: inherit;
    font-size: 0.9rem;
    line-height: 1.2;
    cursor: pointer;
    color: #444;
  }
  .ctrl:hover:not(:disabled) {
    border-color: #5577cc;
    color: #2c4f9c;
    background: #f6f9ff;
  }
  .ctrl:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .rate {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.95em;
    color: #2c7a2c;
    font-weight: 500;
  }
  .rate.paused {
    color: #888;
  }
  .paused-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.15rem 0.6rem;
    background: #fff4e0;
    border: 1px solid #f5c97a;
    border-radius: 999px;
    font-size: 0.78rem;
    color: #8a4a00;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #c0c0c0;
  }
  .dot.live {
    background: #3aa83a;
    box-shadow: 0 0 8px rgba(58, 168, 58, 0.5);
  }
  .phase4-hint {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.72rem;
    color: #6f4a00;
    background: #fff8e1;
    border: 1px solid #f0d77a;
    padding: 0.1rem 0.55rem;
    border-radius: 999px;
    text-decoration: none;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .phase4-hint:hover {
    background: #ffefb8;
  }
  .phase4-hint .dot.pulse {
    background: #ec9b00;
    box-shadow: 0 0 6px rgba(236, 155, 0, 0.5);
    animation: phase4-pulse 2s ease-in-out infinite;
    width: 6px;
    height: 6px;
  }
  .phase4-hint em {
    font-style: normal;
    font-weight: 600;
  }
  @keyframes phase4-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
  .err {
    color: #c0392b;
    padding: 0.4rem 0.9rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
  }
  .layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    flex: 1;
    min-height: 0;
  }
  .sample-list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    border-right: 1px solid #eee;
  }
  .sample-list li.empty {
    padding: 0.8rem;
  }
  .sample-list button {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.5rem;
    align-items: baseline;
    width: 100%;
    padding: 0.3rem 0.7rem;
    background: none;
    border: none;
    border-bottom: 1px solid #f3f3f3;
    text-align: left;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    cursor: pointer;
    color: inherit;
  }
  .sample-list button:hover {
    background: #f6f9ff;
  }
  .sample-list button.active {
    background: #e9f1ff;
    box-shadow: inset 3px 0 0 #5577cc;
  }
  .time {
    color: #444;
  }
  .size {
    color: #888;
    font-size: 0.85em;
  }
  .detail {
    padding: 0.6rem 0.9rem;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .detail-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .view-toggle {
    display: inline-flex;
    border: 1px solid #d8dbe1;
    border-radius: 4px;
    overflow: hidden;
  }
  .view-toggle button {
    background: none;
    border: none;
    padding: 0.15rem 0.55rem;
    font-size: 0.78em;
    cursor: pointer;
    color: #666;
  }
  .view-toggle button.active {
    background: #5577cc;
    color: #fff;
  }
  .hex {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78em;
    line-height: 1.55;
    background: #f7f8fb;
    border: 1px solid #eef0f4;
    border-radius: 4px;
    padding: 0.6rem 0.8rem;
    white-space: pre;
    color: #333;
    overflow-x: auto;
    margin: 0;
  }
  .placeholder {
    margin: 1rem 0;
  }
  .muted {
    color: #888;
  }
  .small {
    font-size: 0.85em;
  }

  @media (prefers-color-scheme: dark) {
    .stats {
      background: #1a1d22;
      border-bottom-color: #2a2a2a;
    }
    .sample-list {
      border-right-color: #2a2a2a;
    }
    .sample-list button {
      border-bottom-color: #2a2a2a;
    }
    .sample-list button:hover {
      background: #232830;
    }
    .sample-list button.active {
      background: #2a3340;
    }
    .hex {
      background: #161616;
      border-color: #2a2a2a;
      color: #ddd;
    }
    .view-toggle {
      border-color: #353d4a;
    }
    .view-toggle button {
      color: #aaa;
    }
    .view-toggle button.active {
      background: #5b7bd6;
    }
    .rate {
      color: #6db96d;
    }
    .rate.paused {
      color: #888;
    }
    .ctrl {
      background: #1f1f1f;
      border-color: #353d4a;
      color: #ccc;
    }
    .ctrl:hover:not(:disabled) {
      border-color: #5b7bd6;
      color: #a8c0ff;
      background: #232830;
    }
    .paused-pill {
      background: #2a2210;
      border-color: #4a3a15;
      color: #f5c980;
    }
    .time {
      color: #ccc;
    }
    .phase4-hint {
      background: #2a2210;
      border-color: #4a3a15;
      color: #f5d480;
    }
    .phase4-hint:hover {
      background: #352a14;
    }
    .phase4-hint .dot.pulse {
      background: #f5b833;
    }
  }
</style>
