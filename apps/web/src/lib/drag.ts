// Drag a sample out to the OS / DAW.
//
// In a plain browser the best we can do is hand over a download LINK
// (DownloadURL + text/uri-list). The desktop shell (Tauri) upgrades this to a
// real native file drag + a local library folder — see P5. We detect the shell
// via window.__rsDrag (injected by the desktop build) and defer to it there.
import { downloadUrl } from './api';
import type { Sample } from './api';

declare global {
	interface Window {
		__rsDrag?: (sample: Sample, url: string) => void;
		__TAURI__?: unknown;
	}
}

export function dragOutSample(e: DragEvent, sample: Sample) {
	const url = new URL(downloadUrl(sample.id), location.href).href;

	// Desktop shell present → let it start a native OS file drag (real file).
	if (typeof window !== 'undefined' && window.__rsDrag) {
		e.preventDefault();
		window.__rsDrag(sample, url);
		return;
	}

	// Browser fallback: a download link the DAW/desktop can fetch, PLUS an
	// internal id so in-app crate drop-zones can accept the same drag.
	if (e.dataTransfer) {
		const name = sample.filename || `${sample.title}.wav`;
		e.dataTransfer.effectAllowed = 'copy';
		e.dataTransfer.setData('DownloadURL', `application/octet-stream:${name}:${url}`);
		e.dataTransfer.setData('text/uri-list', url);
		e.dataTransfer.setData('text/plain', url);
		e.dataTransfer.setData(RS_DRAG_MIME, String(sample.id));
	}
}

// Internal MIME carrying the sample id for drag-to-crate within the app.
export const RS_DRAG_MIME = 'application/x-ranchsample-id';

export function readDraggedSampleId(e: DragEvent): number | null {
	const raw = e.dataTransfer?.getData(RS_DRAG_MIME);
	const id = raw ? parseInt(raw, 10) : NaN;
	return Number.isFinite(id) ? id : null;
}
