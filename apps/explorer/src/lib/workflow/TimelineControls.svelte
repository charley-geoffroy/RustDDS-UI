<script lang="ts">
  import type { Lang } from "../docs/chapters";
  import { PHASES, type Phase } from "./workflow-state";

  type Props = {
    phase: Phase;
    playing: boolean;
    speed: number;
    lang: Lang;
    onPhase: (p: Phase) => void;
    onPlay: () => void;
    onPause: () => void;
    onStep: () => void;
    onRestart: () => void;
    onSpeed: (s: number) => void;
  };
  let {
    phase,
    playing,
    speed,
    lang,
    onPhase,
    onPlay,
    onPause,
    onStep,
    onRestart,
    onSpeed,
  }: Props = $props();

  function tr(en: string, fr: string): string {
    return lang === "fr" ? fr : en;
  }
</script>

<div class="controls">
  <div class="buttons">
    <button
      onclick={onRestart}
      title={tr("Restart", "Redémarrer")}
      aria-label={tr("Restart", "Redémarrer")}
    >
      ⏮
    </button>
    {#if playing}
      <button
        onclick={onPause}
        title={tr("Pause", "Pause")}
        aria-label="Pause"
      >
        ⏸
      </button>
    {:else}
      <button
        onclick={onPlay}
        title={tr("Play", "Lecture")}
        aria-label="Play"
      >
        ▶
      </button>
    {/if}
    <button
      onclick={onStep}
      title={tr("Step forward", "Avancer d'une étape")}
      aria-label="Step"
    >
      ⏭
    </button>

    <div class="speed">
      {#each [0.5, 1, 2] as s (s)}
        <button
          class:active={speed === s}
          onclick={() => onSpeed(s)}
        >
          {s}×
        </button>
      {/each}
    </div>
  </div>

  <div class="phases" role="tablist" aria-label={tr("Workflow phases", "Phases du workflow")}>
    {#each PHASES as p (p.id)}
      <button
        role="tab"
        aria-selected={phase === p.id}
        class:active={phase === p.id}
        onclick={() => onPhase(p.id)}
      >
        {p.label[lang]}
      </button>
    {/each}
  </div>
</div>

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.5rem 0.7rem;
    background: #f7f8fb;
    border: 1px solid #eef0f4;
    border-radius: 6px;
    flex-wrap: wrap;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .buttons button {
    background: #fff;
    border: 1px solid #d8dbe1;
    border-radius: 5px;
    padding: 0.25rem 0.6rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
    line-height: 1.3;
    color: #444;
  }
  .buttons button:hover {
    border-color: #5577cc;
    color: #2c4f9c;
  }
  .speed {
    display: inline-flex;
    margin-left: 0.4rem;
    background: #fff;
    border: 1px solid #d8dbe1;
    border-radius: 5px;
    overflow: hidden;
  }
  .speed button {
    background: none;
    border: none;
    border-radius: 0;
    padding: 0.2rem 0.45rem;
    font-size: 0.78rem;
    color: #555;
    cursor: pointer;
    font: inherit;
  }
  .speed button.active {
    background: #5577cc;
    color: #fff;
  }
  .phases {
    display: inline-flex;
    margin-left: auto;
    gap: 2px;
    background: #fff;
    border: 1px solid #d8dbe1;
    border-radius: 5px;
    padding: 2px;
  }
  .phases button {
    background: none;
    border: none;
    padding: 0.25rem 0.6rem;
    font-size: 0.78rem;
    color: #555;
    cursor: pointer;
    border-radius: 3px;
    font: inherit;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .phases button:hover {
    color: #2c4f9c;
  }
  .phases button.active {
    background: #5577cc;
    color: #fff;
    font-weight: 500;
  }

  @media (prefers-color-scheme: dark) {
    .controls {
      background: #1a1d22;
      border-color: #2a2a2a;
    }
    .buttons button,
    .speed,
    .phases {
      background: #1f1f1f;
      border-color: #353d4a;
      color: #ccc;
    }
    .buttons button {
      color: #ccc;
    }
    .buttons button:hover {
      border-color: #5b7bd6;
      color: #a8c0ff;
    }
    .speed button {
      color: #aaa;
    }
    .speed button.active,
    .phases button.active {
      background: #5b7bd6;
    }
    .phases button {
      color: #aaa;
    }
    .phases button:hover {
      color: #a8c0ff;
    }
  }
</style>
