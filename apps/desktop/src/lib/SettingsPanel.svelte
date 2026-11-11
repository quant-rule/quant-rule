<script lang="ts">
  let {
    workspaceRoot = $bindable(),
    memoryPath = $bindable(),
  }: {
    workspaceRoot: string;
    memoryPath: string;
  } = $props();
</script>

<div class="stack">
  <div class="card">
    <h2>Local paths</h2>
    <p class="muted">
      QuantRule keeps candles, configurations, run summaries, and the research
      memory on this machine. There is no account and no network request.
    </p>

    <label class="field">
      <span>Workspace root</span>
      <input bind:value={workspaceRoot} placeholder="/home/you/quant-rule-workspace" />
      <small class="muted">
        A directory with <code>config/</code> and <code>data/sources/</code>.
      </small>
    </label>

    <label class="field">
      <span>Research memory database</span>
      <input bind:value={memoryPath} placeholder="/home/you/quant-rule-workspace/research.db" />
      <small class="muted">
        A local SQLite file. The app creates it when you open the memory.
      </small>
    </label>
  </div>

  <div class="card">
    <h2>Boundary</h2>
    <ul class="muted">
      <li>No login, subscription, or cloud compute.</li>
      <li>No engine reads the clock or a network resource.</li>
      <li>The UI calls only the shared application service.</li>
      <li>Backtest, strategy, and analytics commands remain on the CLI.</li>
    </ul>
  </div>
</div>

<style>
  .field {
    display: grid;
    gap: 0.35rem;
    margin-top: 0.9rem;
  }

  .field > span {
    font-weight: 600;
  }

  .field input {
    width: 100%;
  }
</style>
