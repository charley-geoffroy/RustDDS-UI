<script lang="ts">
  // Tiny line chart in plain SVG. Multiple series share an x axis (t_s).
  // No external chart lib — keeps the bundle small and matches the rest
  // of the explorer's "everything in-house" style.

  type Series = {
    label: string;
    color: string;
    values: number[];
  };

  let {
    xs,
    series,
    yLabel = "",
    height = 140,
  }: {
    xs: number[];
    series: Series[];
    yLabel?: string;
    height?: number;
  } = $props();

  const PAD_LEFT = 36;
  const PAD_RIGHT = 8;
  const PAD_TOP = 8;
  const PAD_BOTTOM = 20;
  const WIDTH = 480; // SVG viewBox width; CSS makes it responsive.

  const yMax = $derived(
    Math.max(1, ...series.flatMap((s) => s.values).filter((v) => Number.isFinite(v))),
  );
  const xMin = $derived(xs[0] ?? 0);
  const xMax = $derived(xs[xs.length - 1] ?? 1);
  const xRange = $derived(Math.max(0.001, xMax - xMin));

  function xPx(t: number): number {
    return PAD_LEFT + ((t - xMin) / xRange) * (WIDTH - PAD_LEFT - PAD_RIGHT);
  }
  function yPx(v: number): number {
    const innerH = height - PAD_TOP - PAD_BOTTOM;
    return PAD_TOP + innerH - (v / yMax) * innerH;
  }
  function path(values: number[]): string {
    if (values.length === 0) return "";
    return values
      .map((v, i) => `${i === 0 ? "M" : "L"}${xPx(xs[i]).toFixed(1)},${yPx(v).toFixed(1)}`)
      .join(" ");
  }

  // y-axis ticks: 0, mid, max
  const yTicks = $derived([0, yMax / 2, yMax]);
  // x-axis ticks: a few evenly spaced
  const xTicks = $derived.by(() => {
    const n = 4;
    const out: number[] = [];
    for (let i = 0; i <= n; i++) out.push(xMin + (xRange * i) / n);
    return out;
  });

  function fmtY(v: number): string {
    if (v >= 1000) return `${(v / 1000).toFixed(1)}k`;
    if (v >= 10) return v.toFixed(0);
    return v.toFixed(1);
  }
</script>

<div class="wrap">
  <svg viewBox="0 0 {WIDTH} {height}" preserveAspectRatio="none" role="img" aria-label={yLabel}>
    <!-- Grid -->
    {#each yTicks as t}
      <line
        x1={PAD_LEFT}
        y1={yPx(t)}
        x2={WIDTH - PAD_RIGHT}
        y2={yPx(t)}
        stroke="#eee"
        stroke-width="1"
      />
      <text x={PAD_LEFT - 4} y={yPx(t) + 3} font-size="9" fill="#888" text-anchor="end">
        {fmtY(t)}
      </text>
    {/each}
    {#each xTicks as t}
      <text
        x={xPx(t)}
        y={height - 6}
        font-size="9"
        fill="#888"
        text-anchor="middle"
      >
        {t.toFixed(0)}s
      </text>
    {/each}

    <!-- Series -->
    {#each series as s}
      <path d={path(s.values)} fill="none" stroke={s.color} stroke-width="1.5" />
    {/each}
  </svg>

  <!-- Legend -->
  {#if series.length > 1 || (series.length === 1 && series[0].label !== "")}
    <div class="legend">
      {#each series as s}
        <span class="item">
          <span class="dot" style="background:{s.color}"></span>
          {s.label}
        </span>
      {/each}
    </div>
  {/if}
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  svg {
    width: 100%;
    height: auto;
    display: block;
  }
  .legend {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    font-size: 0.75rem;
    color: #666;
  }
  .item {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
</style>
