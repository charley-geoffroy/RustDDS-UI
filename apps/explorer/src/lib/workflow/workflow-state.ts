/**
 * Phase state machine driving the workflow animation.
 *
 * The user can scrub to any phase via the timeline, or auto-advance
 * with play/pause.
 */

import type { WorkflowEndpoint } from "./workflow-scenario";
import { ENDPOINTS, INSTANCE_KEYS } from "./workflow-scenario";
import { matchQos, type MatchResult } from "../qos-matching";

export type Phase = "idle" | "spdp" | "sedp" | "match" | "flowing";

export const PHASES: { id: Phase; label: Record<"en" | "fr", string> }[] = [
  { id: "idle",    label: { en: "Idle",    fr: "Inactif" } },
  { id: "spdp",    label: { en: "SPDP",    fr: "SPDP" } },
  { id: "sedp",    label: { en: "SEDP",    fr: "SEDP" } },
  { id: "match",   label: { en: "Match",   fr: "Match" } },
  { id: "flowing", label: { en: "Flowing", fr: "En flux" } },
];

export function phaseIndex(p: Phase): number {
  return PHASES.findIndex((x) => x.id === p);
}

export function nextPhase(p: Phase): Phase | null {
  const i = phaseIndex(p);
  return i + 1 < PHASES.length ? PHASES[i + 1].id : null;
}

export function previousPhase(p: Phase): Phase | null {
  const i = phaseIndex(p);
  return i > 0 ? PHASES[i - 1].id : null;
}

/**
 * For a given phase, returns which logical edge groups are visible.
 * Used by the diagram to render SPDP/SEDP/match/data-flow edges.
 */
export function edgesVisible(phase: Phase): {
  spdp: boolean;
  sedp: boolean;
  match: boolean;
  flowing: boolean;
} {
  const i = phaseIndex(phase);
  return {
    spdp: i >= 1,
    sedp: i >= 2,
    match: i >= 3,
    flowing: i >= 4,
  };
}

export type EndpointPair = {
  writer: WorkflowEndpoint;
  reader: WorkflowEndpoint;
  match: MatchResult;
  /** Matches on topic? */
  sameTopic: boolean;
};

/**
 * Compute all writer × reader pairs that share a topic, with their
 * match status under the current QoS settings.
 */
export function computePairs(endpoints: WorkflowEndpoint[]): EndpointPair[] {
  const writers = endpoints.filter((e) => e.kind === "writer");
  const readers = endpoints.filter((e) => e.kind === "reader");
  const pairs: EndpointPair[] = [];
  for (const w of writers) {
    for (const r of readers) {
      const sameTopic = w.topic === r.topic && w.typeName === r.typeName;
      pairs.push({
        writer: w,
        reader: r,
        sameTopic,
        match: sameTopic
          ? matchQos(w.qos, r.qos)
          : { ok: false, issues: [{ policy: "Reliability", reason: "Different topic" }] },
      });
    }
  }
  return pairs;
}

/** Convenience: the instance color for a writer endpoint id. */
export function instanceColor(writerId: string): string {
  return INSTANCE_KEYS[writerId]?.color ?? "#888";
}

/** All endpoints, used by callers that don't want to thread the scenario. */
export function allEndpoints(): WorkflowEndpoint[] {
  return ENDPOINTS;
}
