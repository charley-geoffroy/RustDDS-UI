<script lang="ts">
  import type { PairAnalysis } from "./bench-types";

  let { analysis }: { analysis: PairAnalysis } = $props();

  const a = $derived(analysis);
</script>

<article class="card">
  <h3>Comptes croisés pub ↔ sub</h3>

  <div class="grid">
    <div class="col">
      <span class="lbl">Pub envoyé</span>
      <span class="val">{a.pub_sent.toLocaleString()}</span>
    </div>
    <div class="col">
      <span class="lbl">Sub reçu</span>
      <span class="val">{a.sub_recv.toLocaleString()}</span>
    </div>
    <div class="col">
      <span class="lbl">Sub perdu</span>
      <span class="val" class:bad={a.sub_lost > 0}>{a.sub_lost.toLocaleString()}</span>
    </div>
    <div class="col col-delta">
      <span class="lbl">Delta</span>
      <span class="val">{a.delta >= 0 ? "+" : ""}{a.delta.toLocaleString()}</span>
    </div>
  </div>

  <p class="sub">
    Pub à <strong>{a.pub_rate.toFixed(1)}/s</strong>, sub à
    <strong>{a.sub_rate.toFixed(1)}/s</strong> (écart {a.rate_diff_pct.toFixed(1)}%).
    {#if a.expected_warmup_discard != null && a.expected_warmup_discard > 0}
      <br />
      Discard attendu pour le warmup du sub : ~{a.expected_warmup_discard.toLocaleString()} samples
      (= warmup × rate pub).
    {/if}
  </p>
</article>

<style>
  .card {
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 8px;
    padding: 0.9rem 1.1rem;
  }
  .card h3 {
    margin: 0 0 0.6rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.45rem 0.6rem;
    background: #fafbfc;
    border-radius: 6px;
    border: 1px solid #eef0f3;
  }
  .col-delta {
    background: #f0f5ff;
    border-color: #d6e2f5;
  }
  .lbl {
    font-size: 0.7rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .val {
    font-size: 1.05rem;
    font-weight: 600;
    color: #1a1a1a;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .val.bad { color: #c4392c; }
  .sub {
    margin: 0;
    font-size: 0.82rem;
    color: #555;
    line-height: 1.45;
  }
</style>
