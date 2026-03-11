import type {
	CreateVideoRequest,
	CreateVideoResponse,
	UploadSourceResponse,
	VideoStatusResponse,
	ShareResponse,
	ApiError
} from './types';

class ApiClient {
	private baseUrl: string;

	constructor(baseUrl = '') {
		this.baseUrl = baseUrl;
	}

	async createVideo(req: CreateVideoRequest): Promise<CreateVideoResponse> {
		const res = await fetch(`${this.baseUrl}/api/videos`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(req)
		});
		if (!res.ok) {
			const err: ApiError = await res.json();
			throw new ApiRequestError(res.status, err.error, err.message);
		}
		return res.json();
	}

	uploadSource(
		videoId: string,
		file: File,
		onProgress?: (percent: number) => void
	): { promise: Promise<UploadSourceResponse>; abort: () => void } {
		const xhr = new XMLHttpRequest();
		const promise = new Promise<UploadSourceResponse>((resolve, reject) => {
			xhr.upload.onprogress = (e) => {
				if (e.lengthComputable && onProgress) {
					onProgress((e.loaded / e.total) * 100);
				}
			};

			xhr.onload = () => {
				if (xhr.status >= 200 && xhr.status < 300) {
					resolve(JSON.parse(xhr.responseText));
				} else {
					try {
						const err: ApiError = JSON.parse(xhr.responseText);
						reject(new ApiRequestError(xhr.status, err.error, err.message));
					} catch {
						reject(new ApiRequestError(xhr.status, 'UNKNOWN', xhr.statusText));
					}
				}
			};

			xhr.onerror = () => reject(new ApiRequestError(0, 'NETWORK_ERROR', 'Network error'));
			xhr.onabort = () => reject(new ApiRequestError(0, 'ABORTED', 'Upload cancelled'));

			xhr.open('PUT', `${this.baseUrl}/api/videos/${videoId}/source`);
			const formData = new FormData();
			formData.append('file', file);
			xhr.send(formData);
		});

		return { promise, abort: () => xhr.abort() };
	}

	async getVideoStatus(videoId: string): Promise<VideoStatusResponse> {
		const res = await fetch(`${this.baseUrl}/api/videos/${videoId}/status`);
		if (!res.ok) {
			const err: ApiError = await res.json();
			throw new ApiRequestError(res.status, err.error, err.message);
		}
		return res.json();
	}

	async getShare(token: string): Promise<ShareResponse> {
		const res = await fetch(`${this.baseUrl}/api/share/${token}`);
		if (!res.ok) {
			const err: ApiError = await res.json();
			throw new ApiRequestError(res.status, err.error, err.message);
		}
		return res.json();
	}
}

export class ApiRequestError extends Error {
	status: number;
	code: string;

	constructor(status: number, code: string, message: string) {
		super(message);
		this.name = 'ApiRequestError';
		this.status = status;
		this.code = code;
	}
}

export const api = new ApiClient();
