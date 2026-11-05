// perkakas/situs/jalankan_praktik.js — jalankan program lewat mesin Praktik yang
// TERBIT (`docs/aset/jrg-web.js`), dengan badan pekerja yang sama dengan peramban.
'use strict';
const fs = require('fs');
const [aset, daftar] = process.argv.slice(2);
globalThis.window = {};
new Function(fs.readFileSync(aset, 'utf8'))();
const J = window.JRG_WEB;
const modul = new WebAssembly.Module(Buffer.from(J.wasm, 'base64'));
const hasil = [];
for (const sumber of JSON.parse(fs.readFileSync(daftar, 'utf8'))) {
  const pesan = [];
  // Pekerja sungguhan menerima pesan secara asinkron; di sini `onmessage`
  // dipanggil langsung, jadi semua pesannya sudah terkumpul saat ia kembali.
  globalThis.self = { postMessage: (m) => pesan.push(m) };
  new Function('return ' + J.pekerja.toString())()();
  self.onmessage({ data: { modul, sumber } });
  let keluaran = '', galat = '', akhir = null;
  for (const m of pesan) {
    if (m.jenis === 'keluaran') {
      for (const [fd, t] of m.potongan) { if (fd === 2) galat += t; else keluaran += t; }
    } else if (m.jenis === 'selesai') akhir = m;
  }
  hasil.push({ kode: akhir ? akhir.kode : null, sebab: akhir ? akhir.sebab : 'tanpa-akhir', keluaran, galat });
}
process.stdout.write(JSON.stringify(hasil));
