// Menjalankan JRG di peramban lewat mesin Praktik (aset/jrg-web.js, interpreter jrgc dalam WebAssembly).
// Asetnya ±5 MB, jadi baru diunduh saat tombol Jalankan pertama kali ditekan.
(function () {
  const ASET = (document.currentScript && document.currentScript.src)
    ? new URL('jrg-web.js', document.currentScript.src).href : 'aset/jrg-web.js';
  const BATAS_MS = 15000;
  const BATAS_KELUARAN = 1 << 20;
  let mesin = null;

  function muat() {
    if (mesin) return mesin;
    mesin = new Promise(function (ok, gagal) {
      const pakai = function () {
        try {
          const b = atob(window.JRG_WEB.wasm);
          const u8 = new Uint8Array(b.length);
          for (let i = 0; i < b.length; i++) u8[i] = b.charCodeAt(i);
          WebAssembly.compile(u8).then(ok, gagal);
        } catch (e) { gagal(e); }
      };
      if (window.JRG_WEB) return pakai();
      const s = document.createElement('script');
      s.src = ASET;
      s.onload = pakai;
      s.onerror = function () { gagal(new Error('mesin JRG (' + ASET + ') tidak bisa dimuat')); };
      document.head.appendChild(s);
    });
    mesin.catch(function () { mesin = null; });
    return mesin;
  }

  // Jawaban: { potongan: [[fd, teks]...], kode, sebab, terpotong }.
  function jalankan(sumber) {
    return muat().then(function (modul) {
      return new Promise(function (ok) {
        const url = URL.createObjectURL(new Blob(['(' + window.JRG_WEB.pekerja.toString() + ')()'],
          { type: 'text/javascript' }));
        const w = new Worker(url);
        const potongan = [];
        let panjang = 0, terpotong = false, beres = false;
        const selesai = function (h) {
          if (beres) return;
          beres = true;
          clearTimeout(jam);
          w.terminate();
          URL.revokeObjectURL(url);
          ok({ potongan: potongan, terpotong: terpotong, kode: h.kode, sebab: h.sebab });
        };
        const jam = setTimeout(function () { selesai({ kode: null, sebab: 'waktu' }); }, BATAS_MS);
        w.onmessage = function (ev) {
          const d = ev.data;
          if (d.jenis === 'keluaran') {
            for (const [fd, t] of d.potongan) {
              if (panjang + t.length > BATAS_KELUARAN) { terpotong = true; continue; }
              panjang += t.length;
              potongan.push([fd, t]);
            }
          } else if (d.jenis === 'selesai') selesai(d);
        };
        w.onerror = function (e) { e.preventDefault(); selesai({ kode: 1, sebab: 'pekerja: ' + e.message }); };
        w.postMessage({ modul: modul, sumber: sumber });
      });
    });
  }

  function kalimatSebab(h) {
    if (h.sebab === 'waktu') return 'Dihentikan: program jalan lebih dari ' + (BATAS_MS / 1000) +
      ' detik. Mungkin ada perulangan yang tidak pernah berhenti.';
    if (h.sebab === 'rekursi' || (h.sebab && h.sebab.indexOf('out of bounds') >= 0))
      return 'Rekursinya terlalu dalam untuk mesin di peramban (beberapa ratus lapis; jrgc di komputer sampai 10000).';
    if (h.sebab) return 'Mesin di peramban berhenti tidak wajar (' + h.sebab + ').';
    return null;
  }

  window.JRG_PRAKTIK = { jalankan: jalankan, kalimatSebab: kalimatSebab };
})();
