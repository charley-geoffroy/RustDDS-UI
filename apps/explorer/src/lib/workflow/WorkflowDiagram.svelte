<script lang="ts">
  import {
    SvelteFlow,
    Background,
    type Edge,
    type Node,
    type EdgeMarker,
    MarkerType,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";

  import type { Lang } from "../docs/chapters";
  import type { Reliability, Durability } from "../qos-matching";
  import type { WorkflowEndpoint } from "./workflow-scenario";
  import {
    PARTICIPANTS,
    ENDPOINTS,
    INSTANCE_KEYS,
    SCENARIO_DOMAIN_ID,
    endpointsOf,
  } from "./workflow-scenario";
  import {
    PHASES,
    type Phase,
    edgesVisible,
    computePairs,
    instanceColor,
  } from "./workflow-state";

  import ParticipantNode from "./ParticipantNode.svelte";
  import BusNode from "./BusNode.svelte";
  import TimelineControls from "./TimelineControls.svelte";
  import QosTinker from "./QosTinker.svelte";
  import SidePanel from "./SidePanel.svelte";

  type Props = { lang?: Lang };
  let { lang = "en" }: Props = $props();

  // ─── State ────────────────────────────────────────────────────────────
  // Start in "flowing" so the diagram is populated on first render — the
  // user can press ⏮ to rewind and watch the workflow build up step by
  // step from idle.
  let phase = $state<Phase>("flowing");
  let playing = $state(false);
  let speed = $state(1);

  const NARRATION: Record<Phase, { en: string; fr: string }> = {
    idle: {
      en: "Three isolated participants. They don't know about each other yet. Press ▶ or click a phase tab to advance.",
      fr: "Trois participants isolés. Ils ne se connaissent pas encore. Appuie sur ▶ ou click un onglet de phase pour avancer.",
    },
    spdp: {
      en: "SPDP: each participant broadcasts a self-announcement on the multicast group 239.255.0.1. Within ~5 s everyone knows everyone.",
      fr: "SPDP : chaque participant diffuse une auto-annonce sur le groupe multicast 239.255.0.1. En ~5 s tout le monde se connaît.",
    },
    sedp: {
      en: "SEDP: participants exchange the list of their writers/readers point-to-point (unicast), with full QoS attached.",
      fr: "SEDP : les participants échangent la liste de leurs writers/readers en point-à-point (unicast), avec la QoS complète attachée.",
    },
    match: {
      en: "The middleware checks QoS compatibility for each (writer, reader) pair sharing a topic. Compatible pairs turn green.",
      fr: "Le middleware vérifie la compatibilité QoS pour chaque paire (writer, reader) partageant un topic. Les paires compatibles passent au vert.",
    },
    flowing: {
      en: "Data flows. Animated edges carry samples; labels show the instance key each writer publishes (WITH_KEY topic = N sub-streams).",
      fr: "Les données circulent. Les edges animés portent les samples ; les labels montrent la clé d'instance publiée par chaque writer (topic WITH_KEY = N sous-flux).",
    },
  };

  let narration = $derived(NARRATION[phase][lang === "fr" ? "fr" : "en"]);
  let endpoints = $state<WorkflowEndpoint[]>(
    ENDPOINTS.map((e) => ({ ...e, qos: { ...e.qos } })),
  );
  let selection = $state<
    | { kind: "none" }
    | { kind: "participant"; id: string }
    | { kind: "endpoint"; id: string }
    | { kind: "bus" }
    | { kind: "pair"; pair: ReturnType<typeof computePairs>[number] }
  >({ kind: "none" });

  let pairs = $derived(computePairs(endpoints));
  let vis = $derived(edgesVisible(phase));

  // ─── Per-endpoint state (matched/detected/mismatch) ───────────────────
  function endpointState(epId: string): "idle" | "detected" | "matched" | "mismatch" {
    if (!vis.sedp) return "idle";
    const involved = pairs.filter(
      (p) => p.sameTopic && (p.writer.id === epId || p.reader.id === epId),
    );
    if (involved.length === 0) return "detected";
    if (!vis.match) return "detected";
    const anyOk = involved.some((p) => p.match.ok);
    const anyBad = involved.some((p) => !p.match.ok);
    if (anyOk && !anyBad) return "matched";
    if (anyOk && anyBad) return "matched";
    return "mismatch";
  }

  // ─── Nodes ────────────────────────────────────────────────────────────
  const NODE_POSITIONS: Record<string, { x: number; y: number }> = {
    "sensor-a": { x: 0, y: 0 },
    "sensor-b": { x: 420, y: 0 },
    bus: { x: 80, y: 220 },
    dashboard: { x: 195, y: 360 },
  };

  let nodes = $derived<Node[]>([
    ...PARTICIPANTS.map((p) => ({
      id: p.id,
      type: "participant",
      position: NODE_POSITIONS[p.id],
      data: {
        participant: p,
        endpointState,
        discovered: vis.spdp,
        activeSelection: getActiveSelectionId(),
        onParticipantClick: (id: string) =>
          (selection = { kind: "participant", id }),
        onEndpointClick: (epId: string) =>
          (selection = { kind: "endpoint", id: epId }),
      },
      draggable: false,
      selectable: false,
    })),
    {
      id: "bus",
      type: "bus",
      position: NODE_POSITIONS["bus"],
      data: {
        label: "239.255.0.1 :7400",
        sublabel: `multicast · domain ${SCENARIO_DOMAIN_ID}`,
        active: selection.kind === "bus",
        onClick: () => (selection = { kind: "bus" }),
      },
      draggable: false,
      selectable: false,
    },
  ]);

  function getActiveSelectionId(): string | null {
    switch (selection.kind) {
      case "participant":
        return selection.id;
      case "endpoint":
        return selection.id;
      default:
        return null;
    }
  }

  // ─── Edges ────────────────────────────────────────────────────────────
  const arrow: EdgeMarker = {
    type: MarkerType.ArrowClosed,
    width: 14,
    height: 14,
  };

  let edges = $derived<Edge[]>(buildEdges(vis, pairs));

  function buildEdges(
    v: ReturnType<typeof edgesVisible>,
    pp: ReturnType<typeof computePairs>,
  ): Edge[] {
    const out: Edge[] = [];

    // SPDP — dashed multicast lines (participant → bus and back)
    if (v.spdp) {
      for (const p of PARTICIPANTS) {
        out.push({
          id: `spdp-${p.id}`,
          source: p.id,
          target: "bus",
          animated: true,
          style: "stroke: #888; stroke-dasharray: 6 4; stroke-width: 1.4;",
          markerEnd: arrow,
        });
      }
    }

    // Direct match edges (writer participant → reader participant)
    if (v.match) {
      for (const pair of pp.filter((x) => x.sameTopic)) {
        const ok = pair.match.ok;
        const color = ok ? "#1e6e3e" : "#842029";
        out.push({
          id: `match-${pair.writer.id}-${pair.reader.id}`,
          source: pair.writer.ownerId,
          target: pair.reader.ownerId,
          animated: v.flowing && ok,
          style: `stroke: ${color}; stroke-width: 2; ${ok ? "" : "stroke-dasharray: 4 4;"}`,
          markerEnd: { ...arrow, color },
          label: ok
            ? v.flowing
              ? `key=${INSTANCE_KEYS[pair.writer.id]?.key ?? "?"}`
              : ""
            : pair.match.issues[0]?.policy ?? "mismatch",
          labelStyle: ok
            ? `fill: ${instanceColor(pair.writer.id)}; font-weight: 600; font-family: ui-monospace, Menlo, monospace; font-size: 0.7rem;`
            : "fill: #842029; font-weight: 600; font-size: 0.7rem;",
          data: { pair },
          interactionWidth: 18,
        });
      }
    }

    return out;
  }

  // ─── Play loop ────────────────────────────────────────────────────────
  let timer: ReturnType<typeof setTimeout> | null = null;
  function scheduleAdvance() {
    if (!playing) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      const i = PHASES.findIndex((p) => p.id === phase);
      if (i + 1 < PHASES.length) {
        phase = PHASES[i + 1].id;
        scheduleAdvance();
      } else {
        playing = false;
      }
    }, 1500 / speed);
  }
  $effect(() => {
    if (playing) scheduleAdvance();
    else if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  });

  // ─── Handlers ─────────────────────────────────────────────────────────
  function setPhase(p: Phase) {
    phase = p;
    if (playing) scheduleAdvance();
  }
  function play() {
    if (phase === "flowing") phase = "idle";
    playing = true;
  }
  function pause() {
    playing = false;
  }
  function step() {
    const i = PHASES.findIndex((p) => p.id === phase);
    if (i + 1 < PHASES.length) phase = PHASES[i + 1].id;
  }
  function restart() {
    playing = false;
    phase = "idle";
  }
  function setSpeed(s: number) {
    speed = s;
    if (playing) scheduleAdvance();
  }
  function resetQos() {
    endpoints = ENDPOINTS.map((e) => ({ ...e, qos: { ...e.qos } }));
  }
  function updateQos(
    epId: string,
    update: Partial<{
      reliability: Reliability;
      durability: Durability;
      historyDepth: number | null;
    }>,
  ) {
    endpoints = endpoints.map((e) => {
      if (e.id !== epId) return e;
      const next = { ...e, qos: { ...e.qos } };
      if (update.reliability) next.qos.reliability = update.reliability;
      if (update.durability) next.qos.durability = update.durability;
      if (update.historyDepth !== undefined) {
        next.qos.history = {
          kind: update.historyDepth == null ? "KeepAll" : "KeepLast",
          depth: update.historyDepth,
        };
      }
      return next;
    });
  }

  function onEdgeClick(_e: Event, edge: Edge) {
    const pair = edge.data?.pair as
      | ReturnType<typeof computePairs>[number]
      | undefined;
    if (pair) {
      selection = { kind: "pair", pair };
    }
  }

  const nodeTypes = { participant: ParticipantNode, bus: BusNode };
</script>

<div class="workflow">
  <div class="narration">
    <span class="phase-label">
      Phase {PHASES.findIndex((p) => p.id === phase) + 1}/{PHASES.length}
      · <strong>{PHASES.find((p) => p.id === phase)?.label[lang]}</strong>
    </span>
    <p>{narration}</p>
  </div>

  <div class="canvas-row">
    <div class="canvas">
      <SvelteFlow
        {nodes}
        {edges}
        {nodeTypes}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        fitView
        proOptions={{ hideAttribution: true }}
        onedgeclick={(detail) => onEdgeClick(detail.event, detail.edge)}
        onpaneclick={() => (selection = { kind: "none" })}
      >
        <Background />
      </SvelteFlow>
    </div>
    <SidePanel {selection} {endpoints} {lang} />
  </div>

  <TimelineControls
    {phase}
    {playing}
    {speed}
    {lang}
    onPhase={setPhase}
    onPlay={play}
    onPause={pause}
    onStep={step}
    onRestart={restart}
    onSpeed={setSpeed}
  />

  <QosTinker
    {endpoints}
    {pairs}
    {lang}
    onQosChange={updateQos}
    onReset={resetQos}
  />
</div>

<style>
  .workflow {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin: 1.2rem 0;
  }
  .narration {
    background: #f4f7fd;
    border: 1px solid #c8d6f0;
    border-radius: 6px;
    padding: 0.5rem 0.8rem;
    font-size: 0.88rem;
    line-height: 1.45;
  }
  .narration .phase-label {
    display: inline-block;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78rem;
    color: #2c4f9c;
    margin-bottom: 0.2rem;
  }
  .narration p {
    margin: 0;
    color: #495367;
  }
  .canvas-row {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .canvas {
    height: 520px;
    background: #fafbfc;
    border: 1px solid #eef0f4;
    border-radius: 6px;
    overflow: hidden;
  }

  /* Boost svelte-flow's built-in animation for visibility on green
     match edges + dashed SPDP edges. The default stroke-dashoffset
     animation can be quite subtle on light backgrounds. */
  :global(.svelte-flow__edge.animated .svelte-flow__edge-path) {
    stroke-dasharray: 8 4;
    animation: workflow-dash 0.4s linear infinite;
  }
  @keyframes workflow-dash {
    from { stroke-dashoffset: 24; }
    to { stroke-dashoffset: 0; }
  }

  @media (prefers-color-scheme: dark) {
    .narration {
      background: #1a2030;
      border-color: #2d3a55;
    }
    .narration .phase-label {
      color: #a8c0ff;
    }
    .narration p {
      color: #c5d2ee;
    }
    .canvas {
      background: #161616;
      border-color: #2a2a2a;
    }
  }
</style>
