<script lang="ts">
	import { MAX_UPLOAD_BYTES, SUPPORTED_EXTENSIONS, SUPPORTED_MIME_TYPES } from '$lib/types';

	let {
		onfileselected,
		disabled = false
	}: {
		onfileselected: (file: File) => void;
		disabled?: boolean;
	} = $props();

	let dragging = $state(false);
	let error = $state('');
	let fileInputEl: HTMLInputElement;

	function validateFile(file: File): string | null {
		if (file.size > MAX_UPLOAD_BYTES) {
			return `File too large (${formatBytes(file.size)}). Maximum is 1 GB.`;
		}
		const ext = '.' + file.name.split('.').pop()?.toLowerCase();
		if (!SUPPORTED_EXTENSIONS.includes(ext)) {
			return `Unsupported format. Supported: ${SUPPORTED_EXTENSIONS.join(', ')}`;
		}
		if (file.type && !SUPPORTED_MIME_TYPES.includes(file.type)) {
			return `Unsupported MIME type: ${file.type}`;
		}
		return null;
	}

	function handleFile(file: File) {
		error = '';
		const validationError = validateFile(file);
		if (validationError) {
			error = validationError;
			return;
		}
		onfileselected(file);
	}

	function handleDrop(e: DragEvent) {
		dragging = false;
		if (disabled) return;
		const file = e.dataTransfer?.files[0];
		if (file) handleFile(file);
	}

	function handleInput(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (file) handleFile(file);
		input.value = '';
	}

	function formatBytes(bytes: number): string {
		if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(1) + ' GB';
		if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(1) + ' MB';
		return (bytes / 1024).toFixed(1) + ' KB';
	}
</script>

<div
	class="dropzone"
	class:dragging
	class:disabled
	role="button"
	tabindex="0"
	ondragover={(e) => { e.preventDefault(); if (!disabled) dragging = true; }}
	ondragleave={() => { dragging = false; }}
	ondrop={(e) => { e.preventDefault(); handleDrop(e); }}
	onclick={() => { if (!disabled) fileInputEl.click(); }}
	onkeydown={(e) => { if (e.key === 'Enter' && !disabled) fileInputEl.click(); }}
>
	<input
		bind:this={fileInputEl}
		type="file"
		accept="video/*"
		class="file-input"
		onchange={handleInput}
		{disabled}
	/>

	<div class="dropzone-content">
		<div class="dropzone-icon">
			<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
				<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
				<polyline points="17 8 12 3 7 8" />
				<line x1="12" y1="3" x2="12" y2="15" />
			</svg>
		</div>
		<p class="dropzone-label">
			{#if dragging}
				Drop your video here
			{:else}
				Drag & drop a video, or <span class="browse-link">browse</span>
			{/if}
		</p>
		<p class="dropzone-hint">
			MP4, MOV, WebM, AVI, MKV &middot; Max 1 GB
		</p>
	</div>
</div>

{#if error}
	<p class="dropzone-error">{error}</p>
{/if}

<style>
	.dropzone {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		min-height: 200px;
		border: 2px dashed var(--border);
		border-radius: var(--radius-lg);
		background: var(--bg-surface);
		cursor: pointer;
		transition:
			border-color var(--transition-normal),
			background var(--transition-normal),
			box-shadow var(--transition-normal);
	}

	.dropzone:hover:not(.disabled),
	.dropzone.dragging {
		border-color: var(--accent);
		background: var(--accent-subtle);
		box-shadow: 0 0 30px var(--accent-glow);
	}

	.dropzone.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.file-input {
		display: none;
	}

	.dropzone-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		padding: 24px;
		text-align: center;
	}

	.dropzone-icon {
		color: var(--text-muted);
		transition: color var(--transition-normal);
	}

	.dropzone:hover:not(.disabled) .dropzone-icon,
	.dropzone.dragging .dropzone-icon {
		color: var(--accent);
	}

	.dropzone-label {
		font-size: 1rem;
		color: var(--text-secondary);
	}

	.browse-link {
		color: var(--accent);
		font-weight: 600;
	}

	.dropzone-hint {
		font-size: 0.8rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.dropzone-error {
		margin-top: 10px;
		padding: 10px 14px;
		background: var(--danger-subtle);
		color: var(--danger);
		font-size: 0.85rem;
		border-radius: var(--radius-sm);
	}
</style>
