<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, ApiRequestError } from '$lib/api';
	import UploadDropzone from '$lib/components/UploadDropzone.svelte';

	type UploadState = 'idle' | 'ready' | 'creating' | 'uploading' | 'redirecting' | 'error';

	let title = $state('');
	let selectedFile: File | null = $state(null);
	let uploadState = $state<UploadState>('idle');
	let uploadProgress = $state(0);
	let errorMessage = $state('');
	let abortUpload: (() => void) | null = $state(null);

	let canSubmit = $derived(uploadState === 'ready' && selectedFile !== null);

	function onFileSelected(file: File) {
		selectedFile = file;
		if (!title) {
			title = file.name.replace(/\.[^.]+$/, '');
		}
		uploadState = 'ready';
		errorMessage = '';
	}

	function formatBytes(bytes: number): string {
		if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(1) + ' GB';
		if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(1) + ' MB';
		return (bytes / 1024).toFixed(1) + ' KB';
	}

	async function handleSubmit() {
		if (!selectedFile || !title.trim()) return;

		uploadState = 'creating';
		errorMessage = '';

		try {
			const created = await api.createVideo({
				title: title.trim(),
				filename: selectedFile.name,
				size_bytes: selectedFile.size,
				mime_type: selectedFile.type || 'video/mp4'
			});

			uploadState = 'uploading';
			uploadProgress = 0;

			const { promise, abort } = api.uploadSource(
				created.video_id,
				selectedFile,
				(pct) => { uploadProgress = pct; }
			);
			abortUpload = abort;

			await promise;

			uploadState = 'redirecting';
			await goto(`/watch/${created.share_token}`);
		} catch (err) {
			uploadState = 'error';
			if (err instanceof ApiRequestError) {
				errorMessage = err.message;
			} else {
				errorMessage = 'An unexpected error occurred. Please try again.';
			}
		} finally {
			abortUpload = null;
		}
	}

	function handleCancel() {
		if (abortUpload) abortUpload();
		uploadState = 'ready';
		uploadProgress = 0;
		errorMessage = '';
	}

	function handleRetry() {
		uploadState = 'ready';
		errorMessage = '';
	}
</script>

<svelte:head>
	<title>Upload — StreamDrop</title>
</svelte:head>

<div class="upload-page fade-in">
	<div class="upload-card card">
		<h1 class="page-title">Upload a video</h1>
		<p class="page-subtitle">Share it instantly with a link</p>

		<div class="form-group">
			<label for="title-input" class="form-label">Title</label>
			<input
				id="title-input"
				type="text"
				class="form-input"
				placeholder="Give your video a name"
				bind:value={title}
				disabled={uploadState === 'uploading' || uploadState === 'creating'}
			/>
		</div>

		<div class="form-group">
			<UploadDropzone
				onfileselected={onFileSelected}
				disabled={uploadState === 'uploading' || uploadState === 'creating'}
			/>
		</div>

		{#if selectedFile && uploadState !== 'idle'}
			<div class="file-info">
				<div class="file-icon">
					<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<polygon points="23 7 16 12 23 17 23 7" />
						<rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
					</svg>
				</div>
				<div class="file-details">
					<span class="file-name">{selectedFile.name}</span>
					<span class="file-size">{formatBytes(selectedFile.size)}</span>
				</div>
			</div>
		{/if}

		{#if uploadState === 'uploading'}
			<div class="progress-section">
				<div class="progress-header">
					<span class="progress-label">Uploading...</span>
					<span class="progress-pct">{Math.round(uploadProgress)}%</span>
				</div>
				<div class="progress-track">
					<div class="progress-fill" style="width: {uploadProgress}%"></div>
				</div>
				<button class="btn-cancel" onclick={handleCancel}>Cancel</button>
			</div>
		{:else if uploadState === 'creating'}
			<div class="progress-section">
				<div class="progress-header">
					<span class="progress-label">Preparing upload...</span>
				</div>
				<div class="progress-track">
					<div class="progress-fill indeterminate"></div>
				</div>
			</div>
		{:else if uploadState === 'redirecting'}
			<div class="progress-section">
				<div class="progress-header">
					<span class="progress-label">Upload complete! Redirecting...</span>
				</div>
			</div>
		{/if}

		{#if errorMessage}
			<div class="error-banner">
				<span>{errorMessage}</span>
				<button class="btn-retry" onclick={handleRetry}>Try again</button>
			</div>
		{/if}

		{#if uploadState !== 'uploading' && uploadState !== 'creating' && uploadState !== 'redirecting'}
			<button
				class="btn-primary submit-btn"
				disabled={!canSubmit || !title.trim()}
				onclick={handleSubmit}
			>
				<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
					<polyline points="17 8 12 3 7 8" />
					<line x1="12" y1="3" x2="12" y2="15" />
				</svg>
				Upload & Share
			</button>
		{/if}
	</div>
</div>

<style>
	.upload-page {
		width: 100%;
		max-width: 540px;
	}

	.upload-card {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.page-title {
		font-size: 1.6rem;
		font-weight: 700;
		letter-spacing: -0.03em;
	}

	.page-subtitle {
		color: var(--text-secondary);
		font-size: 0.95rem;
		margin-top: -12px;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.form-label {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.form-input {
		padding: 12px 16px;
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		color: var(--text-primary);
		font-size: 0.95rem;
		outline: none;
		transition:
			border-color var(--transition-fast),
			box-shadow var(--transition-fast);
	}

	.form-input:focus {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--accent-glow);
	}

	.form-input::placeholder {
		color: var(--text-muted);
	}

	.form-input:disabled {
		opacity: 0.5;
	}

	.file-info {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		background: var(--accent-subtle);
		border: 1px solid var(--border-accent);
		border-radius: var(--radius-md);
	}

	.file-icon {
		color: var(--accent);
		flex-shrink: 0;
	}

	.file-details {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.file-name {
		font-size: 0.9rem;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.file-size {
		font-size: 0.8rem;
		color: var(--text-secondary);
		font-family: var(--font-mono);
	}

	.progress-section {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.progress-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.progress-label {
		font-size: 0.9rem;
		color: var(--text-secondary);
	}

	.progress-pct {
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
		transition: width 0.3s ease;
	}

	.progress-fill.indeterminate {
		width: 30%;
		animation: indeterminate 1.5s ease-in-out infinite;
	}

	@keyframes indeterminate {
		0% { transform: translateX(-100%); }
		100% { transform: translateX(400%); }
	}

	.btn-cancel {
		align-self: flex-end;
		padding: 6px 16px;
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.85rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		transition:
			color var(--transition-fast),
			border-color var(--transition-fast);
	}

	.btn-cancel:hover {
		color: var(--danger);
		border-color: var(--danger);
	}

	.error-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 12px 16px;
		background: var(--danger-subtle);
		color: var(--danger);
		font-size: 0.9rem;
		border-radius: var(--radius-md);
	}

	.btn-retry {
		flex-shrink: 0;
		padding: 6px 14px;
		background: transparent;
		color: var(--danger);
		font-size: 0.8rem;
		font-weight: 600;
		border: 1px solid var(--danger);
		border-radius: var(--radius-sm);
		transition: background var(--transition-fast);
	}

	.btn-retry:hover {
		background: rgba(239, 68, 68, 0.15);
	}

	.submit-btn {
		width: 100%;
		margin-top: 4px;
	}
</style>
