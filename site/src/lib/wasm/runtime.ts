import {
	WASI,
	File,
	OpenFile,
	ConsoleStdout,
	PreopenDirectory,
	WASIProcExit
} from '@bjorn3/browser_wasi_shim';
import { asset } from '$app/paths';

/** Public URL of the WASI engine, honouring `kit.paths.base` (see svelte.config.js). */
export const WASM_URL = asset('/grapheme-wasm.wasm');

export type ExecuteRequest = {
	source?: string;
	artifact?: unknown;
	initial_current?: unknown;
	args?: unknown;
	entrypoint?: string | null;
};

export type ExecuteError = {
	code: string;
	message: string;
};

export type ExecuteResponse = {
	ok: boolean;
	artifact_id?: string;
	execution?: {
		outcome?: string;
		message?: string | null;
		[key: string]: unknown;
	};
	final_state?: unknown;
	lint_warnings?: unknown[];
	error?: ExecuteError;
};

let wasmBytesPromise: Promise<ArrayBuffer> | null = null;

async function loadWasmBytes(): Promise<ArrayBuffer> {
	if (!wasmBytesPromise) {
		wasmBytesPromise = fetch(WASM_URL).then(async (res) => {
			if (!res.ok) {
				throw new Error(
					`Failed to load grapheme-wasm.wasm (${res.status}). Run bash scripts/build-runtime-wasm.sh && npm run sync-wasm`
				);
			}
			return res.arrayBuffer();
		});
	}
	return wasmBytesPromise;
}

/**
 * Run the RFC-0006 WASI engine in-browser: JSON request on stdin → JSON on stdout.
 */
export async function runGrapheme(request: ExecuteRequest): Promise<ExecuteResponse> {
	const bytes = await loadWasmBytes();
	const stdinPayload = new TextEncoder().encode(JSON.stringify(request));
	const decoder = new TextDecoder();
	let stdout = '';
	let stderr = '';

	const wasi = new WASI(
		['grapheme-wasm'],
		[],
		[
			new OpenFile(new File(stdinPayload)),
			new ConsoleStdout((buf) => {
				stdout += decoder.decode(buf, { stream: true });
			}),
			new ConsoleStdout((buf) => {
				stderr += decoder.decode(buf, { stream: true });
			}),
			new PreopenDirectory('.', new Map())
		],
		{ debug: false }
	);

	const { instance } = await WebAssembly.instantiate(bytes, {
		wasi_snapshot_preview1: wasi.wasiImport
	});

	try {
		wasi.start(
			instance as WebAssembly.Instance & {
				exports: { memory: WebAssembly.Memory; _start: () => void };
			}
		);
	} catch (e) {
		if (!(e instanceof WASIProcExit) || e.code !== 0) {
			const msg = e instanceof Error ? e.message : String(e);
			if (!stdout.trim()) {
				return {
					ok: false,
					error: {
						code: 'WASI_START_FAILED',
						message: msg || 'WASI start failed'
					}
				};
			}
		}
	}

	const raw = stdout.trim() || stderr.trim();
	if (!raw) {
		return {
			ok: false,
			error: {
				code: 'EMPTY_OUTPUT',
				message: 'WASI produced no stdout. Is grapheme-wasm.wasm the WASI release build?'
			}
		};
	}

	try {
		return JSON.parse(raw) as ExecuteResponse;
	} catch {
		return {
			ok: false,
			error: {
				code: 'INVALID_OUTPUT',
				message: `Could not parse engine output: ${raw.slice(0, 400)}`
			}
		};
	}
}
