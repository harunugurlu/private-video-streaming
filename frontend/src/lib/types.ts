export interface CreateVideoRequest {
	title: string;
	filename: string;
	size_bytes: number;
	mime_type: string;
}

export interface CreateVideoResponse {
	video_id: string;
	share_token: string;
	status: string;
	max_upload_bytes: number;
}

export interface UploadSourceResponse {
	video_id: string;
	status: string;
	upload_completed_at: string;
}

export interface VideoStatusResponse {
	video_id: string;
	status: VideoStatus;
	processing_stage: ProcessingStage | null;
	share_token: string;
	updated_at: string;
	error_code: string | null;
	error_message: string | null;
}

export interface PlaybackInfo {
	hls_url: string | null;
}

export interface ShareResponse {
	video_id: string;
	title: string;
	status: VideoStatus;
	processing_stage: ProcessingStage | null;
	playback: PlaybackInfo;
	created_at: string;
}

export interface ApiError {
	error: string;
	message: string;
}

export type VideoStatus = 'uploading' | 'processing' | 'playable' | 'failed';
export type ProcessingStage = 'queued' | 'probing' | 'transcoding' | 'publishing';

export const MAX_UPLOAD_BYTES = 1_073_741_824; // 1 GB

export const SUPPORTED_EXTENSIONS = ['.mp4', '.mov', '.webm', '.avi', '.mkv'];

export const SUPPORTED_MIME_TYPES = [
	'video/mp4',
	'video/quicktime',
	'video/webm',
	'video/x-msvideo',
	'video/x-matroska'
];

export const STAGE_PROGRESS: Record<ProcessingStage, number> = {
	queued: 5,
	probing: 15,
	transcoding: 50,
	publishing: 90
};
