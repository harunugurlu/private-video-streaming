<script lang="ts">
	import type HlsType from 'hls.js';

	let { hlsUrl }: { hlsUrl: string } = $props();

	let videoEl: HTMLVideoElement;
	let hlsInstance: HlsType | null = null;
	let playerError = $state('');

	$effect(() => {
		if (!videoEl || !hlsUrl) return;

		playerError = '';
		let destroyed = false;

		(async () => {
			const { default: Hls } = await import('hls.js');
			if (destroyed) return;

			if (Hls.isSupported()) {
				hlsInstance = new Hls({
					startLevel: -1,
					capLevelToPlayerSize: true
				});

				hlsInstance.on(Hls.Events.ERROR, (_event, data) => {
					if (data.fatal) {
						switch (data.type) {
							case Hls.ErrorTypes.NETWORK_ERROR:
								playerError = 'Network error — retrying...';
								hlsInstance?.startLoad();
								break;
							case Hls.ErrorTypes.MEDIA_ERROR:
								playerError = 'Media error — recovering...';
								hlsInstance?.recoverMediaError();
								break;
							default:
								playerError = 'Playback failed. Please refresh the page.';
								hlsInstance?.destroy();
								break;
						}
					}
				});

				hlsInstance.on(Hls.Events.MANIFEST_PARSED, () => {
					playerError = '';
				});

				hlsInstance.loadSource(hlsUrl);
				hlsInstance.attachMedia(videoEl);
			} else if (videoEl.canPlayType('application/vnd.apple.mpegurl')) {
				videoEl.src = hlsUrl;
			} else {
				playerError = 'Your browser does not support HLS playback.';
			}
		})();

		return () => {
			destroyed = true;
			if (hlsInstance) {
				hlsInstance.destroy();
				hlsInstance = null;
			}
		};
	});
</script>

<div class="player-wrapper">
	<!-- svelte-ignore a11y_media_has_caption -->
	<video
		bind:this={videoEl}
		controls
		playsinline
		class="video-element"
	></video>

	{#if playerError}
		<div class="player-error">
			<p>{playerError}</p>
		</div>
	{/if}
</div>

<style>
	.player-wrapper {
		position: relative;
		width: 100%;
		background: #000;
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.video-element {
		display: block;
		width: 100%;
		max-height: 70vh;
		background: #000;
	}

	.player-error {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		padding: 12px 16px;
		background: rgba(239, 68, 68, 0.9);
		color: white;
		font-size: 0.85rem;
		text-align: center;
	}
</style>
