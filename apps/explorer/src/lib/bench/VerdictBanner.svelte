<script lang="ts">
  import type { BenchVerdict, Severity } from "./bench-types";

  let { verdict }: { verdict: BenchVerdict } = $props();

  const ICON: Record<Severity, string> = {
    ok: "✓",
    warn: "⚠",
    bad: "✗",
  };
</script>

<div class="banner sev-{verdict.severity}">
  <div class="hero">
    <span class="icon">{ICON[verdict.severity]}</span>
    <strong>{verdict.headline}</strong>
  </div>
  <ul class="checks">
    {#each verdict.checks as c}
      <li class="check sev-{c.severity}">
        <div class="row">
          <span class="dot">{ICON[c.severity]}</span>
          <span class="label">{c.label}</span>
          <span class="value">{c.value}</span>
        </div>
        <p class="explain">{c.explain}</p>
      </li>
    {/each}
  </ul>
</div>

<style>
  .banner {
    border: 1px solid;
    border-radius: 10px;
    padding: 1rem 1.1rem;
    background: #fff;
  }
  .banner.sev-ok {
    border-color: #b9e0c2;
    background: #f1faf3;
  }
  .banner.sev-warn {
    border-color: #f3d597;
    background: #fdf6e6;
  }
  .banner.sev-bad {
    border-color: #f1b0a7;
    background: #fdf0ed;
  }

  .hero {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-size: 1.05rem;
  }
  .hero strong {
    font-weight: 600;
    letter-spacing: -0.005em;
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 700;
  }
  .sev-ok .icon { background: #2ea44f; }
  .sev-warn .icon { background: #d18b1f; }
  .sev-bad .icon { background: #c4392c; }

  .checks {
    list-style: none;
    margin: 0.8rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.55rem;
  }
  .check {
    background: rgba(255, 255, 255, 0.6);
    border-radius: 6px;
    padding: 0.5rem 0.7rem;
    border-left: 3px solid transparent;
  }
  .check.sev-ok    { border-left-color: #2ea44f; }
  .check.sev-warn  { border-left-color: #d18b1f; }
  .check.sev-bad   { border-left-color: #c4392c; }

  .row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    font-size: 0.88rem;
  }
  .row .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: none;
    color: inherit;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
  }
  .check.sev-ok    .row .dot { color: #2ea44f; }
  .check.sev-warn  .row .dot { color: #d18b1f; }
  .check.sev-bad   .row .dot { color: #c4392c; }

  .label {
    font-weight: 500;
    color: #1a1a1a;
  }
  .value {
    margin-left: auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.82rem;
    color: #444;
  }
  .explain {
    margin: 0.35rem 0 0;
    padding-left: 1.5rem;
    font-size: 0.82rem;
    color: #555;
    line-height: 1.45;
  }
</style>
