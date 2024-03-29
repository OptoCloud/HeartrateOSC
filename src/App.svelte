<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri"

  import Greet from './lib/Greet.svelte'
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  interface HeartRateMeasurement {
    heart_rate: number;
    sensor_contact_detected: boolean;
    sensor_contact_supported: boolean;
    energy_expended_present: boolean;
    energy_expended: number;
    rr_interval: Array<number>;
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
</script>

<main class="container">
  <h1>Welcome to Tauri!</h1>

  <div class="row">
    <a href="https://vitejs.dev" target="_blank">
      <img src="/vite.svg" class="logo vite" alt="Vite Logo" />
    </a>
    <a href="https://tauri.app" target="_blank">
      <img src="/tauri.svg" class="logo tauri" alt="Tauri Logo" />
    </a>
    <a href="https://svelte.dev" target="_blank">
      <img src="/svelte.svg" class="logo svelte" alt="Svelte Logo" />
    </a>
  </div>

  <p>
    Click on the Tauri, Vite, and Svelte logos to learn more.
  </p>
  <p>
    Heart Rate: {heartRateMeasurement?.heart_rate}
  </p>

  <div class="row">
    <Greet />
  </div>


</main>

<style>
  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.svelte:hover {
    filter: drop-shadow(0 0 2em #ff3e00);
  }
</style>