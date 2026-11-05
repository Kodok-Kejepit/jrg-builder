/* JRG IDE — js/berkas.js
   Berkas dari LUAR dan ke luar IDE: unggah dari penyimpanan HP atau komputer, unduh/ekspor.
   Di APK, pemilih berkas dan penyimpanan lewat `JRGAndroid` (Storage Access Framework: tanpa izin
   penyimpanan, pemakai sendiri yang memilih); di peramban biasa lewat <input type=file> dan unduhan. */
(function (IDE) {
  'use strict';

  var POTONG = 393216;   /* byte mentah per permintaan: 512 KB base64 */

  function base64(buf) {
    var b = new Uint8Array(buf), s = '', i;
    for (i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
    return btoa(s);
  }
  function bacaBuffer(f) {
    if (f.arrayBuffer) return f.arrayBuffer();
    return new Promise(function (ok, gagal) {
      var r = new FileReader();
      r.onload = function () { ok(r.result); };
      r.onerror = function () { gagal(r.error || new Error('berkas tidak terbaca')); };
      r.readAsArrayBuffer(f);
    });
  }

  /* Tulis satu berkas ke `nama` (relatif akar data), berpotongan, biner utuh. */
  function unggahSatu(api, f, nama, kabar) {
    return bacaBuffer(f).then(function (buf) {
      var n = buf.byteLength, rantai = Promise.resolve(), dari = 0;
      do {
        (function (a, b, awal) {
          rantai = rantai.then(function () {
            if (kabar && n > POTONG) kabar(nama + ' ' + Math.round(b / n * 100) + '%');
            return api('/api/unggah', { nama: nama, isi: base64(buf.slice(a, b)), awal: awal });
          });
        })(dari, Math.min(n, dari + POTONG), dari === 0);
        dari += POTONG;
      } while (dari < n);
      return rantai;
    });
  }

  /* `daftar`: FileList/array; `dir`: folder tujuan ('' = akar). Jalur folder (webkitRelativePath)
     dipertahankan. Jawabannya {jumlah, gagal: [pesan]} — satu berkas yang gagal tidak menghentikan sisanya. */
  function unggah(api, daftar, dir, kabar) {
    var berkas = Array.prototype.slice.call(daftar || []), hasil = { jumlah: 0, gagal: [] }, rantai = Promise.resolve();
    berkas.forEach(function (f, i) {
      var rel = (f.webkitRelativePath || f.name || '').replace(/^\/+/, '');
      var nama = (dir ? dir + '/' : '') + rel;
      rantai = rantai.then(function () {
        if (kabar) kabar('Mengunggah ' + (i + 1) + ' dari ' + berkas.length + ': ' + rel);
        return unggahSatu(api, f, nama, kabar).then(function () { hasil.jumlah++; }, function (x) { hasil.gagal.push(rel + ': ' + (x.message || x)); });
      });
    });
    return rantai.then(function () { return hasil; });
  }

  /* ---------- zip tanpa kompresi (untuk mengunduh folder dari peramban) ---------- */
  var CRC = (function () {
    var t = new Uint32Array(256);
    for (var n = 0; n < 256; n++) { var c = n; for (var k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; }
    return t;
  })();
  function crc32(b) { var c = 0xFFFFFFFF; for (var i = 0; i < b.length; i++) c = CRC[(c ^ b[i]) & 0xFF] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; }
  /* `isi`: [{nama, data: Uint8Array}] → Blob zip (metode 0, nama UTF-8). */
  function zip(isi) {
    var bagian = [], pusat = [], posisi = 0, enc = new TextEncoder();
    function u16(v) { return [v & 255, (v >>> 8) & 255]; }
    function u32(v) { return [v & 255, (v >>> 8) & 255, (v >>> 16) & 255, (v >>> 24) & 255]; }
    isi.forEach(function (f) {
      var nm = enc.encode(f.nama), crc = crc32(f.data), n = f.data.length;
      var kepala = [].concat(u32(0x04034b50), u16(20), u16(0x0800), u16(0), u16(0), u16(0x21), u32(crc), u32(n), u32(n), u16(nm.length), u16(0));
      bagian.push(new Uint8Array(kepala), nm, f.data);
      pusat.push(new Uint8Array([].concat(u32(0x02014b50), u16(20), u16(20), u16(0x0800), u16(0), u16(0), u16(0x21), u32(crc), u32(n), u32(n),
        u16(nm.length), u16(0), u16(0), u16(0), u16(0), u32(0), u32(posisi))), nm);
      posisi += kepala.length + nm.length + n;
    });
    var besarPusat = pusat.reduce(function (a, b) { return a + b.length; }, 0);
    var akhir = new Uint8Array([].concat(u32(0x06054b50), u16(0), u16(0), u16(isi.length), u16(isi.length), u32(besarPusat), u32(posisi), u16(0)));
    return new Blob(bagian.concat(pusat, [akhir]), { type: 'application/zip' });
  }

  function jalurUrl(nama) { return nama.split('/').map(encodeURIComponent).join('/'); }
  function simpanBlob(blob, nama) {
    var a = document.createElement('a'), url = URL.createObjectURL(blob);
    a.href = url; a.download = nama;
    document.body.appendChild(a); a.click(); a.remove();
    setTimeout(function () { URL.revokeObjectURL(url); }, 30000);
  }

  /* Berkas atau folder ke luar IDE. Di APK: pemilih "simpan ke" Android; jawabannya datang sebagai
     peristiwa `jrg-berkas`. Di peramban: unduhan (folder sebagai .zip). Jawabannya 'ditanyakan',
     'diunduh', atau sebab gagalnya. */
  /* APK yang dibangun sebelum `JRGAndroid.ekspor` ada: WebView-nya tidak punya pemilih berkas dan tidak
     bisa mengunduh, jadi ketukan apa pun akan diam. Dikatakan, bukan dibiarkan. */
  var BELUM_APK = 'Aplikasi ini dibangun sebelum pemilih berkas Android ada, jadi WebView-nya tidak bisa membuka penyimpanan HP. Perlu APK JRG IDE yang lebih baru (Aktivitas dengan pemilih berkas).';
  function apkLama(info) { return !!(info && info.android) && !(window.JRGAndroid && window.JRGAndroid.ekspor); }

  function ekspor(api, info, nama, folder, semua) {
    var A = window.JRGAndroid;
    if (apkLama(info)) return Promise.resolve(BELUM_APK);
    if (A && A.ekspor) {
      var jalur = info.data + '/' + nama;
      return Promise.resolve(folder ? A.eksporFolder(jalur) : A.ekspor(jalur));
    }
    if (!folder) {
      return fetch('/berkas/' + jalurUrl(nama)).then(function (r) {
        if (!r.ok) throw new Error('berkas tidak terbaca (' + r.status + ')');
        return r.blob();
      }).then(function (b) { simpanBlob(b, nama.slice(nama.lastIndexOf('/') + 1)); return 'diunduh'; });
    }
    var isi = (semua || []).filter(function (f) { return f.nama.indexOf(nama + '/') === 0; });
    var dasar = nama.slice(nama.lastIndexOf('/') + 1);
    return Promise.all(isi.map(function (f) {
      return fetch('/berkas/' + jalurUrl(f.nama)).then(function (r) { return r.arrayBuffer(); })
        .then(function (b) { return { nama: dasar + '/' + f.nama.slice(nama.length + 1), data: new Uint8Array(b) }; });
    })).then(function (f) { simpanBlob(zip(f), dasar + '.zip'); return 'diunduh'; });
  }

  /* Folder dari penyimpanan HP: cuma di APK (WebView tidak bisa memilih folder lewat <input>). */
  function imporFolder(info, dir) {
    var A = window.JRGAndroid;
    if (!A || !A.imporFolder) return Promise.resolve(null);
    return Promise.resolve(A.imporFolder(info.data + (dir ? '/' + dir : '')));
  }

  IDE.berkas = { apkLama: apkLama, BELUM_APK: BELUM_APK, unggah: unggah, ekspor: ekspor, imporFolder: imporFolder, zip: zip, simpanBlob: simpanBlob, android: function () { return !!(window.JRGAndroid && window.JRGAndroid.ekspor); } };
})(window.IDE);
