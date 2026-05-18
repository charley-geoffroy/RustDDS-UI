<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import type {
    EndpointDto,
    ParticipantDto,
    RegistrySnapshot,
    TopicDto,
  } from "$lib/types";
  import { isBuiltinTopic } from "$lib/builtin-topics";
  import Sidebar from "$lib/Sidebar.svelte";
  import TopicDetail from "$lib/TopicDetail.svelte";
  import EmptyState from "$lib/EmptyState.svelte";
  import DocsLayout from "$lib/DocsLayout.svelte";
  import { defaultChapter, DEFAULT_LANG, type Lang } from "$lib/docs/chapters";

  type VersionInfo = {
    app: string;
    backend_name: string;
    backend_version: string;
  };
  const DOMAIN_ID = 0;

  let version = $state<VersionInfo | null>(null);
  let error = $state<string | null>(null);
  let registry = $state<RegistrySnapshot>({
    participants: [],
    topics: [],
    writers: [],
    readers: [],
  });
  let selectedTopic = $state<TopicDto | null>(null);
  let searchQuery = $state("");
  let unlisteners: UnlistenFn[] = [];

  // Mode: explorer (default) or docs. Hash-synced.
  type Mode = "explorer" | "docs";
  let mode = $state<Mode>("explorer");
  let docSlug = $state<string>(defaultChapter().slug);
  let docLang = $state<Lang>(DEFAULT_LANG);

  const LANG_STORAGE_KEY = "rustdds_ui.doc_lang";

  function setDocLang(l: Lang) {
    docLang = l;
    try {
      localStorage.setItem(LANG_STORAGE_KEY, l);
    } catch (_) {
      /* private mode etc. */
    }
  }

  function parseHash(): { mode: Mode; slug: string } {
    const h = window.location.hash.replace(/^#\/?/, "");
    if (h.startsWith("docs")) {
      const slug = h.slice("docs".length).replace(/^\/?/, "") || defaultChapter().slug;
      return { mode: "docs", slug };
    }
    return { mode: "explorer", slug: defaultChapter().slug };
  }

  function writeHash(m: Mode, slug: string) {
    const target = m === "docs" ? `#docs/${slug}` : "";
    if (window.location.hash !== target) {
      // Use replaceState to avoid spamming browser history.
      history.replaceState(null, "", target || window.location.pathname);
    }
  }

  function switchMode(m: Mode) {
    mode = m;
    writeHash(m, docSlug);
  }

  function setDocSlug(slug: string) {
    docSlug = slug;
    if (mode === "docs") writeHash("docs", slug);
  }

  function upsert<T>(arr: T[], item: T, key: (x: T) => string): T[] {
    const k = key(item);
    const idx = arr.findIndex((x) => key(x) === k);
    if (idx >= 0) {
      const copy = arr.slice();
      copy[idx] = item;
      return copy;
    }
    return [...arr, item];
  }
  function removeBy<T>(arr: T[], key: (x: T) => string, value: string): T[] {
    return arr.filter((x) => key(x) !== value);
  }

  async function setup() {
    try {
      version = await invoke<VersionInfo>("get_version");
      registry = await invoke<RegistrySnapshot>("list_state");
    } catch (e) {
      error = String(e);
      return;
    }

    unlisteners.push(
      await listen<ParticipantDto>("dds:participant_added", (e) => {
        registry.participants = upsert(
          registry.participants,
          e.payload,
          (p) => p.guid,
        );
      }),
    );
    unlisteners.push(
      await listen<string>("dds:participant_removed", (e) => {
        registry.participants = removeBy(
          registry.participants,
          (p) => p.guid,
          e.payload,
        );
        registry.writers = registry.writers.filter(
          (w) => w.participant_guid !== e.payload,
        );
        registry.readers = registry.readers.filter(
          (r) => r.participant_guid !== e.payload,
        );
      }),
    );
    unlisteners.push(
      await listen<TopicDto>("dds:topic_added", (e) => {
        registry.topics = upsert(registry.topics, e.payload, (t) => t.name);
      }),
    );
    unlisteners.push(
      await listen<string>("dds:topic_removed", (e) => {
        registry.topics = removeBy(registry.topics, (t) => t.name, e.payload);
        if (selectedTopic?.name === e.payload) {
          selectedTopic = null;
        }
      }),
    );
    unlisteners.push(
      await listen<EndpointDto>("dds:writer_added", (e) => {
        registry.writers = upsert(registry.writers, e.payload, (w) => w.guid);
      }),
    );
    unlisteners.push(
      await listen<string>("dds:writer_removed", (e) => {
        registry.writers = removeBy(registry.writers, (w) => w.guid, e.payload);
      }),
    );
    unlisteners.push(
      await listen<EndpointDto>("dds:reader_added", (e) => {
        registry.readers = upsert(registry.readers, e.payload, (r) => r.guid);
      }),
    );
    unlisteners.push(
      await listen<string>("dds:reader_removed", (e) => {
        registry.readers = removeBy(registry.readers, (r) => r.guid, e.payload);
      }),
    );
  }

  onMount(() => {
    setup();

    // Honor #docs/slug if user opened the app with one.
    const initial = parseHash();
    mode = initial.mode;
    docSlug = initial.slug;

    // Restore the saved docs language if any.
    try {
      const saved = localStorage.getItem(LANG_STORAGE_KEY);
      if (saved === "en" || saved === "fr") docLang = saved;
    } catch (_) {
      /* ignore */
    }

    const onHash = () => {
      const h = parseHash();
      mode = h.mode;
      docSlug = h.slug;
    };
    window.addEventListener("hashchange", onHash);

    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const inEditable =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement;
      if (e.key === "Escape") {
        if (inEditable && target instanceof HTMLInputElement) {
          searchQuery = "";
          target.blur();
        } else if (mode === "explorer" && selectedTopic) {
          selectedTopic = null;
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("hashchange", onHash);
    };
  });

  onDestroy(() => {
    unlisteners.forEach((fn) => fn());
    unlisteners = [];
  });

  let userTopicCount = $derived(
    registry.topics.filter((t) => !isBuiltinTopic(t.name)).length,
  );

  function selectTopic(t: TopicDto) {
    selectedTopic = t;
  }
  function closeTopic() {
    selectedTopic = null;
  }
</script>

<header class="topbar">
  <div class="modes" role="tablist" aria-label="App mode">
    <button
      role="tab"
      aria-selected={mode === "explorer"}
      class:active={mode === "explorer"}
      onclick={() => switchMode("explorer")}
      title="Explorer (live DDS bus)"
    >
      <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
        <path d="M8 2.5a5.5 5.5 0 0 0-3.89 9.39.75.75 0 0 1-1.06 1.06A7 7 0 1 1 15 8a.75.75 0 0 1-1.5 0A5.5 5.5 0 0 0 8 2.5Zm0 3a2.5 2.5 0 0 0-2.5 2.5.75.75 0 0 1-1.5 0 4 4 0 1 1 4 4 .75.75 0 0 1 0-1.5A2.5 2.5 0 0 0 8 5.5Zm0 3.25a.75.75 0 1 1 0 1.5.75.75 0 0 1 0-1.5Z"/>
      </svg>
      <span>Explorer</span>
    </button>
    <button
      role="tab"
      aria-selected={mode === "docs"}
      class:active={mode === "docs"}
      onclick={() => switchMode("docs")}
      title="Documentation (DDS protocol + this app)"
    >
      <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
        <path d="M1.75 2A1.75 1.75 0 0 0 0 3.75v9.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-9.5A1.75 1.75 0 0 0 14.25 2H8.78a.25.25 0 0 0-.18.07L8 2.69l-.6-.62A.25.25 0 0 0 7.22 2H1.75ZM7.25 4.06v8.69a.25.25 0 0 1-.4.2A2.74 2.74 0 0 0 5.25 12.5H1.75a.25.25 0 0 1-.25-.25v-8.5a.25.25 0 0 1 .25-.25h5.25a.25.25 0 0 1 .25.25v.06ZM14.5 4.06v8.19a.25.25 0 0 1-.25.25h-3.5a2.74 2.74 0 0 0-1.6.45.25.25 0 0 1-.4-.2V4.06a.25.25 0 0 1 .25-.25H14.25a.25.25 0 0 1 .25.25Z"/>
      </svg>
      <span>Docs</span>
    </button>
  </div>

  <div class="brand">
    <strong>RustDDS_UI</strong>
    <span class="muted small">· domain {DOMAIN_ID}</span>
  </div>

  <div class="topbar-right">
    {#if version}
      <span
        class="backend-pill"
        title="DDS backend powering this session (see Docs → Backends)"
      >
        <span class="dot"></span>
        <strong>{version.backend_name}</strong>
        <span class="ver">{version.backend_version}</span>
      </span>
      <span class="muted small">v{version.app}</span>
    {:else}
      <span class="muted">loading…</span>
    {/if}
  </div>
</header>

{#if error}
  <p class="err">backend error: {error}</p>
{/if}

<main class="layout">
  {#if mode === "explorer"}
    <Sidebar
      {registry}
      {selectedTopic}
      {searchQuery}
      onSelect={selectTopic}
      onSearchChange={(q) => (searchQuery = q)}
    />

    <section class="main">
      {#if selectedTopic}
        <TopicDetail
          topic={selectedTopic}
          {registry}
          onClose={closeTopic}
        />
      {:else}
        <EmptyState {userTopicCount} />
      {/if}
    </section>
  {:else}
    <DocsLayout
      slug={docSlug}
      lang={docLang}
      onSlugChange={setDocSlug}
      onLangChange={setDocLang}
    />
  {/if}
</main>

<style>
  :global(:root) {
    font-family:
      -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: #1a1a1a;
    background-color: #fafafa;
  }
  :global(html, body) {
    margin: 0;
    height: 100%;
  }
  :global(body) {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .topbar {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 1rem;
    padding: 0.45rem 0.8rem;
    border-bottom: 1px solid #e5e5e5;
    background: #fff;
    flex-shrink: 0;
  }
  .brand {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
    justify-self: center;
    color: #888;
  }
  .brand strong {
    color: #1a1a1a;
    font-weight: 600;
    letter-spacing: -0.005em;
  }
  .modes {
    display: flex;
    gap: 2px;
    background: #f2f3f5;
    border-radius: 8px;
    padding: 3px;
    border: 1px solid #e0e2e6;
  }
  .modes button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: none;
    border: none;
    padding: 0.3rem 0.7rem;
    font-size: 0.85rem;
    color: #555;
    cursor: pointer;
    border-radius: 5px;
    font: inherit;
    line-height: 1.3;
    transition: background 0.12s, color 0.12s;
  }
  .modes button svg {
    opacity: 0.7;
    flex-shrink: 0;
  }
  .modes button:hover {
    color: #2c4f9c;
    background: rgba(255, 255, 255, 0.5);
  }
  .modes button.active {
    background: #fff;
    color: #1a1a1a;
    font-weight: 500;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.07),
      0 0 0 1px rgba(0, 0, 0, 0.03);
  }
  .modes button.active svg {
    opacity: 1;
    color: #5577cc;
  }
  .topbar-right {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .backend-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.18rem 0.55rem;
    background: #eef3ff;
    border: 1px solid #c8d6f0;
    border-radius: 999px;
    font-size: 0.78rem;
    color: #2c4f9c;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .backend-pill strong {
    font-weight: 600;
  }
  .backend-pill .ver {
    opacity: 0.7;
  }
  .backend-pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #5577cc;
    box-shadow: 0 0 4px rgba(85, 119, 204, 0.5);
  }
  .small {
    font-size: 0.85em;
  }
  .muted {
    color: #888;
  }
  .err {
    color: #c0392b;
    padding: 0.4rem 1rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    background: #fff3f0;
    border-bottom: 1px solid #f0c8c0;
    margin: 0;
  }
  .layout {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      color: #e5e5e5;
      background-color: #161616;
    }
    .topbar {
      background: #1f1f1f;
      border-bottom-color: #2a2a2a;
    }
    .err {
      background: #2a1818;
      border-bottom-color: #4a2a2a;
    }
    .modes {
      background: #1a1d22;
    }
    .modes button {
      color: #aaa;
    }
    .modes button:hover {
      color: #a8c0ff;
    }
    .modes button.active {
      background: #2c3340;
      color: #f0f0f0;
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
    }
    .backend-pill {
      background: #1a2030;
      border-color: #2d3a55;
      color: #a8c0ff;
    }
    .backend-pill .dot {
      background: #6e8cd6;
      box-shadow: 0 0 4px rgba(110, 140, 214, 0.7);
    }
  }
</style>
