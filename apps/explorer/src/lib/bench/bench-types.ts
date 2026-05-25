// Mirrors the DTOs in apps/explorer/src-tauri/src/bench.rs.
// Keep them in sync — the Rust side is the source of truth.

export type BenchKind = "pub" | "sub";

export type BenchConfig = {
  kind: BenchKind;
  rate: number | null;
  payload: number | null;
  await_readers: number | null;
  duration: number | null;
  warmup: number | null;
  reliability: string | null;
  history_depth: number | null;
  topic: string | null;
  domain: number | null;
};

export type BenchRow = {
  t_s: number;
  rate_per_s: number;
  // pub
  sent: number | null;
  errors: number | null;
  write_avg_us: number | null;
  write_max_us: number | null;
  // sub
  recv: number | null;
  lost_wire: number | null;
  lost_dds: number | null;
  reord: number | null;
  dup: number | null;
  clock_skew_skipped: number | null;
  lat_p50_us: number | null;
  lat_p95_us: number | null;
  lat_p99_us: number | null;
  lat_max_us: number | null;
};

export type BenchSummary = {
  duration_s: number;
  total_samples: number;
  mean_rate: number;
  max_rate: number;
  min_rate: number;
  total_lost: number | null;
  loss_rate: number | null;
  mean_p50_us: number | null;
  mean_p95_us: number | null;
  max_p99_us: number | null;
  max_lat_us: number | null;
  max_write_us: number | null;
  mean_write_us: number | null;
};

export type Severity = "ok" | "warn" | "bad";

export type VerdictCheck = {
  label: string;
  severity: Severity;
  value: string;
  explain: string;
};

export type BenchVerdict = {
  severity: Severity;
  headline: string;
  checks: VerdictCheck[];
};

export type BenchReport = {
  kind: BenchKind;
  config: BenchConfig;
  rows: BenchRow[];
  summary: BenchSummary;
  verdict: BenchVerdict;
};

export type PairAnalysis = {
  pub_sent: number;
  sub_recv: number;
  sub_lost: number;
  delta: number;
  expected_warmup_discard: number | null;
  pub_rate: number;
  sub_rate: number;
  rate_diff_pct: number;
};

export type PairReport = {
  pub_report: BenchReport;
  sub_report: BenchReport;
  analysis: PairAnalysis;
  verdict: BenchVerdict;
};

export function formatUs(us: number | null | undefined): string {
  if (us == null) return "—";
  if (us < 1_000) return `${us}µs`;
  if (us < 1_000_000) return `${(us / 1_000).toFixed(2)}ms`;
  return `${(us / 1_000_000).toFixed(2)}s`;
}
