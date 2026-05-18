<script lang="ts">
  type Props = { typeName: string };
  let { typeName }: Props = $props();
</script>

<div class="type-view">
  <h3>Type name</h3>
  <p>
    <code class="type-name">{typeName}</code>
  </p>

  <div class="banner">
    <div class="banner-head">
      <strong>Dynamic decoding</strong>
      <span class="phase-pill">
        <span class="dot"></span>
        Phase 4 · in progress
      </span>
    </div>
    <p class="muted small">
      Coming next — manual upload of <code>.idl</code> /
      <code>.msg</code> schemas to decode incoming CDR bytes into a JSON-like
      tree, instead of the hex dump shown in the Samples tab. Built-in
      fallback for common ROS 2 <code>std_msgs</code>.
    </p>
    <ul class="muted small roadmap">
      <li>📋 Schema upload UI (file picker)</li>
      <li>🔌 CDR → typed value decoder</li>
      <li>🌳 JSON tree renderer in the Samples tab</li>
      <li>📦 Built-in ROS 2 std_msgs schemas shipped with the app</li>
    </ul>
  </div>
</div>

<style>
  .type-view {
    padding: 0.8rem 1rem;
    overflow: auto;
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
    font-weight: 600;
  }
  .type-name {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.95em;
  }
  .banner {
    background: #eef3ff;
    border: 1px solid #c8d6f0;
    border-radius: 6px;
    padding: 0.7rem 0.9rem;
    margin-top: 1.5rem;
  }
  .banner-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.4rem;
  }
  .phase-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: #fff8e1;
    border: 1px solid #f0d77a;
    color: #6f4a00;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 500;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .phase-pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #ec9b00;
    box-shadow: 0 0 6px rgba(236, 155, 0, 0.5);
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
  .banner p {
    margin: 0.3rem 0 0;
  }
  .roadmap {
    margin: 0.7rem 0 0;
    padding-left: 0;
    list-style: none;
  }
  .roadmap li {
    padding: 0.15rem 0;
  }
  .muted {
    color: #555;
  }
  .small {
    font-size: 0.85em;
  }
  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  @media (prefers-color-scheme: dark) {
    h3 {
      color: #aaa;
    }
    .banner {
      background: #1a2030;
      border-color: #2d3a55;
      color: #c5d2ee;
    }
    .phase-pill {
      background: #2a2210;
      border-color: #4a3a15;
      color: #f5d480;
    }
    .phase-pill .dot {
      background: #f5b833;
    }
    .muted {
      color: #aab;
    }
  }
</style>
