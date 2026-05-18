<script lang="ts">
  import type { Lang } from "../docs/chapters";

  export type CdrField = {
    /** byte offset in the trace (after the optional encapsulation header) */
    offset: number;
    /** byte length */
    length: number;
    /** field name (e.g. `publisher_id` or `(padding)`) */
    name: string;
    /** displayed type (e.g. `u32`, `string`, `—`) */
    type: string;
    /** decoded value, displayed as-is */
    value: string;
    /** optional explanation */
    note?: { en: string; fr: string };
  };

  type Props = {
    hex: string;
    fields: CdrField[];
    lang?: Lang;
    /** caption above the hex (e.g. "Chatter sample") */
    title?: { en: string; fr: string };
  };

  let { hex, fields, lang = "en", title }: Props = $props();

  // Normalize: strip all whitespace, lowercase
  const clean = $derived(hex.replace(/\s+/g, "").toLowerCase());
  const bytes = $derived(
    Array.from({ length: clean.length / 2 }, (_, i) =>
      clean.slice(i * 2, i * 2 + 2),
    ),
  );

  let selected = $state<number | null>(null);

  function fieldIndexAt(byteOffset: number): number | null {
    const idx = fields.findIndex(
      (f) => byteOffset >= f.offset && byteOffset < f.offset + f.length,
    );
    return idx >= 0 ? idx : null;
  }

  // Distinct, color-blind-tolerant palette
  const palette = [
    "#fde68a", // amber
    "#bfdbfe", // blue
    "#bbf7d0", // green
    "#fbcfe8", // pink
    "#fed7aa", // orange
    "#c7d2fe", // indigo
    "#fef08a", // yellow
    "#e9d5ff", // purple
    "#a5f3fc", // cyan
    "#fecaca", // red
  ];

  function colorOf(fieldIdx: number | null): string {
    return fieldIdx == null ? "transparent" : palette[fieldIdx % palette.length];
  }

  function tr(en: string, fr: string): string {
    return lang === "fr" ? fr : en;
  }

  function bytesOf(f: CdrField): string {
    return bytes.slice(f.offset, f.offset + f.length).join(" ");
  }

  const BYTES_PER_LINE = 16;
</script>

<div class="decoder">
  {#if title}
    <div class="title">{title[lang]}</div>
  {/if}

  <pre class="hex">{#each bytes as b, i}{@const fi = fieldIndexAt(i)}{#if fi != null}<button
          type="button"
          class="byte interactive"
          class:active={selected === fi}
          style:background={colorOf(fi)}
          onclick={() => (selected = fi)}
          title={fields[fi].name}
        >{b}</button>{:else}<span class="byte">{b}</span>{/if}{#if (i + 1) % BYTES_PER_LINE === 0}{"\n"}{:else}{" "}{/if}{/each}</pre>

  <div class="legend">
    {#each fields as f, i (i)}
      <button
        type="button"
        class="chip"
        class:active={selected === i}
        onclick={() => (selected = i)}
      >
        <span class="swatch" style:background={colorOf(i)}></span>
        <span class="name">{f.name}</span>
        <code class="type">{f.type}</code>
      </button>
    {/each}
  </div>

  {#if selected != null}
    {@const f = fields[selected]}
    <div class="detail">
      <header>
        <span class="swatch" style:background={colorOf(selected)}></span>
        <strong>{f.name}</strong>
        <code class="type">{f.type}</code>
      </header>
      <table>
        <tbody>
          <tr>
            <th>{tr("Offset", "Offset")}</th>
            <td>{f.offset}</td>
          </tr>
          <tr>
            <th>{tr("Length", "Longueur")}</th>
            <td>{f.length} {tr("bytes", "bytes")}</td>
          </tr>
          <tr>
            <th>{tr("Bytes", "Bytes")}</th>
            <td><code>{bytesOf(f)}</code></td>
          </tr>
          <tr>
            <th>{tr("Value", "Valeur")}</th>
            <td><code>{f.value}</code></td>
          </tr>
        </tbody>
      </table>
      {#if f.note}
        <p class="note">{f.note[lang]}</p>
      {/if}
    </div>
  {:else}
    <p class="hint muted small">
      {tr(
        "Click a byte or a chip below to decode it.",
        "Click un byte ou un chip ci-dessous pour le décoder.",
      )}
    </p>
  {/if}
</div>

<style>
  .decoder {
    margin: 1rem 0;
    background: #fff;
    border: 1px solid #e5e5e5;
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
  }
  .title {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
    font-weight: 600;
    margin-bottom: 0.4rem;
  }
  .hex {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.92em;
    line-height: 1.6;
    background: #f7f8fb;
    border: 1px solid #eef0f4;
    border-radius: 4px;
    padding: 0.55rem 0.7rem;
    margin: 0 0 0.6rem;
    overflow-x: auto;
    white-space: pre;
    color: #333;
  }
  .byte {
    display: inline-block;
    padding: 0.05em 0.15em;
    border-radius: 3px;
    color: #333;
    border: none;
    font: inherit;
    line-height: inherit;
  }
  .byte.interactive {
    cursor: pointer;
  }
  .byte.interactive:hover {
    outline: 1px solid #555;
  }
  .byte.active {
    outline: 2px solid #1a1a1a;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.6rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.18rem 0.55rem;
    background: #f7f8fb;
    border: 1px solid #e5e5e5;
    border-radius: 999px;
    font: inherit;
    font-size: 0.78rem;
    cursor: pointer;
    color: #333;
  }
  .chip:hover {
    border-color: #5577cc;
  }
  .chip.active {
    border-color: #1a1a1a;
    background: #fff;
    box-shadow: 0 0 0 2px rgba(85, 119, 204, 0.2);
  }
  .swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    border: 1px solid rgba(0, 0, 0, 0.15);
    flex-shrink: 0;
  }
  .type {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    color: #777;
  }
  .name {
    font-weight: 500;
  }
  .detail {
    background: #f7f8fb;
    border: 1px solid #eef0f4;
    border-radius: 5px;
    padding: 0.55rem 0.7rem;
  }
  .detail header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  .detail table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
    margin-bottom: 0.35rem;
  }
  .detail th,
  .detail td {
    text-align: left;
    padding: 0.2rem 0.4rem;
    border-bottom: 1px solid #eef0f4;
    vertical-align: top;
  }
  .detail th {
    color: #555;
    font-weight: 500;
    width: 6em;
  }
  .detail code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.92em;
  }
  .note {
    margin: 0.3rem 0 0;
    color: #555;
    font-size: 0.85rem;
    line-height: 1.5;
  }
  .hint {
    margin: 0;
  }
  .muted {
    color: #777;
  }
  .small {
    font-size: 0.85em;
  }

  @media (prefers-color-scheme: dark) {
    .decoder {
      background: #1f1f1f;
      border-color: #333;
    }
    .title {
      color: #aaa;
    }
    .hex {
      background: #161616;
      border-color: #2a2a2a;
      color: #ddd;
    }
    .byte {
      color: #1a1a1a;
    }
    .byte.interactive:hover {
      outline-color: #ccc;
    }
    .byte.active {
      outline-color: #fff;
    }
    .chip {
      background: #161616;
      border-color: #2a2a2a;
      color: #ddd;
    }
    .chip:hover {
      border-color: #5b7bd6;
    }
    .chip.active {
      background: #1f242c;
      border-color: #fff;
    }
    .detail {
      background: #161616;
      border-color: #2a2a2a;
    }
    .detail th {
      color: #aaa;
    }
    .detail th,
    .detail td {
      border-bottom-color: #2a2a2a;
    }
    .note {
      color: #aaa;
    }
    .muted {
      color: #999;
    }
    .swatch {
      border-color: rgba(255, 255, 255, 0.2);
    }
  }
</style>
