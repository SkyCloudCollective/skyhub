// SkyHub studio — AudioWorkletProcessor hosting the wasm DSP.
//
// This is a classic worklet script (no ES imports). The main thread compiles
// the self-contained dsp_wasm.wasm and posts the WebAssembly.Module here; we
// instantiate it synchronously (no fetch/async in the worklet realm) and call
// the C-ABI exports directly on the linear memory. The same morph/phaseplan
// code the native plugin runs — see docs/dsp-wasm-abi.md.

const QUANTUM = 128;

class DspProcessor extends AudioWorkletProcessor {
	constructor() {
		super();
		this.ready = false;
		this.x = null; // wasm exports
		this.handle = 0;
		this.instrument = 'phaseplan';
		this.outPtr = 0;
		this.frames = 0;
		this.port.onmessage = (e) => this.onMessage(e.data);
		this.port.onmessageerror = (e) =>
			this.port.postMessage({ type: 'error', message: 'messageerror: ' + String(e?.data) });
	}

	onMessage(m) {
		try {
			this.handleMessage(m);
		} catch (err) {
			this.port.postMessage({ type: 'error', message: String(err && err.stack ? err.stack : err) });
		}
	}

	handleMessage(m) {
		switch (m.type) {
			case 'init': {
				// Compile in-realm: synchronous WebAssembly compilation is allowed
				// inside a worklet (unlike the main thread). The module is
				// self-contained — no imports — so the import object is empty.
				const module = new WebAssembly.Module(m.bytes);
				const inst = new WebAssembly.Instance(module, {});
				this.x = inst.exports;
				this.outPtr = this.x.rs_alloc(QUANTUM);
				this.setInstrument(m.instrument || 'phaseplan');
				this.ready = true;
				this.port.postMessage({ type: 'ready' });
				break;
			}
			case 'instrument':
				this.setInstrument(m.instrument);
				break;
			case 'param':
				if (!this.ready) break;
				if (this.instrument === 'phaseplan')
					this.x.phaseplan_set_param(this.handle, m.id, m.value);
				else this.x.morph_set_param(this.handle, m.id, m.value);
				break;
			case 'noteOn':
				if (!this.ready) break;
				if (this.instrument === 'phaseplan')
					this.x.phaseplan_note_on(this.handle, m.note, m.vel ?? 1);
				else this.x.morph_note_on(this.handle, m.note, m.vel ?? 1);
				break;
			case 'noteOff':
				if (!this.ready) break;
				if (this.instrument === 'phaseplan') this.x.phaseplan_note_off(this.handle, m.note);
				else this.x.morph_note_off(this.handle, m.note);
				break;
			case 'allOff':
				if (!this.ready) break;
				if (this.instrument === 'phaseplan') this.x.phaseplan_all_notes_off(this.handle);
				else this.x.morph_all_notes_off(this.handle);
				break;
			case 'routes':
				if (this.ready && this.instrument === 'phaseplan') {
					this.x.phaseplan_clear_routes(this.handle);
					for (const r of m.routes) this.x.phaseplan_add_route(this.handle, r.source, r.target, r.depth);
					this.x.phaseplan_commit_routes(this.handle);
				}
				break;
		}
	}

	setInstrument(which) {
		if (!this.x) return;
		if (this.handle) {
			if (this.instrument === 'phaseplan') this.x.phaseplan_free(this.handle);
			else this.x.morph_free(this.handle);
			this.handle = 0;
		}
		this.instrument = which;
		this.handle =
			which === 'phaseplan' ? this.x.phaseplan_new(sampleRate) : this.x.morph_new(sampleRate);
	}

	process(_inputs, outputs) {
		const out = outputs[0];
		if (!this.ready || !out || out.length === 0) return true;
		const n = Math.min(out[0].length, QUANTUM);

		if (this.instrument === 'phaseplan') this.x.phaseplan_process(this.handle, this.outPtr, n);
		else this.x.morph_process(this.handle, this.outPtr, n);

		// Re-derive the view every block (the buffer detaches if memory grows).
		const buf = new Float32Array(this.x.memory.buffer, this.outPtr, n);
		for (let c = 0; c < out.length; c++) out[c].set(buf);

		// Report active voices / held notes ~20×/s so the UI can show activity.
		this.frames += n;
		if (this.frames >= 2400) {
			this.frames = 0;
			const v =
				this.instrument === 'phaseplan'
					? this.x.phaseplan_active_voices(this.handle)
					: this.x.morph_held_notes(this.handle);
			this.port.postMessage({ type: 'voices', n: v });
		}
		return true;
	}
}

registerProcessor('dsp-processor', DspProcessor);
