<script lang="ts">
	import '../app.postcss';
	import { computePosition, autoUpdate, flip, shift, offset, arrow } from '@floating-ui/dom';
	import { AppBar, AppRail, AppRailAnchor, AppShell, LightSwitch, Modal, Toast, initializeStores, storePopup } from '@skeletonlabs/skeleton';

	storePopup.set({ computePosition, autoUpdate, flip, shift, offset, arrow });
	initializeStores();
</script>

<Modal />
<Toast position="bl" max={5} />

<AppShell>
	<AppBar slot="header">
		<div slot="lead" class="flex items-center space-x-4">
			<!-- Logo -->
			<a
				href="/"
				class="overflow-hidden lg:!ml-0 lg:w-auto select-none"
				data-sveltekit-preload-data="hover"
			>
				<i class="fa-solid fa-heart pointer-events-none" />
				HeartRateOSC
			</a>
		</div>
		<svelte:fragment slot="trail">
			<LightSwitch />
			<div>
				<a href="https://github.com/OptoCloud/HeartRateOSC" class="btn-icon">
					<i class="fa-brands fa-github text-lg" />
				</a>
			</div>
			<a
				href="/settings"
				class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
			>
				Settings
			</a>
		</svelte:fragment>
	</AppBar>
	<slot />
	<div slot="pageFooter" class="m-2 flex items-center justify-center gap-1">
		Made with
		<span style="color: #e25555;">&#9829;</span>
		by OptoCloud
	</div>
</AppShell>