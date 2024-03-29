<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri"

  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  interface HeartRateMeasurement {
    heart_rate: number;
    sensor_contact_detected: boolean;
    sensor_contact_supported: boolean;
    energy_expended_present: boolean;
    energy_expended: number;
    rr_intervals: Array<number>;
  }

  let heartRateMeasurement: HeartRateMeasurement | null = null;

  let unlistenFn: UnlistenFn | null = null;
  
  onMount(async () => {
    unlistenFn = await listen("heartRateMeasurement", (e) => heartRateMeasurement = e.payload as HeartRateMeasurement);

    let resp = await invoke("bluetooth_init");
  })

  onDestroy(() => {
    if (unlistenFn) {
      unlistenFn();
    }
  })

  function smooth(arr: Array<number>) {
    let sum = 0;
    for (let i = 0; i < arr.length; i++) {
      sum += arr[i];
    }
    return sum / arr.length;
  }
</script>

<main class="container">
  <h1>
    Heart Rate: {heartRateMeasurement?.heart_rate}
  </h1>
  <h1>
    RR Interval: {heartRateMeasurement ? smooth(heartRateMeasurement.rr_intervals) : 0}
  </h1>
</main>