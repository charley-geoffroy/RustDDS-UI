/**
 * QoS parsing and matching rules.
 *
 * Endpoints in the registry carry their QoS as opaque JSON (serialized
 * by the backend from its native QoS struct). We normalize that into a
 * common shape and apply the OMG DDS matching rules to decide whether
 * each (writer, reader) pair on a topic can exchange data.
 *
 * Reference: OMG DDS 1.4 spec, §2.2.3 "Supported QoS".
 */

export type Reliability = "BestEffort" | "Reliable";

export type Durability =
  | "Volatile"
  | "TransientLocal"
  | "Transient"
  | "Persistent";

export type HistoryKind = "KeepLast" | "KeepAll";

export type LivelinessKind =
  | "Automatic"
  | "ManualByParticipant"
  | "ManualByTopic";

export type Ownership = "Shared" | "Exclusive";

export type NormalizedQos = {
  reliability: Reliability;
  durability: Durability;
  history: { kind: HistoryKind; depth: number | null };
  /** nanoseconds, null = infinite */
  deadline_ns: number | null;
  liveliness: {
    kind: LivelinessKind;
    /** nanoseconds, null = infinite */
    lease_duration_ns: number | null;
  } | null;
  ownership: Ownership;
};

// ---------------------------------------------------------------------------
// Parser — turn raw serde-JSON into NormalizedQos
// ---------------------------------------------------------------------------

/**
 * RustDDS' QosPolicies fields are Options serialized as null when None,
 * or as their inner value's JSON when Some. Enum variants without payload
 * come out as strings; variants with payload come out as
 * `{ "Variant": { ... } }`.
 */
export function parseQos(raw: unknown): NormalizedQos {
  const r = (raw ?? {}) as Record<string, unknown>;

  return {
    reliability: parseReliability(r.reliability),
    durability: parseDurability(r.durability),
    history: parseHistory(r.history),
    deadline_ns: parseDuration(asField(r.deadline, "period")),
    liveliness: parseLiveliness(r.liveliness),
    ownership: parseOwnership(r.ownership),
  };
}

function asEnumKey(v: unknown): string | null {
  if (typeof v === "string") return v;
  if (v && typeof v === "object") {
    const keys = Object.keys(v as Record<string, unknown>);
    if (keys.length === 1) return keys[0];
  }
  return null;
}

function asField(v: unknown, name: string): unknown {
  if (v && typeof v === "object" && name in (v as Record<string, unknown>)) {
    return (v as Record<string, unknown>)[name];
  }
  return undefined;
}

/** RustDDS' Duration: `{ seconds: i32, fraction: u32 }` (fraction = 1/2^32 s). */
function parseDuration(v: unknown): number | null {
  if (v == null) return null;
  if (typeof v === "number") return v;
  if (typeof v === "object") {
    const o = v as Record<string, unknown>;
    if (typeof o.seconds === "number" && typeof o.fraction === "number") {
      // Treat huge seconds as "infinite"
      if (o.seconds > 1e9) return null;
      const ns_from_frac = Math.round((o.fraction / 2 ** 32) * 1e9);
      return o.seconds * 1_000_000_000 + ns_from_frac;
    }
    if (typeof o.nanos === "number" && typeof o.secs === "number") {
      return o.secs * 1_000_000_000 + o.nanos;
    }
  }
  return null;
}

function parseReliability(v: unknown): Reliability {
  const k = asEnumKey(v);
  if (k === "Reliable") return "Reliable";
  if (k === "BestEffort") return "BestEffort";
  // DDS spec default: BestEffort for DataReaders, Reliable for DataWriters.
  // We don't know which side this is here; default BestEffort is the
  // common-denominator fallback.
  return "BestEffort";
}

function parseDurability(v: unknown): Durability {
  const k = asEnumKey(v);
  if (
    k === "Volatile" ||
    k === "TransientLocal" ||
    k === "Transient" ||
    k === "Persistent"
  )
    return k;
  return "Volatile";
}

function parseHistory(v: unknown): { kind: HistoryKind; depth: number | null } {
  const k = asEnumKey(v);
  if (k === "KeepAll") return { kind: "KeepAll", depth: null };
  if (k === "KeepLast") {
    const depthRaw = asField(v, "depth");
    const depth = typeof depthRaw === "number" ? depthRaw : null;
    return { kind: "KeepLast", depth };
  }
  return { kind: "KeepLast", depth: 1 };
}

function parseLiveliness(v: unknown): NormalizedQos["liveliness"] {
  const k = asEnumKey(v);
  if (
    k !== "Automatic" &&
    k !== "ManualByParticipant" &&
    k !== "ManualByTopic"
  )
    return null;
  const inner = asField(v, k);
  const lease = parseDuration(asField(inner, "lease_duration"));
  return { kind: k, lease_duration_ns: lease };
}

function parseOwnership(v: unknown): Ownership {
  const k = asEnumKey(v);
  if (k === "Exclusive") return "Exclusive";
  return "Shared";
}

// ---------------------------------------------------------------------------
// Matching rules (OMG DDS 1.4)
// ---------------------------------------------------------------------------

export type MatchIssue = {
  policy:
    | "Reliability"
    | "Durability"
    | "Deadline"
    | "Liveliness"
    | "Ownership";
  reason: string;
};

export type MatchResult = {
  ok: boolean;
  issues: MatchIssue[];
};

const DURABILITY_RANK: Record<Durability, number> = {
  Volatile: 0,
  TransientLocal: 1,
  Transient: 2,
  Persistent: 3,
};

const LIVELINESS_RANK: Record<LivelinessKind, number> = {
  ManualByTopic: 0,
  ManualByParticipant: 1,
  Automatic: 2,
};

/**
 * Apply the OMG DDS RxO compatibility rules. The writer "offers", the
 * reader "requests"; the rule is "offered >= requested".
 */
export function matchQos(
  writer: NormalizedQos,
  reader: NormalizedQos,
): MatchResult {
  const issues: MatchIssue[] = [];

  // Reliability
  if (reader.reliability === "Reliable" && writer.reliability === "BestEffort") {
    issues.push({
      policy: "Reliability",
      reason:
        "Reader requests Reliable but writer offers BestEffort. The writer would have to upgrade (or the reader downgrade).",
    });
  }

  // Durability
  if (
    DURABILITY_RANK[reader.durability] > DURABILITY_RANK[writer.durability]
  ) {
    issues.push({
      policy: "Durability",
      reason: `Reader requests ${reader.durability} but writer offers ${writer.durability}. The writer's durability rank must be >= the reader's.`,
    });
  }

  // Deadline — writer's period must be <= reader's (writer promises at least
  // that rate). null means "infinite", which is the OMG default.
  if (
    reader.deadline_ns != null &&
    writer.deadline_ns != null &&
    writer.deadline_ns > reader.deadline_ns
  ) {
    issues.push({
      policy: "Deadline",
      reason: `Writer's deadline ${fmtNs(writer.deadline_ns)} is longer than reader's ${fmtNs(reader.deadline_ns)} — writer can't keep up.`,
    });
  }

  // Liveliness — writer kind must be "stronger or equal" (higher rank);
  // writer's lease must be <= reader's.
  if (writer.liveliness && reader.liveliness) {
    if (
      LIVELINESS_RANK[writer.liveliness.kind] <
      LIVELINESS_RANK[reader.liveliness.kind]
    ) {
      issues.push({
        policy: "Liveliness",
        reason: `Writer's liveliness kind ${writer.liveliness.kind} is weaker than reader's ${reader.liveliness.kind}.`,
      });
    }
    if (
      reader.liveliness.lease_duration_ns != null &&
      writer.liveliness.lease_duration_ns != null &&
      writer.liveliness.lease_duration_ns >
        reader.liveliness.lease_duration_ns
    ) {
      issues.push({
        policy: "Liveliness",
        reason: `Writer's lease ${fmtNs(writer.liveliness.lease_duration_ns)} is longer than reader's ${fmtNs(reader.liveliness.lease_duration_ns)}.`,
      });
    }
  }

  // Ownership must be identical.
  if (writer.ownership !== reader.ownership) {
    issues.push({
      policy: "Ownership",
      reason: `Writer is ${writer.ownership} but reader is ${reader.ownership}; ownership must match exactly.`,
    });
  }

  return { ok: issues.length === 0, issues };
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

export function fmtNs(ns: number | null): string {
  if (ns == null) return "∞";
  if (ns < 1_000) return `${ns} ns`;
  if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)} µs`;
  if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(1)} ms`;
  return `${(ns / 1_000_000_000).toFixed(2)} s`;
}

export function summarizeQos(q: NormalizedQos): string[] {
  const parts: string[] = [
    `Reliability: ${q.reliability}`,
    `Durability: ${q.durability}`,
    `History: ${q.history.kind}${q.history.depth != null ? `(${q.history.depth})` : ""}`,
  ];
  if (q.deadline_ns != null) parts.push(`Deadline: ${fmtNs(q.deadline_ns)}`);
  if (q.liveliness) {
    parts.push(
      `Liveliness: ${q.liveliness.kind} / ${fmtNs(q.liveliness.lease_duration_ns)}`,
    );
  }
  if (q.ownership !== "Shared") parts.push(`Ownership: ${q.ownership}`);
  return parts;
}
