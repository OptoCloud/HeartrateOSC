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

<div class="container m-auto p-4 flex flex-col justify-stretch items-stretch gap-10">
	<div class="card m-a p-8 flex flex-col justify-center items-center gap-4">
		<h1 class="h1 font-bold">
			Heart Rate: {heartRateMeasurement?.heart_rate}
		</h1>
		<h1 class="h1 font-semibold">
			RR Interval: {heartRateMeasurement ? smooth(heartRateMeasurement.rr_intervals) : 0}
		</h1>
	</div>
</div>