//! Parse the CSV produced by `pub-rustdds --csv` / `sub-rustdds --csv`,
//! summarize the run, and emit a verdict (Ok / Warn / Bad + headline +
//! per-check explanations) that the front-end can render directly.
//!
//! The CSV format includes a self-describing "# config: k=v" first
//! line so the parser knows the kind (pub vs sub), the target rate,
//! the QoS, etc. — the verdict uses that to compare observed vs
//! intended values.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;

// ============================================================================
// DTOs (serialized as JSON to the Svelte front)
// ============================================================================

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BenchKind {
    Pub,
    Sub,
}

#[derive(Serialize, Clone, Debug)]
pub struct BenchConfig {
    pub kind: BenchKind,
    pub rate: Option<f64>,
    pub payload: Option<u64>,
    pub await_readers: Option<u64>,
    pub duration: Option<u64>,
    pub warmup: Option<u64>,
    pub reliability: Option<String>,
    pub history_depth: Option<i64>,
    pub topic: Option<String>,
    pub domain: Option<u16>,
}

#[derive(Serialize, Clone, Debug)]
pub struct BenchRow {
    pub t_s: f64,
    pub rate_per_s: f64,
    // pub-only
    pub sent: Option<u64>,
    pub errors: Option<u64>,
    pub write_avg_us: Option<u64>,
    pub write_max_us: Option<u64>,
    // sub-only
    pub recv: Option<u64>,
    pub lost_wire: Option<u64>,
    pub lost_dds: Option<u64>,
    pub reord: Option<u64>,
    pub dup: Option<u64>,
    pub clock_skew_skipped: Option<u64>,
    pub lat_p50_us: Option<u64>,
    pub lat_p95_us: Option<u64>,
    pub lat_p99_us: Option<u64>,
    pub lat_max_us: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct BenchSummary {
    pub duration_s: f64,
    pub total_samples: u64,
    pub mean_rate: f64,
    pub max_rate: f64,
    pub min_rate: f64,
    // sub-only
    pub total_lost: Option<u64>,
    pub loss_rate: Option<f64>,
    pub mean_p50_us: Option<u64>,
    pub mean_p95_us: Option<u64>,
    pub max_p99_us: Option<u64>,
    pub max_lat_us: Option<u64>,
    // pub-only
    pub max_write_us: Option<u64>,
    pub mean_write_us: Option<u64>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ok,
    Warn,
    Bad,
}

#[derive(Serialize, Clone, Debug)]
pub struct VerdictCheck {
    pub label: String,
    pub severity: Severity,
    pub value: String,
    pub explain: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct BenchVerdict {
    pub severity: Severity,
    pub headline: String,
    pub checks: Vec<VerdictCheck>,
}

#[derive(Serialize, Clone, Debug)]
pub struct BenchReport {
    pub kind: BenchKind,
    pub config: BenchConfig,
    pub rows: Vec<BenchRow>,
    pub summary: BenchSummary,
    pub verdict: BenchVerdict,
}

/// Cross-side numbers for a pub+sub paired run. Both reports are
/// post-warmup so counts and rates are directly comparable.
#[derive(Serialize, Clone, Debug)]
pub struct PairAnalysis {
    pub pub_sent: u64,
    pub sub_recv: u64,
    pub sub_lost: u64,
    /// `pub_sent - sub_recv - sub_lost`. Positive means there are
    /// samples the pub sent that the sub never counted (usually
    /// expected: sub's own warmup + tail after sub stopped).
    pub delta: i64,
    /// How big a delta is "expected" given the sub's warmup window
    /// (which silently discards samples between first-receive and
    /// the warmup boundary).
    pub expected_warmup_discard: Option<u64>,
    pub pub_rate: f64,
    pub sub_rate: f64,
    /// `|pub_rate - sub_rate| / pub_rate * 100`. Should be near zero
    /// if the wire isn't saturated.
    pub rate_diff_pct: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct PairReport {
    pub pub_report: BenchReport,
    pub sub_report: BenchReport,
    pub analysis: PairAnalysis,
    /// Combined verdict — worst of pub.verdict, sub.verdict, and
    /// pair-specific checks (rate match + delta sanity).
    pub verdict: BenchVerdict,
}

// ============================================================================
// Entry point
// ============================================================================

pub fn import_bench_csv(path: &Path) -> Result<BenchReport> {
    let content = fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    parse_bench_csv_str(&content)
}

/// Parse a (pub-csv, sub-csv) pair and compute the cross-side
/// analysis: how many samples crossed the wire, what's the rate
/// alignment, where did the missing samples likely go.
pub fn parse_bench_pair_str(pub_csv: &str, sub_csv: &str) -> Result<PairReport> {
    let pub_report = parse_bench_csv_str(pub_csv)?;
    let sub_report = parse_bench_csv_str(sub_csv)?;
    if pub_report.kind != BenchKind::Pub {
        bail!("first argument must be a pub CSV (got kind={:?})", pub_report.kind);
    }
    if sub_report.kind != BenchKind::Sub {
        bail!("second argument must be a sub CSV (got kind={:?})", sub_report.kind);
    }
    // Sanity-check the two CSVs really come from the same run.
    // Mismatched topic / domain / reliability almost always means the
    // user paired files from different runs; without this guard the
    // pair analysis would silently produce nonsense deltas.
    let mismatches = pair_config_mismatches(&pub_report.config, &sub_report.config);
    if !mismatches.is_empty() {
        bail!(
            "pub/sub CSVs disagree on {} — probably from different runs",
            mismatches.join(", ")
        );
    }
    let analysis = analyze_pair(&pub_report, &sub_report);
    let verdict = pair_verdict(&pub_report, &sub_report, &analysis);
    Ok(PairReport {
        pub_report,
        sub_report,
        analysis,
        verdict,
    })
}

/// Parse a CSV directly from an in-memory string. The front-end uses
/// this via the `parse_bench_csv` command after reading the file
/// through an `<input type=file>` (avoids needing the Tauri dialog
/// plugin + extra capabilities).
pub fn parse_bench_csv_str(content: &str) -> Result<BenchReport> {
    let (config, body) = split_config_header(content)?;
    let rows = parse_rows(&body, config.kind)?;
    let summary = compute_summary(&rows, config.kind);
    let verdict = compute_verdict(&config, &summary);
    Ok(BenchReport {
        kind: config.kind,
        config,
        rows,
        summary,
        verdict,
    })
}

// ============================================================================
// Config header
// ============================================================================

fn split_config_header(content: &str) -> Result<(BenchConfig, String)> {
    let mut lines = content.lines();
    let first = lines
        .next()
        .ok_or_else(|| anyhow!("empty file"))?
        .trim_end();
    let config = if let Some(rest) = first.strip_prefix("# config:") {
        parse_config_header(rest.trim())?
    } else {
        bail!(
            "missing '# config:' header on the first line — re-run with a \
             pub/sub-rustdds that emits it (>= bench-csv revision)"
        );
    };
    // Everything past the first line is normal CSV.
    let body: Vec<&str> = lines.collect();
    Ok((config, body.join("\n")))
}

fn parse_config_header(s: &str) -> Result<BenchConfig> {
    let mut map: HashMap<&str, &str> = HashMap::new();
    for token in s.split_whitespace() {
        if let Some((k, v)) = token.split_once('=') {
            map.insert(k, v);
        }
    }
    let kind = match map.get("kind").copied() {
        Some("pub") => BenchKind::Pub,
        Some("sub") => BenchKind::Sub,
        Some(other) => bail!("unknown kind={other}"),
        None => bail!("missing kind in '# config:' header"),
    };
    Ok(BenchConfig {
        kind,
        rate: map.get("rate").and_then(|v| v.parse().ok()),
        payload: map.get("payload").and_then(|v| v.parse().ok()),
        await_readers: map.get("await_readers").and_then(|v| v.parse().ok()),
        duration: map.get("duration").and_then(|v| v.parse().ok()),
        warmup: map.get("warmup").and_then(|v| v.parse().ok()),
        reliability: map.get("reliability").map(|s| s.to_string()),
        history_depth: map.get("history_depth").and_then(|v| v.parse().ok()),
        topic: map.get("topic").map(|s| s.to_string()),
        domain: map.get("domain").and_then(|v| v.parse().ok()),
    })
}

// ============================================================================
// Rows
// ============================================================================

fn parse_rows(body: &str, kind: BenchKind) -> Result<Vec<BenchRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(body.as_bytes());
    let mut out = Vec::new();
    for rec in reader.records() {
        let rec = rec.context("csv parse")?;
        out.push(row_from_record(&rec, kind)?);
    }
    Ok(out)
}

fn row_from_record(rec: &csv::StringRecord, kind: BenchKind) -> Result<BenchRow> {
    let f = |i: usize| -> Result<f64> {
        rec.get(i)
            .and_then(|v| v.parse::<f64>().ok())
            .ok_or_else(|| anyhow!("bad f64 at col {i}: {:?}", rec.get(i)))
    };
    let u = |i: usize| -> Result<u64> {
        rec.get(i)
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| anyhow!("bad u64 at col {i}: {:?}", rec.get(i)))
    };

    let mut row = BenchRow {
        t_s: f(0)?,
        rate_per_s: 0.0,
        sent: None,
        errors: None,
        write_avg_us: None,
        write_max_us: None,
        recv: None,
        lost_wire: None,
        lost_dds: None,
        reord: None,
        dup: None,
        clock_skew_skipped: None,
        lat_p50_us: None,
        lat_p95_us: None,
        lat_p99_us: None,
        lat_max_us: None,
    };
    match kind {
        BenchKind::Pub => {
            // t_s,sent,errors,rate_per_s,write_avg_us,write_max_us
            row.sent = Some(u(1)?);
            row.errors = Some(u(2)?);
            row.rate_per_s = f(3)?;
            row.write_avg_us = Some(u(4)?);
            row.write_max_us = Some(u(5)?);
        }
        BenchKind::Sub => {
            // t_s,recv,lost_wire,lost_dds,reord,dup,clock_skew_skipped,
            // rate_per_s,lat_p50_us,lat_p95_us,lat_p99_us,lat_max_us
            row.recv = Some(u(1)?);
            row.lost_wire = Some(u(2)?);
            row.lost_dds = Some(u(3)?);
            row.reord = Some(u(4)?);
            row.dup = Some(u(5)?);
            row.clock_skew_skipped = Some(u(6)?);
            row.rate_per_s = f(7)?;
            row.lat_p50_us = Some(u(8)?);
            row.lat_p95_us = Some(u(9)?);
            row.lat_p99_us = Some(u(10)?);
            row.lat_max_us = Some(u(11)?);
        }
    }
    Ok(row)
}

// ============================================================================
// Summary
// ============================================================================

fn compute_summary(rows: &[BenchRow], kind: BenchKind) -> BenchSummary {
    if rows.is_empty() {
        return BenchSummary {
            duration_s: 0.0,
            total_samples: 0,
            mean_rate: 0.0,
            max_rate: 0.0,
            min_rate: 0.0,
            total_lost: None,
            loss_rate: None,
            mean_p50_us: None,
            mean_p95_us: None,
            max_p99_us: None,
            max_lat_us: None,
            max_write_us: None,
            mean_write_us: None,
        };
    }
    let last = rows.last().unwrap();
    let duration_s = last.t_s;
    let total_samples = match kind {
        BenchKind::Pub => last.sent.unwrap_or(0),
        BenchKind::Sub => last.recv.unwrap_or(0),
    };
    let mean_rate = rows.iter().map(|r| r.rate_per_s).sum::<f64>() / rows.len() as f64;
    let max_rate = rows
        .iter()
        .map(|r| r.rate_per_s)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_rate = rows
        .iter()
        .map(|r| r.rate_per_s)
        .fold(f64::INFINITY, f64::min);

    let (total_lost, loss_rate, mean_p50_us, mean_p95_us, max_p99_us, max_lat_us) =
        if kind == BenchKind::Sub {
            let total_lost = last.lost_wire.unwrap_or(0);
            let denom = total_samples + total_lost;
            let lr = if denom > 0 {
                total_lost as f64 / denom as f64
            } else {
                0.0
            };
            let mean_p50 = mean_u64(rows.iter().filter_map(|r| r.lat_p50_us));
            let mean_p95 = mean_u64(rows.iter().filter_map(|r| r.lat_p95_us));
            let max_p99 = rows.iter().filter_map(|r| r.lat_p99_us).max();
            let max_lat = rows.iter().filter_map(|r| r.lat_max_us).max();
            (
                Some(total_lost),
                Some(lr),
                Some(mean_p50),
                Some(mean_p95),
                max_p99,
                max_lat,
            )
        } else {
            (None, None, None, None, None, None)
        };
    let (max_write_us, mean_write_us) = if kind == BenchKind::Pub {
        let mx = rows.iter().filter_map(|r| r.write_max_us).max();
        let mn = mean_u64(rows.iter().filter_map(|r| r.write_avg_us));
        (mx, Some(mn))
    } else {
        (None, None)
    };

    BenchSummary {
        duration_s,
        total_samples,
        mean_rate,
        max_rate,
        min_rate,
        total_lost,
        loss_rate,
        mean_p50_us,
        mean_p95_us,
        max_p99_us,
        max_lat_us,
        max_write_us,
        mean_write_us,
    }
}

fn mean_u64<I: Iterator<Item = u64>>(it: I) -> u64 {
    let (sum, count) = it.fold((0u64, 0u64), |(s, c), v| (s + v, c + 1));
    if count > 0 {
        sum / count
    } else {
        0
    }
}

// ============================================================================
// Verdict — thresholds tuned for "DDS playground on a desktop OS"
// ============================================================================

fn compute_verdict(cfg: &BenchConfig, s: &BenchSummary) -> BenchVerdict {
    let mut checks = Vec::new();

    if let Some(loss_rate) = s.loss_rate {
        let pct = loss_rate * 100.0;
        let sev = if loss_rate == 0.0 {
            Severity::Ok
        } else if loss_rate < 0.001 {
            Severity::Warn
        } else {
            Severity::Bad
        };
        let explain = match sev {
            Severity::Ok => "Aucun trou détecté dans la séquence de compteurs. \
                Avec Reliable + loopback, c'est le résultat attendu."
                .to_string(),
            Severity::Warn => format!(
                "{pct:.3}% de pertes — faible mais non nul. Probablement un buffer KeepLast \
                 trop petit pendant un pic de charge, ou une fenêtre de catch-up qu'on n'a \
                 pas drainée à temps."
            ),
            Severity::Bad => format!(
                "{pct:.2}% de pertes — significatif. Vérifie que la durabilité, la fiabilité \
                 et `--history-depth` sont alignés entre pub et sub."
            ),
        };
        checks.push(VerdictCheck {
            label: "Pertes".into(),
            severity: sev,
            value: format!("{} ({:.2}%)", s.total_lost.unwrap_or(0), pct),
            explain,
        });
    }

    if let Some(p99) = s.max_p99_us {
        let sev = if p99 < 2_000 {
            Severity::Ok
        } else if p99 < 10_000 {
            Severity::Warn
        } else {
            Severity::Bad
        };
        let explain = match sev {
            Severity::Ok => "p99 sous 2ms — bon résultat. 1 sample sur 100 a été plus lent \
                que cette valeur."
                .into(),
            Severity::Warn => "p99 entre 2 et 10ms — jitter notable. Typiquement un GC, \
                un context switch, ou un autre process qui mange un coeur. Acceptable hors \
                temps-réel."
                .into(),
            Severity::Bad => "p99 > 10ms — gros jitter. Sur un OS desktop ça peut arriver mais \
                ça mérite investigation : charge système, allocations, pauses GC."
                .into(),
        };
        checks.push(VerdictCheck {
            label: "Latence p99".into(),
            severity: sev,
            value: format_us(p99),
            explain,
        });
    }

    if let (Some(p99), Some(max_lat)) = (s.max_p99_us, s.max_lat_us) {
        let ratio = max_lat as f64 / p99.max(1) as f64;
        if ratio > 5.0 {
            checks.push(VerdictCheck {
                label: "Outlier max".into(),
                severity: Severity::Warn,
                value: format!("{} (×{:.1} le p99)", format_us(max_lat), ratio),
                explain: format!(
                    "Un sample a atteint {}, soit {:.1}× le p99. C'est un événement isolé, \
                     typique d'une pause système ponctuelle — le max ne reflète pas la \
                     latence \"normale\" du run.",
                    format_us(max_lat),
                    ratio,
                ),
            });
        }
    }

    if let Some(target_rate) = cfg.rate {
        let diff = s.mean_rate - target_rate;
        let diff_pct = (diff / target_rate) * 100.0;
        let abs_pct = diff_pct.abs();
        let sev = if abs_pct < 5.0 {
            Severity::Ok
        } else if abs_pct < 20.0 {
            Severity::Warn
        } else {
            Severity::Bad
        };
        let explain = match sev {
            Severity::Ok => "Le débit observé est proche de la cible — pas de saturation.".into(),
            Severity::Warn => "Le débit s'écarte sensiblement de la cible. Tu approches \
                peut-être la limite du writer (essaie de réduire `--rate` ou d'augmenter \
                `--history-depth`)."
                .into(),
            Severity::Bad => "Le débit est très loin de la cible. Probable saturation ou \
                contention sur le writer."
                .into(),
        };
        checks.push(VerdictCheck {
            label: "Débit vs cible".into(),
            severity: sev,
            value: format!(
                "{:.1}/s observé vs {:.0}/s demandé ({:+.1}%)",
                s.mean_rate, target_rate, diff_pct
            ),
            explain,
        });
    } else {
        checks.push(VerdictCheck {
            label: "Débit moyen".into(),
            severity: Severity::Ok,
            value: format!("{:.1}/s", s.mean_rate),
            explain: format!(
                "Pas de cible explicite dans le CSV (côté sub). Débit observé : {:.1}/s, \
                 min {:.1}, max {:.1}.",
                s.mean_rate, s.min_rate, s.max_rate
            ),
        });
    }

    let severity = checks
        .iter()
        .map(|c| c.severity)
        .max()
        .unwrap_or(Severity::Ok);
    let headline = match severity {
        Severity::Ok => "Run sain",
        Severity::Warn => "Run avec quelques alertes",
        Severity::Bad => "Problèmes détectés",
    }
    .to_string();

    BenchVerdict {
        severity,
        headline,
        checks,
    }
}

// ============================================================================
// Pair analysis + combined verdict
// ============================================================================

fn analyze_pair(pub_r: &BenchReport, sub_r: &BenchReport) -> PairAnalysis {
    let pub_sent = pub_r.summary.total_samples;
    let sub_recv = sub_r.summary.total_samples;
    let sub_lost = sub_r.summary.total_lost.unwrap_or(0);
    let delta = pub_sent as i64 - sub_recv as i64 - sub_lost as i64;

    let pub_rate = pub_r.summary.mean_rate;
    let sub_rate = sub_r.summary.mean_rate;
    let rate_diff_pct = if pub_rate > 0.0 {
        ((pub_rate - sub_rate) / pub_rate * 100.0).abs()
    } else {
        0.0
    };

    // Both sides skip samples during their own warmup window before
    // resetting their counters. Since `await_readers` aligns the start
    // times within milliseconds, the expected delta is dominated by the
    // *difference* in warmup lengths: when sub_warmup > pub_warmup, pub
    // counts samples that sub still drops, and the delta is positive.
    // When pub_warmup > sub_warmup, it goes the other way and delta can
    // legitimately be negative.
    let expected_warmup_discard = sub_r.config.warmup.map(|sw| {
        let pw = pub_r.config.warmup.unwrap_or(0);
        let net = sw.saturating_sub(pw);
        (net as f64 * pub_rate).round() as u64
    });

    PairAnalysis {
        pub_sent,
        sub_recv,
        sub_lost,
        delta,
        expected_warmup_discard,
        pub_rate,
        sub_rate,
        rate_diff_pct,
    }
}

/// List the config fields where pub and sub disagree. Used to refuse
/// pairing CSVs from clearly different runs.
fn pair_config_mismatches(p: &BenchConfig, s: &BenchConfig) -> Vec<String> {
    let mut out = Vec::new();
    if p.topic != s.topic {
        out.push(format!("topic ({:?} vs {:?})", p.topic, s.topic));
    }
    if p.domain != s.domain {
        out.push(format!("domain ({:?} vs {:?})", p.domain, s.domain));
    }
    if p.reliability != s.reliability {
        out.push(format!(
            "reliability ({:?} vs {:?})",
            p.reliability, s.reliability
        ));
    }
    out
}

fn pair_verdict(
    pub_r: &BenchReport,
    sub_r: &BenchReport,
    a: &PairAnalysis,
) -> BenchVerdict {
    // Start from the union of both individual verdicts, then add
    // pair-specific checks.
    let mut checks: Vec<VerdictCheck> = pub_r
        .verdict
        .checks
        .iter()
        .map(|c| VerdictCheck {
            label: format!("[pub] {}", c.label),
            ..c.clone()
        })
        .chain(sub_r.verdict.checks.iter().map(|c| VerdictCheck {
            label: format!("[sub] {}", c.label),
            ..c.clone()
        }))
        .collect();

    // Rate alignment — if the sub couldn't keep up, its mean rate will
    // sit below the pub's mean rate.
    let rate_sev = if a.rate_diff_pct < 5.0 {
        Severity::Ok
    } else if a.rate_diff_pct < 15.0 {
        Severity::Warn
    } else {
        Severity::Bad
    };
    let rate_explain = match rate_sev {
        Severity::Ok => "Le pub et le sub vivent au même rythme — pas de congestion.".into(),
        Severity::Warn => format!(
            "Écart de {:.1}% entre les débits — le sub commence à traîner. Vérifie la charge \
             du process consommateur ou réduis `--rate`.",
            a.rate_diff_pct
        ),
        Severity::Bad => format!(
            "Écart de {:.1}% entre les débits — le sub est saturé. Probable backpressure ou \
             contention système.",
            a.rate_diff_pct
        ),
    };
    checks.push(VerdictCheck {
        label: "[pair] Alignement débit".into(),
        severity: rate_sev,
        value: format!("pub {:.0}/s vs sub {:.0}/s", a.pub_rate, a.sub_rate),
        explain: rate_explain,
    });

    // Delta sanity — compare `delta` to the signed expected value
    // derived from the *difference* between pub and sub warmups. If
    // pub_warmup > sub_warmup the expected delta is negative (sub
    // counted more than pub before its own reset) and a negative
    // observed delta is fine, not a bug.
    let pub_warmup = pub_r.config.warmup;
    let sub_warmup = sub_r.config.warmup;
    let expected_signed: i64 = match (pub_warmup, sub_warmup) {
        (Some(pw), Some(sw)) => {
            ((sw as i64 - pw as i64) as f64 * a.pub_rate).round() as i64
        }
        _ => 0,
    };
    let dev = (a.delta - expected_signed).abs();
    let scale = expected_signed.unsigned_abs() as f64;
    // Symmetric tolerance around `expected_signed`, with absolute floors
    // for the case where the expected delta is ~0 (tail + scheduling
    // jitter still produce a few hundred samples of slack).
    let tol_ok = (scale * 0.5).round() as i64 + 100;
    let tol_warn = (scale * 2.0).round() as i64 + 500;
    let warmup_desc = match (pub_warmup, sub_warmup) {
        (Some(pw), Some(sw)) => format!("pub warmup {}s, sub warmup {}s", pw, sw),
        (Some(pw), None) => format!("pub warmup {}s, sub warmup inconnu", pw),
        (None, Some(sw)) => format!("pub warmup inconnu, sub warmup {}s", sw),
        (None, None) => "warmups inconnus".to_string(),
    };
    let (delta_sev, delta_explain) = if dev <= tol_ok {
        (
            Severity::Ok,
            format!(
                "Delta de {} samples — cohérent avec l'attendu ~{} ({}). \
                 L'écart résiduel couvre le tail et la gigue d'ordonnancement.",
                a.delta, expected_signed, warmup_desc,
            ),
        )
    } else if dev <= tol_warn {
        (
            Severity::Warn,
            format!(
                "Delta de {} samples (attendu ~{}, écart {}). Le pub a peut-être \
                 continué à émettre longtemps après l'arrêt du sub, ou il y a eu de la \
                 contention. {}.",
                a.delta, expected_signed, dev, warmup_desc,
            ),
        )
    } else {
        (
            Severity::Bad,
            format!(
                "Delta de {} samples (attendu ~{}, écart {}). Les deux CSV \
                 semblent provenir de runs différents, ou il manque vraiment des samples \
                 côté sub. {}.",
                a.delta, expected_signed, dev, warmup_desc,
            ),
        )
    };
    checks.push(VerdictCheck {
        label: "[pair] Cohérence des comptes".into(),
        severity: delta_sev,
        value: format!(
            "pub sent {} · sub recv {} · sub lost {} · delta {:+}",
            a.pub_sent, a.sub_recv, a.sub_lost, a.delta
        ),
        explain: delta_explain,
    });

    let severity = checks
        .iter()
        .map(|c| c.severity)
        .max()
        .unwrap_or(Severity::Ok);
    let headline = match severity {
        Severity::Ok => "Pub et sub cohérents",
        Severity::Warn => "Cohérents avec quelques alertes",
        Severity::Bad => "Incohérences détectées",
    }
    .to_string();

    BenchVerdict {
        severity,
        headline,
        checks,
    }
}

fn format_us(us: u64) -> String {
    if us < 1_000 {
        format!("{}µs", us)
    } else if us < 1_000_000 {
        format!("{:.2}ms", us as f64 / 1_000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sub_csv(rows: &str) -> String {
        format!(
            "# config: kind=sub duration=10 warmup=0 reliability=Reliable history_depth=100 topic=Chatter domain=0\n\
             t_s,recv,lost_wire,lost_dds,reord,dup,clock_skew_skipped,rate_per_s,lat_p50_us,lat_p95_us,lat_p99_us,lat_max_us\n{rows}"
        )
    }

    fn pub_csv(rows: &str) -> String {
        format!(
            "# config: kind=pub rate=1000 payload=128 duration=10 warmup=0 await_readers=1 \
             reliability=Reliable history_depth=100 topic=Chatter domain=0\n\
             t_s,sent,errors,rate_per_s,write_avg_us,write_max_us\n{rows}"
        )
    }

    fn parse_str(s: &str) -> BenchReport {
        // Use the in-memory parser so parallel tests don't race on a
        // shared temp filename.
        parse_bench_csv_str(s).unwrap()
    }

    #[test]
    fn parse_sub_healthy_run() {
        let csv = sub_csv(
            "1.0,1000,0,0,0,0,0,1000.0,196,393,786,1129\n\
             2.0,2000,0,0,0,0,0,1000.0,196,393,786,1129\n",
        );
        let report = parse_str(&csv);
        assert_eq!(report.kind, BenchKind::Sub);
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.summary.total_samples, 2000);
        assert_eq!(report.summary.total_lost, Some(0));
        assert_eq!(report.verdict.severity, Severity::Ok);
    }

    #[test]
    fn parse_sub_with_loss_flags_bad() {
        let csv = sub_csv(
            "1.0,500,50,0,0,0,0,500.0,500,1000,2000,3000\n\
             2.0,1000,100,0,0,0,0,500.0,500,1000,2000,3000\n",
        );
        let r = parse_str(&csv);
        // 100 lost out of 1100 = ~9% → Bad
        assert_eq!(r.verdict.severity, Severity::Bad);
        let loss = r.verdict.checks.iter().find(|c| c.label == "Pertes").unwrap();
        assert_eq!(loss.severity, Severity::Bad);
    }

    #[test]
    fn parse_pub_with_rate_target() {
        let csv = pub_csv(
            "1.0,1000,0,1000.0,15,40\n\
             2.0,2000,0,1000.0,15,40\n",
        );
        let r = parse_str(&csv);
        assert_eq!(r.kind, BenchKind::Pub);
        assert_eq!(r.config.rate, Some(1000.0));
        assert_eq!(r.verdict.severity, Severity::Ok);
    }

    #[test]
    fn outlier_max_triggers_warn() {
        let csv = sub_csv(
            "1.0,1000,0,0,0,0,0,1000.0,196,393,786,15000\n\
             2.0,2000,0,0,0,0,0,1000.0,196,393,786,15000\n",
        );
        let r = parse_str(&csv);
        let outlier = r.verdict.checks.iter().find(|c| c.label == "Outlier max");
        assert!(outlier.is_some());
        assert_eq!(outlier.unwrap().severity, Severity::Warn);
    }

    #[test]
    fn pair_analysis_healthy_run() {
        // Pub at 1000Hz for ~3s, sub at 2s warmup with one fewer
        // second of measurement (typical setup).
        let p = pub_csv(
            "1.0,1000,0,1000.0,15,40\n\
             2.0,2000,0,1000.0,15,40\n\
             3.0,3000,0,1000.0,15,40\n",
        );
        let s = sub_csv(
            "1.0,1000,0,0,0,0,0,1000.0,196,393,786,1129\n\
             2.0,2000,0,0,0,0,0,1000.0,196,393,786,1129\n",
        );
        let pair = parse_bench_pair_str(&p, &s).unwrap();
        assert_eq!(pair.analysis.pub_sent, 3000);
        assert_eq!(pair.analysis.sub_recv, 2000);
        assert_eq!(pair.analysis.sub_lost, 0);
        assert_eq!(pair.analysis.delta, 1000);
        // sub.config.warmup=0 in sub_csv helper, so no warmup estimate.
        assert_eq!(pair.analysis.expected_warmup_discard, Some(0));
        // Rates match → Ok on alignment, but delta=1000 is way above
        // the 0 warmup estimate → Bad on coherence.
        let coh = pair
            .verdict
            .checks
            .iter()
            .find(|c| c.label.contains("Cohérence"))
            .unwrap();
        assert_eq!(coh.severity, Severity::Bad);
    }

    #[test]
    fn pair_rejects_swapped_csvs() {
        let p = pub_csv("1.0,1000,0,1000.0,15,40\n");
        let s = sub_csv("1.0,1000,0,0,0,0,0,1000.0,196,393,786,1129\n");
        // Swap arguments — should error.
        let err = parse_bench_pair_str(&s, &p).unwrap_err();
        assert!(err.to_string().contains("first argument must be a pub"));
    }

    #[test]
    fn missing_config_header_errors_clearly() {
        let csv = "t_s,sent,errors,rate_per_s,write_avg_us,write_max_us\n1.0,100,0,100.0,15,40\n";
        let err = parse_bench_csv_str(csv).unwrap_err();
        assert!(err.to_string().contains("# config:"));
    }
}
