// Studio engine — the main-thread side of the AudioWorklet DSP.
//
// Browsers only allow audio after a user gesture, so `start()` is called from a
// click. It boots an AudioContext, loads the worklet, compiles the wasm and
// hands the module to the worklet (which instantiates it). After that, params
// and notes are fire-and-forget messages. See static/dsp/dsp-worklet.js.
import { browser } from '$app/environment';

export type Instrument = 'phaseplan' | 'morph';
export interface Route {
	source: number;
	target: number;
	depth: number;
}

const WORKLET_URL = '/dsp/dsp-worklet.js';
const WASM_URL = '/dsp/dsp_wasm.wasm';

export class StudioEngine {
	ctx: AudioContext | null = null;
	node: AudioWorkletNode | null = null;
	analyser: AnalyserNode | null = null;
	instrument: Instrument = 'phaseplan';
	ready = false;
	onVoices: ((n: number) => void) | null = null;
	onReady: (() => void) | null = null;

	/** Boot audio (idempotent; resumes a suspended context). */
	async start(instrument: Instrument = this.instrument): Promise<void> {
		if (!browser) return;
		this.instrument = instrument;
		if (this.ctx) {
			await this.ctx.resume();
			return;
		}
		const ctx = new AudioContext({ latencyHint: 'interactive' });
		await ctx.audioWorklet.addModule(WORKLET_URL);

		// Fetch the wasm bytes and hand them to the worklet. We deliberately send
		// the raw ArrayBuffer (transferred), not a compiled WebAssembly.Module —
		// Chromium's AudioWorklet can't deserialize a Module across postMessage
		// (it fires messageerror). The worklet compiles synchronously in-realm.
		const bytes = await fetch(WASM_URL).then((r) => r.arrayBuffer());

		const node = new AudioWorkletNode(ctx, 'dsp-processor', {
			numberOfInputs: 0,
			numberOfOutputs: 1,
			outputChannelCount: [2]
		});
		const analyser = ctx.createAnalyser();
		analyser.fftSize = 1024;
		node.connect(analyser);
		analyser.connect(ctx.destination);

		node.port.onmessage = (e) => {
			const m = e.data;
			if (m?.type === 'ready') {
				this.ready = true;
				this.onReady?.();
			} else if (m?.type === 'voices') this.onVoices?.(m.n);
			else if (m?.type === 'error') console.error('[dsp-worklet]', m.message);
		};
		node.port.postMessage({ type: 'init', bytes, instrument }, [bytes] /* transfer */);

		this.ctx = ctx;
		this.node = node;
		this.analyser = analyser;
	}

	get running(): boolean {
		return !!this.ctx && this.ctx.state === 'running';
	}

	setInstrument(which: Instrument) {
		this.instrument = which;
		this.node?.port.postMessage({ type: 'instrument', instrument: which });
	}

	setParam(id: number, value: number) {
		this.node?.port.postMessage({ type: 'param', id, value });
	}

	noteOn(note: number, vel = 1) {
		this.node?.port.postMessage({ type: 'noteOn', note, vel });
	}

	noteOff(note: number) {
		this.node?.port.postMessage({ type: 'noteOff', note });
	}

	allNotesOff() {
		this.node?.port.postMessage({ type: 'allOff' });
	}

	setRoutes(routes: Route[]) {
		this.node?.port.postMessage({ type: 'routes', routes });
	}

	/** Current peak level 0..1 from the analyser (for a meter). */
	peak(): number {
		if (!this.analyser) return 0;
		const buf = new Float32Array(this.analyser.fftSize);
		this.analyser.getFloatTimeDomainData(buf);
		let p = 0;
		for (const v of buf) p = Math.max(p, Math.abs(v));
		return p;
	}

	async dispose() {
		this.node?.disconnect();
		this.analyser?.disconnect();
		await this.ctx?.close();
		this.ctx = null;
		this.node = null;
		this.analyser = null;
		this.ready = false;
	}
}
