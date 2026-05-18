/**
 * The static fixture data for the interactive workflow chapter.
 *
 * Three participants on domain 0, two writers, one reader, one topic.
 * Two instances on the WITH_KEY topic (one per sensor). Deliberately
 * minimal but rich enough to teach every concept introduced in the
 * surrounding chapters.
 */

import type { NormalizedQos } from "../qos-matching";

export type WorkflowParticipantId = "sensor-a" | "sensor-b" | "dashboard";
export type WorkflowEndpointKind = "writer" | "reader";

export type WorkflowEndpoint = {
  id: string;
  ownerId: WorkflowParticipantId;
  kind: WorkflowEndpointKind;
  topic: string;
  typeName: string;
  qos: NormalizedQos;
};

export type WorkflowParticipant = {
  id: WorkflowParticipantId;
  name: string;
  vendor: string;
  guidPrefix: string;
  leaseSeconds: number;
};

export const SCENARIO_DOMAIN_ID = 0;
export const SCENARIO_TOPIC = "/sensors/temp";
export const SCENARIO_TYPE = "Temperature";

const QOS_RELIABLE_VOLATILE_KL10: NormalizedQos = {
  reliability: "Reliable",
  durability: "Volatile",
  history: { kind: "KeepLast", depth: 10 },
  deadline_ns: null,
  liveliness: null,
  ownership: "Shared",
};

export const PARTICIPANTS: WorkflowParticipant[] = [
  {
    id: "sensor-a",
    name: "sensor-a",
    vendor: "RustDDS",
    guidPrefix: "0112aaaa00000001",
    leaseSeconds: 10,
  },
  {
    id: "sensor-b",
    name: "sensor-b",
    vendor: "RustDDS",
    guidPrefix: "0112bbbb00000002",
    leaseSeconds: 10,
  },
  {
    id: "dashboard",
    name: "dashboard",
    vendor: "RustDDS",
    guidPrefix: "0112cccc00000003",
    leaseSeconds: 10,
  },
];

export const ENDPOINTS: WorkflowEndpoint[] = [
  {
    id: "sensor-a/writer",
    ownerId: "sensor-a",
    kind: "writer",
    topic: SCENARIO_TOPIC,
    typeName: SCENARIO_TYPE,
    qos: { ...QOS_RELIABLE_VOLATILE_KL10 },
  },
  {
    id: "sensor-b/writer",
    ownerId: "sensor-b",
    kind: "writer",
    topic: SCENARIO_TOPIC,
    typeName: SCENARIO_TYPE,
    qos: { ...QOS_RELIABLE_VOLATILE_KL10 },
  },
  {
    id: "dashboard/reader",
    ownerId: "dashboard",
    kind: "reader",
    topic: SCENARIO_TOPIC,
    typeName: SCENARIO_TYPE,
    qos: { ...QOS_RELIABLE_VOLATILE_KL10 },
  },
];

/** Instance keys per writer — these become the dot colors during the flow phase. */
export const INSTANCE_KEYS: Record<string, { key: number; color: string; label: string }> = {
  "sensor-a/writer": { key: 42, color: "#e8a33d", label: "key=42 (amber)" },
  "sensor-b/writer": { key: 43, color: "#3da6a6", label: "key=43 (teal)" },
};

export function participant(id: WorkflowParticipantId): WorkflowParticipant {
  const p = PARTICIPANTS.find((x) => x.id === id);
  if (!p) throw new Error(`unknown participant: ${id}`);
  return p;
}

export function endpointsOf(
  ownerId: WorkflowParticipantId,
): WorkflowEndpoint[] {
  return ENDPOINTS.filter((e) => e.ownerId === ownerId);
}
