<script lang="ts">
	import { page } from '$app/state';
	import { api, ApiRequestError } from '$lib/api';
	import type { ShareResponse } from '$lib/types';
	import VideoPlayer from '$lib/components/VideoPlayer.svelte';
	import ProcessingProgress from '$lib/components/ProcessingProgress.svelte';

	let token = $derived(page.params.token ?? '');
	let video: ShareResponse | null = $state(null);
	let pageError = $state('');
	let notFound = $state(false);
	let pollTimer: ReturnType<typeof setInterval> | null = null;

	async function fetchStatus() {
		try {
			video = await api.getShare(token);
			pageError = '';

			if (video.status === 'playable' || video.status === 'failed') {
				stopPolling();
			}
		} catch (err) {
			if (err instanceof ApiRequestError && err.status === 404) {
				notFound = true;
				stopPolling();
			} else {
				pageError = 'Failed to load video status. Retrying...';
			}
		}
	}

	function startPolling() {
		fetchStatus();
		pollTimer = setInterval(fetchStatus, 2500);
	}

	function stopPolling() {
		if (pollTimer) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
	}

	$effect(() => {
		if (token) {
			startPolling();
		}
		return () => stopPolling();
	});

	function copyLink() {
		navigator.clipboard.writeText(window.location.href);
		copied = true;
		setTimeout(() => { copied = false; }, 2000);
	}

	let copied = $state(false);

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head>
	<title>{video?.title ?? 'Video'} — StreamDrop</title>
</svelte:head>

<div class="share-page fade-in">
	{#if notFound}
		<div class="state-card card">
			<div class="state-icon error-icon">
				<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
					<circle cx="12" cy="12" r="10" />
					<line x1="15" y1="9" x2="9" y2="15" />
					<line x1="9" y1="9" x2="15" y2="15" />
				</svg>
			</div>
			<h2>Video not found</h2>
			<p class="state-hint">This link may be invalid or the video may have been removed.</p>
			<a href="/upload" class="btn-primary">Upload a new video</a>
		</div>
	{:else if !video}
		<div class="state-card card">
			<div class="loading-spinner"></div>
			<p class="state-hint">Loading...</p>
		</div>
	{:else if video.status === 'uploading'}
		<div class="video-container">
			<div class="video-header">
				<h1 class="video-title">{video.title}</h1>
				<p class="video-meta">Uploaded {formatDate(video.created_at)}</p>
			</div>

			<div class="state-card card">
				<div class="state-icon upload-icon">
					<svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
						<polyline points="17 8 12 3 7 8" />
						<line x1="12" y1="3" x2="12" y2="15" />
					</svg>
				</div>
				<h2>Upload in progress</h2>
				<p class="state-hint">The video is still being uploaded. This page will update automatically.</p>
				<div class="pulse-bar">
					<div class="pulse-fill"></div>
				</div>
			</div>
		</div>
	{:else if video.status === 'processing'}
		<div class="video-container">
			<div class="video-header">
				<h1 class="video-title">{video.title}</h1>
				<p class="video-meta">Uploaded {formatDate(video.created_at)}</p>
			</div>

			<ProcessingProgress status={video.status} stage={video.processing_stage} />
		</div>
	{:else if video.status === 'playable' && video.playback.hls_url}
		<div class="video-container">
			<VideoPlayer hlsUrl={video.playback.hls_url} />

			<div class="video-header">
				<h1 class="video-title">{video.title}</h1>
				<div class="video-actions">
					<p class="video-meta">Uploaded {formatDate(video.created_at)}</p>
					<button class="btn-copy" onclick={copyLink}>
						{#if copied}
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="20 6 9 17 4 12" />
							</svg>
							Copied!
						{:else}
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
								<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
							</svg>
							Copy link
						{/if}
					</button>
				</div>
			</div>
		</div>
	{:else if video.status === 'failed'}
		<div class="video-container">
			<div class="video-header">
				<h1 class="video-title">{video.title}</h1>
				<p class="video-meta">Uploaded {formatDate(video.created_at)}</p>
			</div>

			<div class="state-card card error-card">
				<div class="state-icon error-icon">
					<svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
						<line x1="12" y1="9" x2="12" y2="13" />
						<line x1="12" y1="17" x2="12.01" y2="17" />
					</svg>
				</div>
				<h2>Processing failed</h2>
				<p class="state-hint">Something went wrong while processing this video.</p>
				<a href="/upload" class="btn-primary">Upload again</a>
			</div>
		</div>
	{/if}

	{#if pageError}
		<div class="page-error">{pageError}</div>
	{/if}
</div>

<style>
	.share-page {
		width: 100%;
		max-width: 800px;
	}

	.video-container {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.video-header {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.video-title {
		font-size: 1.4rem;
		font-weight: 700;
		letter-spacing: -0.02em;
	}

	.video-meta {
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	.video-actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.btn-copy {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 14px;
		background: var(--bg-surface);
		color: var(--text-secondary);
		font-size: 0.8rem;
		font-weight: 500;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		transition:
			color var(--transition-fast),
			border-color var(--transition-fast),
			background var(--transition-fast);
	}

	.btn-copy:hover {
		color: var(--accent);
		border-color: var(--accent);
		background: var(--accent-subtle);
	}

	.state-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 14px;
		text-align: center;
		padding: 48px 32px;
	}

	.state-card h2 {
		font-size: 1.2rem;
		font-weight: 700;
	}

	.state-icon {
		color: var(--text-muted);
	}

	.state-icon.upload-icon {
		color: var(--accent);
		animation: pulse-glow 2s ease infinite;
	}

	.state-icon.error-icon {
		color: var(--danger);
	}

	.state-hint {
		color: var(--text-secondary);
		font-size: 0.9rem;
		max-width: 360px;
	}

	.error-card {
		border-color: rgba(239, 68, 68, 0.2);
	}

	.loading-spinner {
		width: 32px;
		height: 32px;
		border: 3px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	.pulse-bar {
		width: 200px;
		height: 4px;
		background: var(--bg-primary);
		border-radius: 2px;
		overflow: hidden;
		margin-top: 4px;
	}

	.pulse-fill {
		height: 100%;
		width: 40%;
		background: var(--accent);
		border-radius: 2px;
		animation: indeterminate 1.5s ease-in-out infinite;
	}

	@keyframes indeterminate {
		0% { transform: translateX(-100%); }
		100% { transform: translateX(350%); }
	}

	.page-error {
		margin-top: 16px;
		padding: 12px 16px;
		background: var(--danger-subtle);
		color: var(--danger);
		font-size: 0.85rem;
		border-radius: var(--radius-md);
		text-align: center;
	}
</style>
