<script lang="ts">
  import { COMMANDS, SECTIONS } from "./capabilities";

  function screen(section: string): string {
    return SECTIONS.find((item) => item.id === section)?.label ?? section;
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Command reference</h2>
    <span class="badge">{COMMANDS.length} commands</span>
  </div>
  <p class="muted small">
    The command shell and this browser reach the same gateway contract. Each row
    names the command and the screen that reaches its use case here.
  </p>
  <table>
    <thead>
      <tr><th>Command</th><th>Screen</th><th>Control</th></tr>
    </thead>
    <tbody>
      {#each COMMANDS as item (item.command)}
        <tr>
          <td class="mono">{item.command}</td>
          <td>{screen(item.section)}</td>
          <td>{item.control}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</section>

<style>
  section {
    max-width: 70rem;
    gap: 0.8rem;
  }
</style>
