<script lang="ts">
  import type { ParticipantDto, RegistrySnapshot, TopicDto } from "./types";
  import {
    builtinTopicDescription,
    isBuiltinTopic,
  } from "./builtin-topics";

  type Props = {
    registry: RegistrySnapshot;
    selectedTopic: TopicDto | null;
    searchQuery: string;
    onSelect: (t: TopicDto) => void;
    onSearchChange: (q: string) => void;
  };
  let {
    registry,
    selectedTopic,
    searchQuery,
    onSelect,
    onSearchChange,
  }: Props = $props();

  let showSystem = $state(false);
  let showParticipants = $state(true);
  let searchInput: HTMLInputElement | undefined;

  // Global keyboard shortcuts: "/" focuses search.
  $effect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const inEditable =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement;
      if (e.key === "/" && !inEditable) {
        e.preventDefault();
        searchInput?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  function matchesTopic(t: TopicDto): boolean {
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      t.name.toLowerCase().includes(q) ||
      t.type_name.toLowerCase().includes(q)
    );
  }

  function matchesParticipant(p: ParticipantDto): boolean {
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      (p.entity_name?.toLowerCase().includes(q) ?? false) ||
      p.guid.toLowerCase().includes(q)
    );
  }

  let userTopics = $derived(
    registry.topics
      .filter((t) => !isBuiltinTopic(t.name) && matchesTopic(t))
      .sort((a, b) => a.name.localeCompare(b.name)),
  );
  let systemTopics = $derived(
    registry.topics
      .filter((t) => isBuiltinTopic(t.name) && matchesTopic(t))
      .sort((a, b) => a.name.localeCompare(b.name)),
  );
  let visibleParticipants = $derived(
    registry.participants
      .filter(matchesParticipant)
      .sort((a, b) =>
        (a.entity_name ?? a.guid).localeCompare(b.entity_name ?? b.guid),
      ),
  );

  function topicCounts(name: string) {
    const w = registry.writers.filter((x) => x.topic_name === name).length;
    const r = registry.readers.filter((x) => x.topic_name === name).length;
    return { w, r };
  }
</script>

<aside class="sidebar">
  <div class="search-row">
    <input
      bind:this={searchInput}
      value={searchQuery}
      oninput={(e) => onSearchChange((e.target as HTMLInputElement).value)}
      placeholder="Search… (/)"
      type="text"
      spellcheck="false"
      autocomplete="off"
    />
    {#if searchQuery}
      <button
        class="clear"
        onclick={() => onSearchChange("")}
        title="Clear"
        aria-label="Clear search"
      >
        ×
      </button>
    {/if}
  </div>

  <nav class="sections">
    <section>
      <div class="section-header static">
        User topics <span class="count">{userTopics.length}</span>
      </div>
      {#if userTopics.length === 0}
        <p class="empty muted small">
          {searchQuery ? "No match." : "Waiting for traffic…"}
        </p>
      {/if}
      {#each userTopics as t (t.name)}
        {@const c = topicCounts(t.name)}
        <button
          class="topic-row"
          class:active={selectedTopic?.name === t.name}
          onclick={() => onSelect(t)}
        >
          <div class="topic-main">
            <span class="topic-name">{t.name}</span>
            <span class="topic-type muted small">{t.type_name}</span>
          </div>
          <span class="endpoints" title="{c.w} writers, {c.r} readers">
            {c.w}↑ {c.r}↓
          </span>
        </button>
      {/each}
    </section>

    <section>
      <button
        class="section-header toggle"
        onclick={() => (showSystem = !showSystem)}
      >
        <span class="caret">{showSystem ? "▾" : "▸"}</span>
        System
        <span class="count">{systemTopics.length}</span>
      </button>
      {#if showSystem}
        {#each systemTopics as t (t.name)}
          {@const c = topicCounts(t.name)}
          <button
            class="topic-row builtin"
            class:active={selectedTopic?.name === t.name}
            onclick={() => onSelect(t)}
            title={builtinTopicDescription(t.name) ?? ""}
          >
            <div class="topic-main">
              <span class="topic-name">{t.name}</span>
              <span class="topic-type muted small">{t.type_name}</span>
            </div>
            <span class="endpoints" title="{c.w} writers, {c.r} readers">
              {c.w}↑ {c.r}↓
            </span>
          </button>
        {/each}
      {/if}
    </section>

    <section>
      <button
        class="section-header toggle"
        onclick={() => (showParticipants = !showParticipants)}
      >
        <span class="caret">{showParticipants ? "▾" : "▸"}</span>
        Participants
        <span class="count">{visibleParticipants.length}</span>
      </button>
      {#if showParticipants}
        {#each visibleParticipants as p (p.guid)}
          <div class="participant-row">
            <div class="topic-main">
              <span class="topic-name">
                {p.entity_name ?? "(unnamed)"}
              </span>
              <span class="topic-type muted small">{p.vendor_id}</span>
            </div>
            <code class="muted small guid">{p.guid.slice(0, 8)}…</code>
          </div>
        {/each}
      {/if}
    </section>
  </nav>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 300px;
    min-width: 300px;
    border-right: 1px solid #e5e5e5;
    background: #fafafa;
    overflow: hidden;
  }
  .search-row {
    position: relative;
    padding: 0.55rem 0.6rem;
    border-bottom: 1px solid #eee;
    background: #fff;
  }
  .search-row input {
    width: 100%;
    padding: 0.35rem 1.6rem 0.35rem 0.6rem;
    border: 1px solid #d8dbe1;
    border-radius: 5px;
    font-size: 0.88rem;
    background: #fff;
    color: inherit;
    box-sizing: border-box;
  }
  .search-row input:focus {
    outline: none;
    border-color: #5577cc;
    box-shadow: 0 0 0 2px rgba(85, 119, 204, 0.2);
  }
  .clear {
    position: absolute;
    right: 0.95rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    font-size: 1.1em;
    cursor: pointer;
    color: #888;
    padding: 0 0.2rem;
  }
  .clear:hover {
    color: #000;
  }
  .sections {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0.3rem 0;
  }
  .section-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    width: 100%;
    padding: 0.4rem 0.8rem;
    background: none;
    border: none;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #666;
    cursor: pointer;
  }
  .section-header.static {
    cursor: default;
  }
  .section-header.toggle:hover {
    color: #2c4f9c;
  }
  .caret {
    font-size: 0.6em;
    display: inline-block;
    width: 0.8em;
    color: #999;
  }
  .count {
    margin-left: auto;
    background: #e8eaee;
    color: #555;
    border-radius: 999px;
    padding: 0 0.5em;
    font-size: 0.95em;
    font-weight: 500;
  }
  .topic-row,
  .participant-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.6rem;
    align-items: center;
    width: 100%;
    padding: 0.35rem 0.8rem;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
    border-left: 3px solid transparent;
    box-sizing: border-box;
  }
  .topic-row:hover {
    background: #eef3ff;
  }
  .topic-row.active {
    background: #dde6fb;
    border-left-color: #5577cc;
  }
  .topic-row.builtin {
    opacity: 0.85;
  }
  .topic-main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .topic-name {
    display: block;
    min-width: 0;
    max-width: 100%;
    font-size: 0.88rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .topic-type {
    display: block;
    min-width: 0;
    max-width: 100%;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .endpoints {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.75em;
    color: #888;
    white-space: nowrap;
  }
  .participant-row {
    cursor: default;
  }
  .guid {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .empty {
    padding: 0.3rem 0.9rem 0.6rem;
  }
  .muted {
    color: #888;
  }
  .small {
    font-size: 0.85em;
  }

  @media (prefers-color-scheme: dark) {
    .sidebar {
      background: #181a1f;
      border-right-color: #2a2a2a;
    }
    .search-row {
      background: #1f1f1f;
      border-bottom-color: #2a2a2a;
    }
    .search-row input {
      background: #161616;
      border-color: #353d4a;
      color: #ddd;
    }
    .search-row input:focus {
      border-color: #5b7bd6;
      box-shadow: 0 0 0 2px rgba(91, 123, 214, 0.25);
    }
    .clear {
      color: #aaa;
    }
    .clear:hover {
      color: #fff;
    }
    .section-header {
      color: #999;
    }
    .section-header.toggle:hover {
      color: #a8c0ff;
    }
    .caret {
      color: #777;
    }
    .count {
      background: #2c3340;
      color: #ccc;
    }
    .topic-row:hover {
      background: #232a36;
    }
    .topic-row.active {
      background: #283449;
      border-left-color: #5b7bd6;
    }
    .endpoints {
      color: #999;
    }
  }
</style>
