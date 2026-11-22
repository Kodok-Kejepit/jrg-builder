// Dibuat jrgc --emit-wasm. Pakai: node --stack-size=3000 <program>.wasm.mjs [argumen...]
import fs from 'node:fs';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { WASI } from 'node:wasi';
const wasm = fileURLToPath(import.meta.url).replace(/\.mjs$/, '');
const wasi = new WASI({ version: 'preview1', args: [wasm, ...process.argv.slice(2)], env: process.env,
  preopens: { '/': '/' }, returnOnExit: true });
const mod = await WebAssembly.compile(fs.readFileSync(wasm));
const inst = await WebAssembly.instantiate(mod, { wasi_snapshot_preview1: wasi.wasiImport });
process.exitCode = wasi.start(inst);
