<script lang="ts">
	import type { ProcessingStage, VideoStatus } from '$lib/types';
	import { STAGE_PROGRESS } from '$lib/types';

	let {
		status,
		stage
	}: {
		status: VideoStatus;
		stage: ProcessingStage | null;
	} = $props();

	let progressPercent = $derived(
		status === 'playable'
			? 100
			: stage
				? STAGE_PROGRESS[stage] ?? 0
				: 0
	);

	let stageLabel = $derived(
		stage === 'queued' ? 'Queued'
		: stage === 'probing' ? 'Analyzing video'
		: stage === 'transcoding' ? 'Transcoding'
		: stage === 'publishing' ? 'Almost done'
		: 'Preparing'
	);
</script>

<div class="processing">
	<div class="processing-header">
		<div class="processing-indicator">
			<div class="spinner"></div>
			<span class="processing-label">{stageLabel}...</span>
		</div>
		<span class="processing-pct">{progressPercent}%</span>
	</div>

	<div class="progress-track">
		<div class="progress-fill" style="width: {progressPercent}%"></div>
	</div>

	<p class="processing-hint">
		{#if stage === 'transcoding'}
			This may take a moment for larger files
		{:else if stage === 'publishing'}
			Finalizing playback files
		{:else}
			Processing will begin shortly
		{/if}
	</p>
</div>

<style>
	.processing {
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 24px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}

	.processing-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.processing-indicator {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.spinner {
		width: 18px;
		height: 18px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	.processing-label {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.processing-pct {
		font-size: 0.85rem;
		font-family: var(--font-mono);
		color: var(--accent);
		font-weight: 700;
	}

	.progress-track {
		width: 100%;
		height: 6px;
		background: var(--bg-primary);
		border-radius: 3px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 3px;
		transition: width 0.6s ease;
	}

	.processing-hint {
		font-size: 0.8rem;
		color: var(--text-muted);
	}
</style>
