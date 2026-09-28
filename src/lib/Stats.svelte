<script lang="ts">
  import { slotLabel, type SlotStats } from "./api";
  import { duration } from "./format";
  import Modal from "./Modal.svelte";

  // How work flows through each slot, over the done topics of the stats window.
  let { stats, days, onClose }: { stats: SlotStats[]; days: number; onClose: () => void } = $props();

  const percent = (share: number | null) => (share == null ? "—" : `${Math.round(share * 100)}%`);
</script>

<Modal title="Flow over the last {days} days" wide {onClose}>
  <table>
    <thead>
      <tr>
        <th></th>
        <th>Done</th>
        <th>Going</th>
        <th title="Median time from written down to done">Lead</th>
        <th title="Median time from started to done">Cycle</th>
        <th title="85% of the topics were done within">Cycle 85%</th>
        <th title="Share of the cycle time spent blocked">Blocked</th>
        <th title="Rework events, and topics that had some">Rework</th>
      </tr>
    </thead>
    <tbody>
      {#each stats as row (row.slot)}
        <tr>
          <th>{slotLabel(row.slot)}</th>
          <td>{row.done}</td>
          <td>{row.in_progress}</td>
          <td>{duration(row.lead_median)}</td>
          <td>{duration(row.cycle_median)}</td>
          <td>{duration(row.cycle_p85)}</td>
          <td>{percent(row.blocked_share)}</td>
          <td>{row.reworks}{row.reworked ? ` (${row.reworked})` : ""}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p>
    Lead time runs from when a topic is written down, cycle time from when it first takes its slot, both until
    it is done. A long wait between the two means topics sit in the backlog; much blocked time or rework is
    where to look for what to improve. The window is a setting (<kbd>⌘,</kbd>).
  </p>
</Modal>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
    font-size: 13px;
  }

  th,
  td {
    padding: 6px 8px;
    text-align: right;
    white-space: nowrap;
  }

  thead th {
    font-size: 11px;
    font-weight: 500;
    color: var(--overlay0);
  }

  tbody th {
    text-align: left;
  }

  tbody tr + tr {
    border-top: 1px solid var(--surface0);
  }

  p {
    margin: 14px 0 0;
    font-size: 12px;
    color: var(--subtext);
  }
</style>
