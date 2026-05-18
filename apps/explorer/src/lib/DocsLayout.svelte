<script lang="ts">
  import type { Component } from "svelte";
  import {
    CHAPTERS,
    GROUPS,
    LANGS,
    LOADERS,
    findChapter,
    defaultChapter,
    type ChapterMeta,
    type GroupId,
    type Lang,
  } from "./docs/chapters";

  type Props = {
    slug: string;
    lang: Lang;
    onSlugChange: (slug: string) => void;
    onLangChange: (lang: Lang) => void;
  };
  let { slug, lang, onSlugChange, onLangChange }: Props = $props();

  let chapter = $derived<ChapterMeta>(findChapter(slug) ?? defaultChapter());

  let chapterComponent = $state<Component | null>(null);
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    loadComponent(chapter, lang);
  });

  async function loadComponent(c: ChapterMeta, l: Lang) {
    if (c.status !== "ready") {
      chapterComponent = null;
      loadError = null;
      loading = false;
      return;
    }
    const loaderByLang = LOADERS[c.slug];
    const loader = loaderByLang?.[l];
    if (!loader) {
      loadError = `No loader for chapter "${c.slug}" in language "${l}".`;
      chapterComponent = null;
      return;
    }
    loading = true;
    loadError = null;
    try {
      const mod = await loader();
      chapterComponent = mod.default;
    } catch (e) {
      loadError = `Failed to load chapter: ${String(e)}`;
      chapterComponent = null;
    } finally {
      loading = false;
    }
  }

  let searchQuery = $state("");
  let visibleByGroup = $derived(
    GROUPS.map((g) => ({
      group: g,
      chapters: CHAPTERS.filter(
        (c) =>
          c.group === g.id &&
          (!searchQuery ||
            c.title[lang].toLowerCase().includes(searchQuery.toLowerCase())),
      ),
    })).filter((g) => g.chapters.length > 0),
  );

  function pick(c: ChapterMeta) {
    onSlugChange(c.slug);
  }

  function groupLabel(id: GroupId): string {
    return GROUPS.find((g) => g.id === id)?.label[lang] ?? id;
  }
</script>

<div class="docs">
  <aside class="toc">
    <div class="toc-head">
      <input
        type="text"
        placeholder={lang === "fr" ? "Rechercher…" : "Search chapters…"}
        bind:value={searchQuery}
        spellcheck="false"
      />
      <div class="lang-toggle" role="group" aria-label="Documentation language">
        {#each LANGS as l (l.code)}
          <button
            class:active={lang === l.code}
            onclick={() => onLangChange(l.code)}
            title={l.label}
          >
            {l.flag}
          </button>
        {/each}
      </div>
    </div>
    <nav class="toc-list">
      {#each visibleByGroup as g (g.group.id)}
        <section>
          <h4>{g.group.label[lang]}</h4>
          {#each g.chapters as c (c.slug)}
            <button
              class="chapter-row"
              class:active={c.slug === chapter.slug}
              class:disabled={c.status !== "ready"}
              onclick={() => pick(c)}
            >
              <span class="title">{c.title[lang]}</span>
              <span class="meta">
                {#if c.status === "ready"}
                  <span class="time">{c.estimateMin} min</span>
                {:else}
                  <span class="badge">Phase {c.comingInPhase}</span>
                {/if}
              </span>
            </button>
          {/each}
        </section>
      {/each}
      {#if visibleByGroup.length === 0}
        <p class="muted small empty">
          {lang === "fr" ? "Aucun chapitre ne correspond." : "No chapter matches."}
        </p>
      {/if}
    </nav>
  </aside>

  <article class="content">
    <div class="prose">
      {#if chapter.status !== "ready"}
        <header class="ch-head">
          <h1>{chapter.title[lang]}</h1>
          <p class="muted">
            ~{chapter.estimateMin} min · {groupLabel(chapter.group)}
          </p>
        </header>
        <div class="placeholder">
          <p>
            🚧 {lang === "fr"
              ? "Ce chapitre arrive en"
              : "This chapter arrives in"}
            <strong>Phase {chapter.comingInPhase}</strong>.
          </p>
          <p class="muted small">
            {lang === "fr"
              ? "Le squelette est en place pour que la table des matières reste stable ; le contenu est écrit incrémentalement."
              : "The skeleton is in place so the table of contents stays stable; content is being written incrementally."}
          </p>
        </div>
      {:else if loading}
        <p class="muted">{lang === "fr" ? "Chargement…" : "Loading…"}</p>
      {:else if loadError}
        <p class="err">{loadError}</p>
      {:else if chapterComponent}
        {@const C = chapterComponent}
        <C />
      {/if}
    </div>
  </article>
</div>

<style>
  .docs {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
    background: #fafafa;
  }

  .toc {
    width: 280px;
    min-width: 280px;
    border-right: 1px solid #e5e5e5;
    background: #f5f6f8;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .toc-head {
    padding: 0.55rem 0.6rem;
    border-bottom: 1px solid #e5e5e5;
    background: #fff;
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .toc-head input {
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    padding: 0.35rem 0.6rem;
    border: 1px solid #d8dbe1;
    border-radius: 5px;
    font-size: 0.88rem;
    color: inherit;
    background: #fff;
    font: inherit;
  }
  .toc-head input:focus {
    outline: none;
    border-color: #5577cc;
    box-shadow: 0 0 0 2px rgba(85, 119, 204, 0.2);
  }
  .lang-toggle {
    display: flex;
    background: #f2f3f5;
    border: 1px solid #e0e2e6;
    border-radius: 5px;
    padding: 2px;
    gap: 2px;
  }
  .lang-toggle button {
    background: none;
    border: none;
    padding: 0.15rem 0.4rem;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.72rem;
    font-weight: 600;
    color: #777;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    letter-spacing: 0.04em;
    line-height: 1.4;
  }
  .lang-toggle button:hover {
    color: #2c4f9c;
  }
  .lang-toggle button.active {
    background: #fff;
    color: #1a1a1a;
    box-shadow: 0 1px 1px rgba(0, 0, 0, 0.08);
  }
  .toc-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0.3rem 0 1rem;
  }
  .toc-list h4 {
    margin: 0.6rem 0.8rem 0.25rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    color: #888;
  }
  .chapter-row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    padding: 0.4rem 0.8rem;
    cursor: pointer;
    border-left: 3px solid transparent;
    box-sizing: border-box;
  }
  .chapter-row:hover {
    background: #e8edf6;
  }
  .chapter-row.active {
    background: #dde6fb;
    border-left-color: #5577cc;
  }
  .chapter-row.disabled .title {
    color: #888;
  }
  .title {
    font-size: 0.88rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 0.72rem;
    color: #888;
    white-space: nowrap;
  }
  .badge {
    background: #ddd;
    color: #555;
    padding: 0.05em 0.45em;
    border-radius: 3px;
    font-weight: 500;
  }
  .time {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .empty {
    padding: 0.8rem;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 2rem 2.5rem;
    background: #fff;
  }
  .prose {
    max-width: 70ch;
    margin: 0 auto;
    line-height: 1.65;
  }

  .ch-head h1 {
    margin: 0 0 0.3rem;
    font-weight: 600;
    font-size: 1.8rem;
    letter-spacing: -0.01em;
  }
  .placeholder {
    margin-top: 2rem;
    padding: 1.5rem;
    background: #fff8e1;
    border: 1px solid #f0e3a8;
    border-radius: 6px;
  }
  .placeholder p {
    margin: 0;
  }
  .placeholder p + p {
    margin-top: 0.5rem;
  }

  /* Markdown content styles (mdsvex output renders inside .prose) */
  :global(.prose h1) {
    font-size: 1.8rem;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin: 0 0 1.2rem;
  }
  :global(.prose h2) {
    font-size: 1.3rem;
    font-weight: 600;
    margin: 2rem 0 0.6rem;
  }
  :global(.prose h3) {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 1.6rem 0 0.4rem;
  }
  :global(.prose p) {
    margin: 0 0 1rem;
  }
  :global(.prose ul),
  :global(.prose ol) {
    padding-left: 1.4rem;
    margin: 0 0 1rem;
  }
  :global(.prose li) {
    margin: 0.25rem 0;
  }
  :global(.prose code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.88em;
    background: #f3f4f7;
    padding: 0.1em 0.35em;
    border-radius: 3px;
  }
  :global(.prose pre) {
    background: #f7f8fb;
    border: 1px solid #eef0f4;
    border-radius: 6px;
    padding: 0.8rem 1rem;
    overflow-x: auto;
    margin: 0 0 1rem;
    line-height: 1.5;
  }
  :global(.prose pre code) {
    background: none;
    padding: 0;
  }
  :global(.prose table) {
    border-collapse: collapse;
    margin: 0 0 1.5rem;
    font-size: 0.92em;
  }
  :global(.prose th),
  :global(.prose td) {
    border: 1px solid #e5e5e5;
    padding: 0.4rem 0.7rem;
    text-align: left;
    vertical-align: top;
  }
  :global(.prose th) {
    background: #f5f6f8;
    font-weight: 600;
  }
  :global(.prose blockquote) {
    margin: 1rem 0;
    padding: 0.6rem 1rem;
    border-left: 3px solid #c8d6f0;
    background: #f4f7fd;
    color: #495367;
    border-radius: 0 5px 5px 0;
  }
  :global(.prose blockquote p) {
    margin: 0;
  }
  :global(.prose a) {
    color: #2c4f9c;
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 2px;
  }
  :global(.prose a:hover) {
    color: #5577cc;
  }
  :global(.prose kbd) {
    background: #eef0f4;
    border: 1px solid #d8dbe1;
    border-bottom-width: 2px;
    border-radius: 3px;
    padding: 0 0.4em;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    color: #444;
  }

  .muted {
    color: #888;
  }
  .small {
    font-size: 0.85em;
  }
  .err {
    color: #c0392b;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  @media (prefers-color-scheme: dark) {
    .docs {
      background: #161616;
    }
    .toc {
      background: #181a1f;
      border-right-color: #2a2a2a;
    }
    .toc-head {
      background: #1f1f1f;
      border-bottom-color: #2a2a2a;
    }
    .toc-head input {
      background: #161616;
      border-color: #353d4a;
      color: #ddd;
    }
    .toc-head input:focus {
      border-color: #5b7bd6;
      box-shadow: 0 0 0 2px rgba(91, 123, 214, 0.25);
    }
    .lang-toggle {
      background: #1a1d22;
      border-color: #2a2a2a;
    }
    .lang-toggle button {
      color: #aaa;
    }
    .lang-toggle button:hover {
      color: #a8c0ff;
    }
    .lang-toggle button.active {
      background: #2c3340;
      color: #f0f0f0;
    }
    .toc-list h4 {
      color: #888;
    }
    .chapter-row:hover {
      background: #232a36;
    }
    .chapter-row.active {
      background: #283449;
      border-left-color: #5b7bd6;
    }
    .badge {
      background: #2c3340;
      color: #ccc;
    }
    .content {
      background: #1a1a1a;
    }
    .placeholder {
      background: #2a2615;
      border-color: #4a3f1a;
      color: #e8dcaa;
    }
    :global(.prose code) {
      background: #1f242c;
    }
    :global(.prose pre) {
      background: #161616;
      border-color: #2a2a2a;
    }
    :global(.prose th) {
      background: #1f242c;
    }
    :global(.prose th),
    :global(.prose td) {
      border-color: #2a2a2a;
    }
    :global(.prose blockquote) {
      background: #1a2030;
      border-left-color: #3d4a63;
      color: #c5d2ee;
    }
    :global(.prose a) {
      color: #a8c0ff;
    }
    :global(.prose a:hover) {
      color: #c8d8ff;
    }
    :global(.prose kbd) {
      background: #1f242c;
      border-color: #353d4a;
      color: #ccc;
    }
  }
</style>
