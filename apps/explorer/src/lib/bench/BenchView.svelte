<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import LineChart from "./LineChart.svelte";
  import VerdictBanner from "./VerdictBanner.svelte";
  import PairSummaryCard from "./PairSummaryCard.svelte";
  import type { BenchReport, PairReport } from "./bench-types";
  import { formatUs } from "./bench-types";

  type Slot = { report: BenchReport; text: string; fileName: string };

  let pubSlot = $state<Slot | null>(null);
  let subSlot = $state<Slot | null>(null);
  let pair = $state<PairReport | null>(null);
  let error = $state<string | null>(null);

  async function onFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    error = null;
    try {
      const text = await file.text();
      const report = await invoke<BenchReport>("parse_bench_csv", { content: text });
      const slot: Slot = { report, text, fileName: file.name };
      if (report.kind === "pub") {
        pubSlot = slot;
      } else {
        subSlot = slot;
      }
      await refreshPair();
    } catch (e: unknown) {
      error = String(e);
    } finally {
      // Reset so the user can re-pick the same file after clearing.
      input.value = "";
    }
  }

  async function refreshPair() {
    if (pubSlot && subSlot) {
      try {
        pair = await invoke<PairReport>("parse_bench_pair", {
          pubContent: pubSlot.text,
          subContent: subSlot.text,
        });
      } catch (e: unknown) {
        error = String(e);
        pair = null;
      }
    } else {
      pair = null;
    }
  }

  function clearAll() {
    pubSlot = null;
    subSlot = null;
    pair = null;
    error = null;
  }

  function clearPub() {
    pubSlot = null;
    pair = null;
  }
  function clearSub() {
    subSlot = null;
    pair = null;
  }

  const isSinglePub = $derived(pubSlot != null && subSlot == null);
  const isSingleSub = $derived(subSlot != null && pubSlot == null);
  const isPair = $derived(pair != null);
</script>

<div class="bench">
  <header class="head">
    <div>
      <h2>Bench report</h2>
      <p class="muted">
        Importe un CSV produit par <code>pub-rustdds --csv</code> et/ou
        <code>sub-rustdds --csv</code>. Si tu charges les deux, le verdict
        compare aussi les comptes pub vs sub.
      </p>
    </div>
    <div class="actions">
      <label class="btn">
        Ajouter un CSV…
        <input type="file" accept=".csv" onchange={onFile} hidden />
      </label>
      {#if pubSlot || subSlot}
        <button class="btn ghost" onclick={clearAll}>Tout effacer</button>
      {/if}
    </div>
  </header>

  <!-- Slot chips so the user sees what's loaded -->
  {#if pubSlot || subSlot}
    <div class="slots">
      <div class="slot" class:filled={pubSlot}>
        <span class="kind kind-pub">PUB</span>
        {#if pubSlot}
          <span class="filename">{pubSlot.fileName}</span>
          <button class="x" onclick={clearPub} title="Retirer le CSV pub">×</button>
        {:else}
          <span class="empty-hint">aucun</span>
        {/if}
      </div>
      <div class="slot" class:filled={subSlot}>
        <span class="kind kind-sub">SUB</span>
        {#if subSlot}
          <span class="filename">{subSlot.fileName}</span>
          <button class="x" onclick={clearSub} title="Retirer le CSV sub">×</button>
        {:else}
          <span class="empty-hint">aucun</span>
        {/if}
      </div>
    </div>
  {/if}

  {#if error}
    <div class="err">
      <strong>Erreur:</strong> {error}
    </div>
  {/if}

  {#if !pubSlot && !subSlot && !error}
    <div class="empty">
      <p>Pas encore de rapport chargé. Lance par exemple :</p>
      <pre><code>cargo run --release -p sub-rustdds -- --warmup 5 --duration 30 --csv /tmp/sub.csv
cargo run --release -p pub-rustdds -- --await-readers 1 --rate 1000 \
  --duration 30 --warmup 5 --csv /tmp/pub.csv</code></pre>
      <p>puis ajoute les deux CSV ci-dessus.</p>
    </div>
  {/if}

  {#if isPair && pair}
    <!-- Combined verdict on top, then both reports side by side. -->
    <VerdictBanner verdict={pair.verdict} />

    <section class="metrics">
      <PairSummaryCard analysis={pair.analysis} />

      <article class="card">
        <h3>Débit pub vs sub</h3>
        <p class="big">
          {pair.analysis.pub_rate.toFixed(1)}
          <span class="unit">vs {pair.analysis.sub_rate.toFixed(1)} /s</span>
        </p>
        <p class="sub">Écart {pair.analysis.rate_diff_pct.toFixed(1)}%</p>
        <!-- Two series over their OWN time axes (different start times),
             but both relative to their own t=0 — useful to see shape -->
        <LineChart
          xs={pair.pub_report.rows.map((r) => r.t_s)}
          series={[
            {
              label: "pub",
              color: "#2c4f9c",
              values: pair.pub_report.rows.map((r) => r.rate_per_s),
            },
          ]}
        />
        <LineChart
          xs={pair.sub_report.rows.map((r) => r.t_s)}
          series={[
            {
              label: "sub",
              color: "#2ea44f",
              values: pair.sub_report.rows.map((r) => r.rate_per_s),
            },
          ]}
        />
      </article>

      <article class="card">
        <h3>Latence (sub)</h3>
        <p class="big">
          {formatUs(pair.sub_report.summary.mean_p50_us)}
          <span class="unit">médian</span>
        </p>
        <p class="sub">
          p95 ≈ {formatUs(pair.sub_report.summary.mean_p95_us)} · p99 max
          {formatUs(pair.sub_report.summary.max_p99_us)}
        </p>
        <LineChart
          xs={pair.sub_report.rows.map((r) => r.t_s)}
          series={[
            { label: "p50", color: "#2ea44f", values: pair.sub_report.rows.map((r) => r.lat_p50_us ?? 0) },
            { label: "p95", color: "#d18b1f", values: pair.sub_report.rows.map((r) => r.lat_p95_us ?? 0) },
            { label: "p99", color: "#c4392c", values: pair.sub_report.rows.map((r) => r.lat_p99_us ?? 0) },
          ]}
        />
      </article>
    </section>
  {:else if isSinglePub && pubSlot}
    {@render single(pubSlot.report, pubSlot.fileName)}
  {:else if isSingleSub && subSlot}
    {@render single(subSlot.report, subSlot.fileName)}
  {/if}
</div>

{#snippet single(report: BenchReport, fileName: string)}
  {@const cfg = report.config}
  {@const s = report.summary}
  {@const isSub = report.kind === "sub"}

  <section class="meta">
    <div class="filebar">
      <span class="kind kind-{report.kind}">{report.kind.toUpperCase()}</span>
      <span class="filename">{fileName}</span>
    </div>
    <dl class="config">
      {#if cfg.topic}<dt>topic</dt><dd>{cfg.topic}</dd>{/if}
      {#if cfg.domain != null}<dt>domain</dt><dd>{cfg.domain}</dd>{/if}
      {#if cfg.rate != null}<dt>rate cible</dt><dd>{cfg.rate}/s</dd>{/if}
      {#if cfg.payload != null}<dt>payload</dt><dd>{cfg.payload} B</dd>{/if}
      {#if cfg.reliability}<dt>reliability</dt><dd>{cfg.reliability}</dd>{/if}
      {#if cfg.history_depth != null}<dt>history</dt><dd>KeepLast {cfg.history_depth}</dd>{/if}
      {#if cfg.duration != null}<dt>duration</dt><dd>{cfg.duration}s</dd>{/if}
      {#if cfg.warmup != null}<dt>warmup</dt><dd>{cfg.warmup}s</dd>{/if}
      {#if cfg.await_readers != null && cfg.await_readers > 0}
        <dt>await readers</dt><dd>{cfg.await_readers}</dd>
      {/if}
    </dl>
  </section>

  <VerdictBanner verdict={report.verdict} />

  <section class="metrics">
    <article class="card">
      <h3>Débit</h3>
      <p class="big">{s.mean_rate.toFixed(1)} <span class="unit">/s</span></p>
      <p class="sub">
        {s.total_samples.toLocaleString()} samples sur {s.duration_s.toFixed(1)}s
        (min {s.min_rate.toFixed(0)} / max {s.max_rate.toFixed(0)})
      </p>
      <LineChart
        xs={report.rows.map((r) => r.t_s)}
        series={[{ label: "", color: "#2c4f9c", values: report.rows.map((r) => r.rate_per_s) }]}
      />
    </article>

    {#if isSub}
      <article class="card">
        <h3>Latence (p50 / p95 / p99)</h3>
        <p class="big">{formatUs(s.mean_p50_us)} <span class="unit">médian</span></p>
        <p class="sub">
          p95 ≈ {formatUs(s.mean_p95_us)} · p99 max {formatUs(s.max_p99_us)} ·
          pire sample {formatUs(s.max_lat_us)}
        </p>
        <LineChart
          xs={report.rows.map((r) => r.t_s)}
          series={[
            { label: "p50", color: "#2ea44f", values: report.rows.map((r) => r.lat_p50_us ?? 0) },
            { label: "p95", color: "#d18b1f", values: report.rows.map((r) => r.lat_p95_us ?? 0) },
            { label: "p99", color: "#c4392c", values: report.rows.map((r) => r.lat_p99_us ?? 0) },
          ]}
        />
      </article>

      <article class="card">
        <h3>Pertes</h3>
        <p class="big">{s.total_lost ?? 0} <span class="unit">samples</span></p>
        <p class="sub">{((s.loss_rate ?? 0) * 100).toFixed(2)}% sur le total reçu+perdu</p>
        <LineChart
          xs={report.rows.map((r) => r.t_s)}
          series={[
            { label: "lost_wire (cumul.)", color: "#c4392c", values: report.rows.map((r) => r.lost_wire ?? 0) },
          ]}
        />
      </article>
    {:else}
      <article class="card">
        <h3>writer.write()</h3>
        <p class="big">{formatUs(s.mean_write_us)} <span class="unit">moyen</span></p>
        <p class="sub">pire write {formatUs(s.max_write_us)} (cumulatif)</p>
        <LineChart
          xs={report.rows.map((r) => r.t_s)}
          series={[
            { label: "write_avg", color: "#2c4f9c", values: report.rows.map((r) => r.write_avg_us ?? 0) },
            { label: "write_max", color: "#c4392c", values: report.rows.map((r) => r.write_max_us ?? 0) },
          ]}
        />
      </article>
    {/if}
  </section>

  <details class="raw">
    <summary>Données brutes ({report.rows.length} lignes)</summary>
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>t_s</th>
            <th>rate</th>
            {#if isSub}
              <th>recv</th><th>lost</th><th>p50</th><th>p95</th><th>p99</th><th>max</th>
            {:else}
              <th>sent</th><th>err</th><th>w_avg</th><th>w_max</th>
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each report.rows as r}
            <tr>
              <td>{r.t_s.toFixed(2)}</td>
              <td>{r.rate_per_s.toFixed(1)}</td>
              {#if isSub}
                <td>{r.recv}</td>
                <td class:bad={(r.lost_wire ?? 0) > 0}>{r.lost_wire}</td>
                <td>{formatUs(r.lat_p50_us)}</td>
                <td>{formatUs(r.lat_p95_us)}</td>
                <td>{formatUs(r.lat_p99_us)}</td>
                <td>{formatUs(r.lat_max_us)}</td>
              {:else}
                <td>{r.sent}</td>
                <td class:bad={(r.errors ?? 0) > 0}>{r.errors}</td>
                <td>{formatUs(r.write_avg_us)}</td>
                <td>{formatUs(r.write_max_us)}</td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </details>
{/snippet}

<style>
  .bench {
    padding: 1rem 1.25rem;
    max-width: 1100px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }
  .head h2 {
    margin: 0 0 0.25rem;
    font-size: 1.2rem;
    font-weight: 600;
  }
  .muted {
    margin: 0;
    color: #666;
    font-size: 0.85rem;
    max-width: 60ch;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.9rem;
    border-radius: 6px;
    background: #2c4f9c;
    color: #fff;
    cursor: pointer;
    font-size: 0.85rem;
    border: none;
    font: inherit;
  }
  .btn:hover { background: #25437f; }
  .btn.ghost {
    background: #f2f3f5;
    color: #555;
    border: 1px solid #e0e2e6;
  }
  .btn.ghost:hover { background: #e9eaed; }

  .slots {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .slot {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.65rem;
    background: #f6f7f9;
    border: 1px solid #e0e2e6;
    border-radius: 6px;
    font-size: 0.82rem;
  }
  .slot.filled {
    background: #fff;
  }
  .empty-hint {
    color: #999;
    font-style: italic;
    font-size: 0.78rem;
  }
  .x {
    background: none;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0 0.2rem;
  }
  .x:hover { color: #c4392c; }

  .empty {
    border: 1px dashed #d0d4d9;
    border-radius: 8px;
    padding: 1.5rem;
    text-align: center;
    color: #666;
    font-size: 0.9rem;
  }
  .empty pre {
    text-align: left;
    background: #f6f7f9;
    padding: 0.75rem;
    border-radius: 6px;
    overflow-x: auto;
    margin: 0.75rem auto;
    max-width: 540px;
    font-size: 0.78rem;
  }

  .err {
    border-radius: 6px;
    background: #fdf0ed;
    border: 1px solid #f1b0a7;
    color: #7a2218;
    padding: 0.7rem 0.9rem;
    font-size: 0.85rem;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .filebar {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .filename {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85rem;
    color: #555;
  }
  .kind {
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.05em;
  }
  .kind-pub { background: #eef3ff; color: #2c4f9c; }
  .kind-sub { background: #f1faf3; color: #2ea44f; }

  .config {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.25rem 0.75rem;
    margin: 0;
    font-size: 0.78rem;
  }
  .config dt {
    color: #888;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .config dd {
    margin: 0 0 0.25rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: #1a1a1a;
  }

  .metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 0.8rem;
  }
  .card {
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 8px;
    padding: 0.8rem 1rem;
  }
  .card h3 {
    margin: 0 0 0.4rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .card .big {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #1a1a1a;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .card .unit {
    font-size: 0.8rem;
    color: #888;
    font-weight: 400;
    font-family: -apple-system, sans-serif;
    margin-left: 0.25rem;
  }
  .card .sub {
    margin: 0.2rem 0 0.7rem;
    font-size: 0.78rem;
    color: #666;
  }

  .raw summary {
    cursor: pointer;
    color: #2c4f9c;
    font-size: 0.85rem;
    padding: 0.5rem 0;
  }
  .table-wrap {
    overflow-x: auto;
    max-height: 360px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.78rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  th, td {
    padding: 0.3rem 0.55rem;
    text-align: right;
    border-bottom: 1px solid #f0f1f3;
  }
  th {
    color: #666;
    font-weight: 500;
    background: #fafbfc;
    position: sticky;
    top: 0;
    z-index: 1;
  }
  td.bad {
    color: #c4392c;
    font-weight: 600;
  }
</style>
