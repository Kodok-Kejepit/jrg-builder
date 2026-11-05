// ide/uji/peramban.js — skenario fitur IDE 0.6 di chromium headless (DevTools Protocol), ukuran HP.
// Pakai: node peramban.js <chromium> <url>; mencetak satu objek JSON hasil.
'use strict';
const { spawn } = require('child_process');
const fs = require('fs'), os = require('os'), path = require('path');
const [krom, url] = process.argv.slice(2);
const profil = fs.mkdtempSync(path.join(os.tmpdir(), 'jrg-ide-'));
const proses = spawn(krom, ['--headless', '--disable-gpu', '--no-sandbox', '--password-store=basic', '--remote-debugging-port=0',
  '--user-data-dir=' + profil, 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
const akhiri = (kode, pesan) => {
  if (pesan) process.stdout.write(pesan + '\n');
  proses.kill('SIGKILL');
  fs.rmSync(profil, { recursive: true, force: true });
  process.exit(kode);
};
setTimeout(() => akhiri(2, 'BATAS WAKTU'), 420000);
let buf = '', tersambung = false;
proses.stderr.on('data', (d) => {
  buf += d;
  const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
  if (!m || tersambung) return;
  tersambung = true;
  const ws = new WebSocket(m[1]);
  let id = 0;
  const tunggu = new Map(), galat = [];
  const kirim = (method, params, sessionId) => new Promise((ok) => {
    const i = ++id;
    tunggu.set(i, ok);
    ws.send(JSON.stringify(Object.assign({ id: i, method, params: params || {} }, sessionId ? { sessionId } : {})));
  });
  ws.onmessage = (e) => {
    const d = JSON.parse(e.data);
    if (d.id && tunggu.has(d.id)) { tunggu.get(d.id)(d); tunggu.delete(d.id); }
    if (d.method === 'Runtime.exceptionThrown') galat.push(d.params.exceptionDetails.exception ? d.params.exceptionDetails.exception.description : d.params.exceptionDetails.text);
    if (d.method === 'Runtime.consoleAPICalled' && d.params.type === 'error') galat.push('console.error: ' + d.params.args.map((a) => a.value || a.description).join(' '));
  };
  ws.onopen = async () => {
    const t = await kirim('Target.createTarget', { url: 'about:blank' });
    const s = await kirim('Target.attachToTarget', { targetId: t.result.targetId, flatten: true });
    const sid = s.result.sessionId;
    const hasil = {};
    const nilai = async (expr) => {
      const r = await kirim('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }, sid);
      if (r.result && r.result.exceptionDetails) return 'GALAT: ' + JSON.stringify(r.result.exceptionDetails.exception || r.result.exceptionDetails.text);
      return r.result && r.result.result ? r.result.result.value : undefined;
    };
    const sampai = async (expr, apa) => {
      for (let i = 0; i < 160; i++) {
        if (await nilai(expr)) return;
        await new Promise((r) => setTimeout(r, 250));
      }
      hasil.gagal = 'menunggu ' + apa;
      hasil.galat = galat;
      akhiri(1, JSON.stringify(hasil));
    };
    const simpan = (nama, isi) => nilai('fetch("/api/simpan", { method: "POST", body: ' + JSON.stringify(JSON.stringify({ nama, isi })) + ' }).then((r) => r.ok)');
    const buka = async (nama) => {
      await sampai('!!document.querySelector(\'#laci [data-buka="' + nama + '"]\')', nama + ' di laci');
      await nilai('document.querySelector(\'#laci [data-buka="' + nama + '"]\').click()');
      await sampai("document.querySelector('#j-nama').textContent === '" + nama + "'", 'buka ' + nama);
    };
    const selesai = "['ok','gagal'].indexOf(document.querySelector('#status').getAttribute('data-s')) >= 0";
    const setel = "document.querySelector('#status').setAttribute('data-s', ''); document.querySelector('#hasil').textContent = ''; ";
    await kirim('Runtime.enable', {}, sid);
    await kirim('Emulation.setDeviceMetricsOverride', { width: 390, height: 800, deviceScaleFactor: 2, mobile: true }, sid);
    await simpan('proyekku/jrgm.toml', '[package]\nname = "p"\nentry = "src/main.jrg"\n');
    await kirim('Page.navigate', { url }, sid);
    await sampai("document.querySelectorAll('#tab [role=tab]').length > 0", 'tab pertama');
    await simpan('proyekku/src/main.jrg', 'package main\n\nimport "hitung"\n\npub fn main() {\n    wok(hitung::tambah(1, 2))\n}\n');
    await simpan('proyekku/src/hitung.jrg', 'package hitung\n\npub fn tambah(a, b) { a + b }\n');
    await simpan('proyekku/src/rusak.jrg', 'package rusak\n\npub fn f(a) { a + }\n');
    await simpan('argumen.jrg', 'package main\n\npub fn main() {\n    wok(arr::skip(env::args(), 1))\n}\n\nfn test_satu() { assert_eq(1 + 1, 2) }\nfn uji_dua() { assert_eq(2 * 2, 5) }\n');
    await simpan('apku/jrgm.toml', '[package]\nname = "apku"\n\n[android]\nnama = "Apku"\nikon = "lama.png"\n');
    await nilai('location.reload()');
    await new Promise((r) => setTimeout(r, 800));
    await sampai("document.querySelectorAll('#tab [role=tab]').length > 0", 'muat ulang');
    // Logo (0.8.2) termuat di kepala, ikon tab, dan tema "air" mengganti latar serta aksen.
    hasil.logo = await nilai("(document.querySelector('.atas .judul img.logo').naturalWidth) + '|' + !!document.querySelector('link[rel=icon][href$=\"logo.png\"]') + '|' + (function () { var h = document.documentElement, l = h.getAttribute('data-tema'); h.setAttribute('data-tema', 'air'); var c = getComputedStyle(h), r = c.getPropertyValue('--bg').trim() + ' ' + c.getPropertyValue('--accent').trim(); h.setAttribute('data-tema', l); return r; })()");
    // Ikon dan logo (0.8.2): delapan bentuk tergambar, logo IDE diganti (blob, bertahan), PNG ke proyek mengisi [android] ikon.
    await nilai("document.querySelector('#tb-lain').click()");
    await nilai("document.querySelector('#o-logo').click()");
    await sampai("(() => { var k = document.querySelector('[data-bentuk=hex] canvas'); return k && k.getContext('2d').getImageData(48, 48, 1, 1).data[3] > 0; })()", 'pratinjau ikon');
    await nilai("document.querySelector('[data-bentuk=hex]').click(); document.querySelector('[data-lgk=ke][data-v=ide]').click(); document.querySelector('#lg-terap').click()");
    await sampai("document.querySelector('.atas img.logo').src.indexOf('blob:') === 0", 'logo diganti');
    await nilai("var j = document.querySelector('#lg-jalur'); j.value = 'apku/aset/ikon.png'; j.oninput(); document.querySelector('#lg-proyek').click()");
    await sampai("/memakainya/.test(document.querySelector('#lg-kabar').textContent)", 'ikon proyek');
    hasil.ikon = await nilai("Promise.all([fetch('/api/baca?nama=apku/jrgm.toml').then((r) => r.json()), fetch('/berkas/apku/aset/ikon.png').then((r) => r.blob())]).then((d) => document.querySelectorAll('.lg-bentuk button').length + '|' + /ikon += \"aset\\/ikon.png\"/.test(d[0].isi) + '|' + !/lama.png/.test(d[0].isi) + '|' + d[1].type + '|' + (d[1].size > 1000))");
    await nilai("document.querySelector('#lg-polos').click()");
    hasil.logoPolos = await nilai("getComputedStyle(document.querySelector('.atas img.logo')).display + '|' + JSON.parse(localStorage.getItem('jrgide.atur') || '{}').logo");
    await nilai("document.querySelector('#lg-tampil').click()");
    await nilai("document.querySelector('#lg-bawaan').click()");
    await sampai("document.querySelector('.atas img.logo').getAttribute('src') === '/web/aset/logo.png'", 'logo bawaan');
    await nilai("document.querySelector('#le-tutup').click()");
    // Pohon: tiap berkas di proyekku/src punya kolom leluhur + kolom cabang; yang terakhir berujung.
    hasil.pohon = await nilai("Array.prototype.map.call(document.querySelectorAll('#laci [data-buka^=\"proyekku/src/\"]'), function (b) { var r = b.closest('.baris'); return r.querySelectorAll('.pohon i').length + (r.querySelector('.pohon i.c.u') ? 'u' : 'c'); }).join(',')");
    // Pemeriksa: galat tanpa menjalankan, di talang dan di panel Masalah.
    await buka('proyekku/src/rusak.jrg');
    await sampai("document.querySelector('.ed-gutter .is-err')", 'tanda galat di talang');
    hasil.periksa = await nilai("document.querySelector('.ed-gutter .is-err').textContent + ':' + document.querySelector('#bl-masalah-n').textContent");
    await nilai("document.querySelector('#bilah [data-alat=masalah]').click()");
    hasil.masalah = await nilai("document.querySelector('#m-isi b').textContent");
    hasil.simbolBerkas = await nilai("IDE.saran.simbol('package x\\nstruct T { a: int }\\nimpl T {\\n    fn m(self) { 1 }\\n}\\npub fn f() {}\\n', 'jrg').map(function (s) { return s.jenis + ' ' + s.nama + ':' + s.baris; }).join(', ')");
    // Saran ketik: str::spl -> str::split() dengan kursor di kurung, tanda tangannya tampil.
    await nilai("var ta = document.querySelector('#ed textarea'); ta.focus(); ta.setSelectionRange(ta.value.length, ta.value.length); document.execCommand('insertText', false, '\\npub fn g() { str::spl')");
    await sampai("!document.querySelector('.saran').hidden", 'saran ketik');
    await nilai("document.querySelector('#ed textarea').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))");
    await sampai("!document.querySelector('#petunjuk').hidden", 'tanda tangan fungsi');
    hasil.saran = await nilai("document.querySelector('#ed textarea').value.split('\\n').pop() + '|' + document.querySelector('#petunjuk').textContent.split('\\n')[0]");
    // Gandakan dari lembar tindakan berkas.
    await nilai("document.querySelector('#laci [data-aksi-berkas=\"proyekku/src/rusak.jrg\"]').click()");
    await sampai("!!document.querySelector('#bb-salin')", 'lembar berkas');
    await nilai("document.querySelector('#bb-salin').click()");
    await sampai("!!document.querySelector('#laci [data-buka=\"proyekku/src/rusak-salinan.jrg\"]')", 'salinan di laci');
    hasil.salin = await nilai("document.querySelector('#laci [data-buka=\"proyekku/src/rusak-salinan.jrg\"]').getAttribute('data-buka')");
    // Palet buka cepat.
    await nilai("document.querySelector('#j-judul').click()");
    await sampai("!!document.querySelector('#pl-isi')", 'palet');
    hasil.palet = await nilai("var i = document.querySelector('#pl-isi'); i.value = 'argum'; i.oninput(); document.querySelector('#pl-hasil b').textContent");
    await nilai("document.querySelector('#pl-isi').onkeydown(new KeyboardEvent('keydown', { key: 'Enter' }))");
    await sampai("document.querySelector('#j-nama').textContent === 'argumen.jrg'", 'palet membuka berkas');
    // Argumen program dan mode uji.
    await nilai("document.querySelector('#tb-lain').click()");
    await sampai("!!document.querySelector('#o-argumen')", 'Lainnya');
    await nilai("document.querySelector('#o-argumen').click()");
    await sampai("!!document.querySelector('#ar-isi')", 'lembar argumen');
    await nilai(setel + "document.querySelector('#ar-isi').value = 'satu \"dua tiga\"'; document.querySelector('#ar-jalan').click()");
    await sampai(selesai, 'jalankan dengan argumen');
    hasil.argumen = await nilai("document.querySelector('#hasil').textContent.trim()");
    await nilai(setel + "document.querySelector('#tb-lain').click()");
    await sampai("!!document.querySelector('#o-uji')", 'Lainnya (uji)');
    await nilai("document.querySelector('#o-uji').click()");
    await sampai(selesai, 'mode uji');
    hasil.uji = await nilai("document.querySelector('#status').textContent.split(' · ').slice(0, 2).join(' · ') + '|' + document.querySelector('#hasil').textContent.trim().split('\\n').pop() + '|' + document.querySelectorAll('#hasil a[data-baris]').length");
    // Berkas biner dari luar: 700 KB acak, jadi tiga potongan; dibaca balik byte demi byte.
    hasil.unggah = await nilai(`(function () {
      var b = new Uint8Array(700000), x = 7;
      for (var i = 0; i < b.length; i++) { x = (x * 1103515245 + 12345) >>> 0; b[i] = x >>> 24; }
      return IDE.berkas.unggah(function (j, d) { return fetch(j, { method: 'POST', body: JSON.stringify(d) }).then(function (r) { return r.json(); }); },
        [new File([b], 'acak.bin')], 'dari-luar').then(function (h) {
        return fetch('/berkas/dari-luar/acak.bin').then(function (r) { return r.arrayBuffer(); }).then(function (a) {
          var u = new Uint8Array(a), sama = u.length === b.length;
          for (var i = 0; sama && i < u.length; i++) if (u[i] !== b[i]) sama = false;
          return h.jumlah + ' berkas, ' + u.length + ' byte, ' + (sama ? 'utuh' : 'RUSAK');
        });
      });
    })()`);
    // Zip folder (unduhan peramban): dibaca balik lewat direktori pusatnya.
    hasil.zip = await nilai(`IDE.berkas.zip([{ nama: 'a/x.txt', data: new TextEncoder().encode('halo') }, { nama: 'a/b/y.txt', data: new Uint8Array([0, 255, 1]) }])
      .arrayBuffer().then(function (b) {
        var u = new Uint8Array(b), v = new DataView(b), akhir = u.length - 22, n = v.getUint16(akhir + 10, true), p = v.getUint32(akhir + 16, true), keluar = [];
        for (var i = 0; i < n; i++) {
          var panjang = v.getUint16(p + 28, true), besar = v.getUint32(p + 24, true), lokal = v.getUint32(p + 42, true);
          var nama = new TextDecoder().decode(u.subarray(p + 46, p + 46 + panjang));
          var awal = lokal + 30 + v.getUint16(lokal + 26, true) + v.getUint16(lokal + 28, true);
          keluar.push(nama + '=' + (nama === 'a/x.txt' ? new TextDecoder().decode(u.subarray(awal, awal + besar)) : besar));
          p += 46 + panjang;
        }
        return keluar.join(',');
      })`);
    // Tombol kembali Android: menutup lembar yang terbuka (true), lalu tidak ada lagi yang ditutup (false).
    await nilai('(() => { for (var i = 0; i < 9 && window.jrgKembali(); i++); })()');
    await nilai("document.querySelector('#tb-lain').click()");
    hasil.kembali = await nilai("[window.jrgKembali(), document.querySelector('#lembar').classList.contains('is-on'), window.jrgKembali()].join(',')");
    // Latar: gambar lewat pemilih berkas Pengaturan, video lewat IDE.latar.simpan (direkam di halaman).
    const png = path.join(profil, 'latar.png');
    fs.writeFileSync(png, Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP8z8DAwMDAxMDAwMDAAAANHQEDasKb6QAAAABJRU5ErkJggg==', 'base64'));
    await nilai("document.querySelector('#tb-lain').click()");
    await nilai("[...document.querySelectorAll('#lembar .baris')].find((b) => /Pengaturan/.test(b.textContent)).click()");
    const dok = await kirim('DOM.getDocument', {}, sid);
    const inp = await kirim('DOM.querySelector', { nodeId: dok.result.root.nodeId, selector: '#lt-berkas' }, sid);
    await kirim('DOM.setFileInputFiles', { nodeId: inp.result.nodeId, files: [png] }, sid);
    await sampai("document.querySelector('#latar img') && document.querySelector('#latar img').naturalWidth > 0", 'latar gambar');
    hasil.latarGambar = await nilai("[document.querySelector('#latar img').naturalWidth, document.documentElement.classList.contains('ada-latar'), /rgba\\(.*0\\.75\\)/.test(getComputedStyle(document.querySelector('.atas')).backgroundColor)].join(',')");
    hasil.latarVideo = await nilai(`new Promise(function (ok) {
      var c = document.createElement('canvas'); c.width = 64; c.height = 48;
      var g = c.getContext('2d'), r = new MediaRecorder(c.captureStream(10), { mimeType: 'video/webm' }), potong = [], n = 0;
      var t = setInterval(function () { g.fillStyle = n++ % 2 ? '#f00' : '#00f'; g.fillRect(0, 0, 64, 48); }, 50);
      r.ondataavailable = function (e) { potong.push(e.data); };
      r.onstop = function () { clearInterval(t); ok(new Blob(potong, { type: 'video/webm' })); };
      r.start(); setTimeout(function () { r.stop(); }, 600);
    }).then(function (b) { return IDE.latar.simpan(new File([b], 'latar.webm', { type: 'video/webm' }), { latarTembus: 25, latarRedup: 35 }); })
      .then(function () { var v = document.querySelector('#latar video'); return new Promise(function (ok) {
        if (v.readyState >= 1) return ok(v);
        v.onloadedmetadata = function () { ok(v); }; v.onerror = function () { ok(v); };
      }); }).then(function (v) { return [v.muted, v.loop, v.error ? 'galat ' + v.error.code : 'terbaca', !document.querySelector('#latar img')].join(','); })`);
    await nilai("IDE.latar.hapus({})");
    hasil.latarHapus = await nilai("[document.documentElement.classList.contains('ada-latar'), !!document.querySelector('#latar video')].join(',')");
    await nilai('window.jrgKembali()');
    // Berkas besar: textarea cuma memegang jendela di sekitar kursor; ketikan di dekat akhir tersimpan
    // di baris yang benar, dan urung mengembalikan isi persis.
    const besar = Array.from({ length: 3000 }, (_, i) => 'fn f' + i + '() { wok(' + i + ') }').join('\n') + '\n';
    await simpan('besar.jrg', besar);
    await nilai('location.reload()');
    await new Promise((r) => setTimeout(r, 1500));
    await nilai("document.querySelector('#tb-menu').click()");
    await buka('besar.jrg');
    await nilai("(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'g', ctrlKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = ':2990'; i.dispatchEvent(new Event('input')); document.querySelector('#pl-hasil button').click(); })()");
    for (const c of 'xyz') await kirim('Input.insertText', { text: c }, sid);
    const simpanKetik = "(() => { document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true, bubbles: true, cancelable: true })); return new Promise((r) => setTimeout(r, 700)); })()";
    await nilai(simpanKetik);
    const sesudah = await nilai("fetch('/berkas/besar.jrg').then((r) => r.text())");
    await nilai("document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true, cancelable: true }))");
    await nilai(simpanKetik);
    const urung = await nilai("fetch('/berkas/besar.jrg').then((r) => r.text())");
    hasil.jendela = [await nilai("document.querySelector('.ed-ta').value.split('\\n').length"), sesudah.split('\n')[2989],
      urung === besar ? 'urung persis' : 'urung BEDA'].join('|');

    // 0.7 — alat kode. Kursor ditaruh lewat textarea (berkas kecil: jendela = seluruh berkas).
    const kursorDi = (teks, geser) => nilai("(() => { var ta = document.querySelector('.ed-ta'); ta.focus(); var i = ta.value.indexOf(" + JSON.stringify(teks) + ") + " + (geser || 0) + "; ta.setSelectionRange(i, i); ta.dispatchEvent(new Event('select')); return i; })()");
    const tombol = (key, mod) => nilai("document.dispatchEvent(new KeyboardEvent('keydown', Object.assign({ key: '" + key + "', bubbles: true, cancelable: true }, " + JSON.stringify(mod || {}) + ")))");
    const tunggu = (ms) => new Promise((r) => setTimeout(r, ms));
    await simpan('kd/a.jrg', 'package main\n\nimport "b"\n\npub fn main() {\n    wok(b::kalikan(2))\n}\n');
    await simpan('kd/b.jrg', 'package b\n\npub fn kalikan(x) { x * 3 }\n');
    await simpan('kd/rapi.jrg', 'package main\n\npub fn main() {\nif true {\n        wok("a {")\n  }\n}\n');
    await simpan('kd/todo.jrg', '// TODO: satu\nfn f() {} // FIXME dua\n// TODOS bukan\n');
    await simpan('kd/rw.jrg', 'versi satu\n');
    await simpan('kd/rw.jrg', 'versi dua\n');
    await simpan('kd/hilang.jrg', 'isi yang dihapus\n');
    await nilai("fetch('/api/hapus', { method: 'POST', body: JSON.stringify({ nama: 'kd/hilang.jrg' }) })");
    await nilai('location.reload()');
    await tunggu(1200);
    await nilai("document.querySelector('#tb-menu').click()");
    await buka('kd/a.jrg');
    await kursorDi('kalikan', 2);
    await tombol('F12');
    await sampai("document.querySelector('#j-nama').textContent === 'kd/b.jrg'", 'ke definisi');
    await tunggu(200);
    hasil.definisi = await nilai("document.querySelector('#j-nama').textContent + '|' + document.querySelector('#st-pos').textContent");
    await kursorDi('kalikan', 1);
    await tombol('F12', { shiftKey: true });
    await sampai("document.querySelectorAll('#kd-hasil [data-i]').length > 0", 'rujukan');
    hasil.rujukan = await nilai("document.querySelectorAll('#kd-hasil [data-i]').length + '|' + document.querySelector('#kd-hasil mark').textContent");
    await nilai('window.jrgKembali()');
    await tombol('F2');
    await sampai("!!document.querySelector('#gn-baru')", 'ganti nama');
    await nilai("document.querySelector('#gn-baru').value = 'kali3'; document.querySelector('#gn-ok').click()");
    await sampai("!document.querySelector('#lembar').classList.contains('is-on')", 'ganti nama selesai');
    await sampai("Promise.all(['kd/a.jrg', 'kd/b.jrg'].map((n) => fetch('/berkas/' + n).then((r) => r.text()))).then((d) => d.every((t) => /kali3/.test(t)))", 'ganti nama tersimpan');
    hasil.gantiNama = await nilai("Promise.all(['kd/a.jrg', 'kd/b.jrg'].map((n) => fetch('/berkas/' + n).then((r) => r.text()))).then((d) => d.map((t) => (t.match(/kali3/g) || []).length + '/' + (t.match(/kalikan/g) || []).length).join(','))");
    // Rapikan, pindah dan hapus baris, pasangan kurung.
    await buka('kd/rapi.jrg');
    await tombol('F', { shiftKey: true, altKey: true });
    await sampai("/\\n    if true/.test(document.querySelector('.ed-ta').value)", 'rapikan');
    hasil.rapikan = await nilai("JSON.stringify(document.querySelector('.ed-ta').value)");
    // Rapikan penuh lewat formatter JRG (jrgm fmt di mesin ini; jrgc --fmt kalau ada): spasi di dalam baris juga.
    await simpan('kd/acak.jrg', 'package main\npub   fn   main( ){\nlet    angka=[1,2,3]\n}\n');
    await nilai("document.querySelector('#tb-menu').click()");
    await buka('kd/acak.jrg');
    await tombol('F', { shiftKey: true, altKey: true });
    await sampai("/let angka = \\[1, 2, 3\\]/.test(document.querySelector('.ed-ta').value)", 'rapikan penuh');
    hasil.rapikanPenuh = await nilai("document.querySelector('.ed-ta').value.split('\\n')[1] + '|' + /jrgm fmt|jrgc --fmt/.test(document.querySelector('#toast').textContent)");
    await buka('kd/rapi.jrg');
    await kursorDi('wok');
    await nilai("document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', altKey: true, bubbles: true, cancelable: true }))");
    hasil.pindahBaris = await nilai("document.querySelector('.ed-ta').value.split('\\n').slice(2, 5).map((l) => l.trim()).join('|')");
    await nilai("document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 'K', ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))");
    hasil.hapusBaris = await nilai("String(document.querySelector('.ed-ta').value.split('\\n').length)");
    await kursorDi('{', 1);
    await sampai("document.querySelectorAll('.ed-kurung:not([hidden])').length > 0", 'sorot kurung');
    hasil.kurung = await nilai("String(document.querySelectorAll('.ed-kurung:not([hidden])').length)");
    // Keadaan per tab: urung dan kursor tetap ada sesudah pindah tab dan kembali.
    await kursorDi('fn main', 7);
    await kirim('Input.insertText', { text: 'Q' }, sid);
    await buka('kd/b.jrg');
    await nilai("document.querySelector('[role=tab][data-i=\"' + [...document.querySelectorAll('#tab [role=tab]')].findIndex((b) => /rapi/.test(b.textContent)) + '\"]').click()");
    await tunggu(150);
    const posTab = await nilai("document.querySelector('#st-pos').textContent");
    await nilai("document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true, cancelable: true }))");
    hasil.tabKeadaan = posTab + '|' + await nilai("document.querySelector('.ed-ta').value.indexOf('mainQ') < 0 && document.querySelector('.ed-ta').value.indexOf('main') > 0");
    // Cuplikan: `fori` + Tab.
    await kursorDi('pub fn', 0);
    for (const c of 'fori') await kirim('Input.insertText', { text: c }, sid);
    await sampai("document.querySelector('.saran') && !document.querySelector('.saran').hidden", 'saran cuplikan');
    hasil.cuplikanSaran = await nilai("document.querySelector('.saran .sr').textContent");
    await nilai("document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }))");
    for (const c of '10') await kirim('Input.insertText', { text: c }, sid);
    hasil.cuplikan = await nilai("JSON.stringify(document.querySelector('.ed-ta').value.split('\\n').slice(2, 4).join('\\n'))");
    // Riwayat: salinan 'versi satu' dibandingkan lalu dipulihkan.
    await buka('kd/rw.jrg');
    await nilai("(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, shiftKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = '>riwayat berkas'; i.dispatchEvent(new Event('input')); document.querySelector('#pl-hasil button').click(); })()");
    await sampai("document.querySelectorAll('#lembar [data-i]').length > 0", 'daftar riwayat');
    await nilai("document.querySelector('#lembar [data-i]').click()");
    await sampai("!!document.querySelector('#rw-pulih')", 'beda riwayat');
    hasil.riwayatBeda = await nilai("[document.querySelector('.bd-min').textContent, document.querySelector('.bd-plus').textContent].join('|')");
    await nilai("document.querySelector('#rw-pulih').click()");
    hasil.riwayat = await nilai("JSON.stringify(document.querySelector('.ed-ta').value)");
    // Berkas terhapus dipulihkan dari riwayatnya.
    await nilai("(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, shiftKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = '>berkas terhapus'; i.dispatchEvent(new Event('input')); document.querySelector('#pl-hasil button').click(); })()");
    await sampai("document.querySelectorAll('#lembar [data-i]').length > 0", 'daftar berkas terhapus');
    await nilai("[...document.querySelectorAll('#lembar [data-i]')].find((b) => /hilang/.test(b.textContent)).click()");
    await sampai("!!document.querySelector('#th-pulih')", 'pratinjau berkas terhapus');
    await nilai("document.querySelector('#th-pulih').click()");
    await sampai("document.querySelector('#j-nama').textContent === 'kd/hilang.jrg'", 'berkas dipulihkan');
    hasil.terhapus = await nilai("fetch('/berkas/kd/hilang.jrg').then((r) => r.text())");
    // TODO di semua berkas.
    await nilai("(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, shiftKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = '>daftar todo'; i.dispatchEvent(new Event('input')); document.querySelector('#pl-hasil button').click(); })()");
    await sampai("document.querySelectorAll('#kd-hasil [data-i]').length > 0", 'daftar TODO');
    hasil.todo = await nilai("[...document.querySelectorAll('#kd-hasil b')].map((b) => b.textContent).filter((t) => /^kd\\//.test(t)).join(',')");
    await nilai('window.jrgKembali()');

    // Cari dan ganti dengan pola + kata utuh; palet @ ke simbol.
    await buka('kd/b.jrg');
    await nilai("(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, shiftKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = '>cari dan ganti'; i.dispatchEvent(new Event('input')); document.querySelector('#pl-hasil button').click(); })()");
    await sampai("!!document.querySelector('#c-pola')", 'lembar cari');
    await nilai("document.querySelector('#c-pola').checked = true; document.querySelector('#c-utuh').checked = true; var k = document.querySelector('#c-kata'); k.value = 'x(\\\\s*)\\\\*'; k.dispatchEvent(new Event('input')); document.querySelector('#c-ganti').value = 'y$1/'; document.querySelector('#c-ganti').dispatchEvent(new Event('input')); document.querySelector('#c-semua').click()");
    hasil.cariPola = await nilai("document.querySelector('.ed-ta').value.split('\\n')[2]");
    await nilai("(() => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'o', ctrlKey: true, shiftKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = '@kali'; i.dispatchEvent(new Event('input')); return document.querySelector('#pl-hasil b').textContent + '|' + (document.querySelector('#pl-hasil button').click(), document.querySelector('#st-pos').textContent); })()");
    hasil.paletSimbol = await nilai("document.querySelector('#st-pos').textContent");
    // Profil.
    await buka('kd/b.jrg');
    await simpan('kd/p.jrg', 'fn f(n) { n * 2 }\npub fn main() { wok(f(21)) }\n');
    await nilai('location.reload()');
    await tunggu(500);
    await sampai("document.querySelector('#j-nama').textContent === 'kd/b.jrg'", 'sesi dipulihkan');
    hasil.sesi = await nilai("[...document.querySelectorAll('#tab [role=tab]')].map((b) => b.textContent).filter((t) => /^(a|b|rapi|rw|hilang)\\.jrg$/.test(t)).join(',') + '|' + document.querySelector('#j-nama').textContent");
    await nilai("document.querySelector('#tb-menu').click()");
    await buka('kd/p.jrg');
    await nilai(setel + "document.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, shiftKey: true, bubbles: true })); var i = document.querySelector('#pl-isi'); i.value = '>profil'; i.dispatchEvent(new Event('input')); document.querySelector('#pl-hasil button').click();");
    await sampai(selesai, 'profil selesai');
    hasil.profil = await nilai("document.querySelector('#status').textContent.split(' · ')[0] + '|' + !!document.querySelector('#hasil .profil .pf-fn') + '|' + /(^|\\n)42\\n/.test(document.querySelector('#hasil').textContent) + '|' + [...document.querySelectorAll('#hasil .pf-nama code')].map((e) => e.textContent).sort().join(',')");

    // Terminal (0.8): PTY sungguhan, warna, Ctrl dari baris tombol, layar alternatif, buka lewat OSC,
    // ukuran sampai ke stty, huruf lebar, jalankan di terminal, sesi pulih sesudah muat ulang.
    const ketik = async (t) => { for (const c of t) await kirim('Input.insertText', { text: c }, sid); };
    const enter = () => kirim('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }, sid);
    // Layar dan riwayat gulirnya: ubah ukuran (mis. keluar dari layar penuh) memindah baris atas ke riwayat.
    const layar = "document.querySelector('.t-layar:not([hidden])').textContent";
    const perintahT = async (baris, tanda, apa) => { await ketik(baris); await enter(); await sampai(layar + '.indexOf(' + JSON.stringify(tanda) + ') >= 0', apa); };
    await nilai("document.querySelector('#bilah [data-alat=konsol]').click()");
    await sampai("!!document.querySelector('.t-layar:not([hidden]) .vkur')", 'kursor terminal');
    // Terminal memenuhi layar HP: kepala IDE disembunyikan, laci berkas dibuka dari bilah sesi dan benar-benar tampil.
    await nilai("document.querySelector('#t-laci').click()");
    await tunggu(400);
    hasil.termLaci = await nilai("(() => { var a = document.querySelector('#app'), r = document.querySelector('#laci').getBoundingClientRect(); return [a.classList.contains('alat-penuh'), getComputedStyle(document.querySelector('.atas')).display, r.left >= 0 && r.width > 200 && r.height > 300, document.elementFromPoint(r.left + 40, r.top + 200).closest('#laci') !== null].join('|'); })()");
    await nilai("document.querySelector('#tirai-laci').click()");
    await perintahT("printf '\\033[1;31mmerah\\033[0m %s\\n' \"$((6*7))\"", 'merah 42', 'warna');
    hasil.termWarna = await nilai("(() => { var s = [...document.querySelectorAll('.t-layar:not([hidden]) .t-scr .vf1')].filter((e) => e.textContent === 'merah')[0]; return s ? s.className + '|' + getComputedStyle(s).fontWeight : 'tidak ada'; })()");
    await ketik('sleep 30'); await enter(); await tunggu(400);
    const tekanKunci = (k) => nilai("(() => { var b = document.querySelector('#t-kunci [data-kunci=\"" + k + "\"]'); b.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, clientX: 5, clientY: 50 })); b.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientX: 5, clientY: 50 })); return b.getAttribute('data-n'); })()");
    hasil.termCtrl = String(await tekanKunci('CTRL'));
    await ketik('c');
    await perintahT('echo kode=$?', 'kode=130', 'Ctrl+C dari baris tombol');
    hasil.termCtrl += '|130';
    await ketik("printf '\\033[?1049hLAYAR-ALT-%s\\n' $((3*3)); read x; printf '\\033[?1049l'"); await enter();
    await sampai("document.querySelector('.t-layar:not([hidden])').classList.contains('alt') && " + layar + ".indexOf('LAYAR-ALT-9') >= 0", 'layar alternatif');
    await enter();
    await sampai("!document.querySelector('.t-layar:not([hidden])').classList.contains('alt')", 'layar alternatif ditutup');
    hasil.termAlt = 'masuk|keluar|' + await nilai(layar + ".indexOf('LAYAR-ALT-9') < 0");
    await perintahT('stty size; echo ukuran-ok', 'ukuran-ok', 'stty size');
    hasil.termUkuran = await nilai("(() => { var v = IDE.term.sesi()[0].vt, m = " + layar + ".match(/(\\d+) (\\d+)\\s*ukuran-ok/); return m ? (m[1] == v.baris && m[2] == v.kolom) + '|' + v.kolom : 'tidak ada: ' + " + layar + ".slice(-200); })()");
    await perintahT("printf '\\xe6\\xbc\\xa2X\\n'", '漢X', 'huruf lebar');
    hasil.termLebar = await nilai("[...document.querySelectorAll('.t-layar:not([hidden]) .vw2')].some((e) => e.textContent === '漢') + ''");
    // Tempelan banyak baris dari papan ketik HP (sisipan tanpa event paste) tetap banyak baris; dulu `<input>` menyambungnya.
    await kirim('Input.insertText', { text: 'echo tp-$((2*1))\necho tq-$((2*2))\n' }, sid); await enter();
    await sampai(layar + ".indexOf('tq-4') >= 0", 'tempelan banyak baris');
    hasil.termTempel = await nilai(layar + ".indexOf('tp-2echo') < 0 && " + layar + ".indexOf('tp-2') >= 0") + '';
    // Konsol berbaris: tempelan tiga baris tinggal di kotak (yang tumbuh), lalu dijalankan sebagai satu skrip.
    await nilai("document.querySelector('#t-menu').click()"); await tunggu(200);
    await nilai("document.querySelector('#tm-baris').click()"); await tunggu(300);
    await nilai("document.querySelector('#k-masuk').focus()");
    await kirim('Input.insertText', { text: 'echo ka-$((3*1))\necho kb-$((3*2))\necho kc-ok' }, sid);
    const tinggiKotak = await nilai("document.querySelector('#k-masuk').getBoundingClientRect().height > 50");
    await nilai("document.querySelector('#k-kirim').click()");
    await sampai("/ka-3\\nkb-6\\nkc-ok/.test(document.querySelector('#k-layar').textContent)", 'konsol berbaris menjalankan tempelan');
    hasil.konsolTempel = tinggiKotak + '|ok';
    await nilai("document.querySelector('#k-ke-term').click()"); await tunggu(300);
    await perintahT('echo isi-osc > dibuka.txt; buka dibuka.txt; echo buka-ok', 'buka-ok', 'buka lewat OSC');
    await sampai("document.querySelector('#j-nama').textContent === 'dibuka.txt'", 'berkas terbuka dari terminal');
    hasil.termBuka = await nilai("document.querySelector('#j-nama').textContent");
    await simpan('kd/term.jrg', 'pub fn main() {\n    wok("di-terminal", 6 * 7)\n}\n');
    await perintahT('buka kd/term.jrg; echo buka2-ok', 'buka2-ok', 'buka kd/term.jrg');
    await sampai("document.querySelector('#j-nama').textContent === 'kd/term.jrg'", 'kd/term.jrg terbuka');
    await nilai("document.querySelector('#t-turun').click()");
    await nilai("document.querySelector('.ed-ta').focus(); document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, shiftKey: true, bubbles: true }))");
    await sampai(layar + ".indexOf('di-terminal 42') >= 0", 'jalankan di terminal');
    hasil.termJalan = 'di-terminal 42';
    await nilai("document.querySelector('#t-baru').click()");
    await sampai("document.querySelectorAll('#t-tab [role=tab]').length === 2 && !!document.querySelector('.t-layar:not([hidden]) .vkur')", 'sesi kedua');
    await perintahT('echo sesi-$((1+1)); bahasa; echo bhs-ok', 'bhs-ok', 'sesi kedua menjawab');
    await nilai('location.reload()');
    await tunggu(800);
    await sampai("!!document.querySelector('#bilah [data-alat=konsol]')", 'muat ulang');
    // Tadi di terminal: panelnya terbuka lagi sendiri (Android membuat ulang aplikasinya), tanpa diketuk.
    await sampai("!document.querySelector('#konsol').hidden && document.querySelectorAll('#t-tab [role=tab]').length === 2 && " + layar + ".indexOf('sesi-2') >= 0", 'sesi pulih');
    await nilai("document.querySelector('#t-tab [data-i=\"0\"]').click()");
    await sampai(layar + ".indexOf('di-terminal 42') >= 0", 'isi sesi pertama pulih');
    hasil.termPulih = '2|sesi-2|di-terminal 42';
    // Memulihkan tidak menjalankan lagi perintah IDE di keluaran lama (dulu `bahasa` mengetik "0" ke shell).
    await tunggu(800);
    hasil.termUlang = await nilai("[...document.querySelectorAll('.t-layar')].some((e) => /not found|tidak ditemukan/.test(e.textContent)) + '|' + document.querySelector('#j-nama').textContent");
    await nilai("document.querySelector('#t-tab [data-x=\"1\"]').click()");
    await sampai("document.querySelectorAll('#t-tab [role=tab]').length === 1", 'sesi ditutup');
    hasil.termTutup = await nilai("fetch('/api/pty').then((r) => r.json()).then((d) => String(d.sesi.length))");
    // Bahasa terkompilasi (kalau kompilator C ada di mesin ini) lewat tombol Jalankan, galatnya bertaut.
    const adaC = await nilai("fetch('/api/info').then((r) => r.json()).then((d) => !!d.bahasa.filter((b) => b.id === 'c' && b.program).length)");
    if (adaC) {
      await nilai("document.querySelector('#t-turun').click()");
      await simpan('kd/c1.c', '#include <stdio.h>\nint main(void) { printf("C %d\\n", 6 * 7); return 0; }\n');
      await simpan('kd/c2.c', 'int main(void) {\n    return x;\n}\n');
      const bukaT = async (n) => {
        await nilai("document.querySelector('#bilah [data-alat=konsol]').click()");
        await sampai("!!document.querySelector('.t-layar:not([hidden]) .vkur')", 'terminal');
        await perintahT('buka ' + n + '; echo dibuka-' + n.length, 'dibuka-' + n.length, 'buka ' + n);
        await sampai("document.querySelector('#j-nama').textContent === '" + n + "'", n + ' terbuka');
        await nilai("document.querySelector('#t-turun').click()");
      };
      await bukaT('kd/c1.c');
      await nilai("document.querySelector('#tb-jalan').click()");
      await sampai(selesai, 'C selesai');
      const c1 = await nilai("document.querySelector('#hasil').textContent");
      await bukaT('kd/c2.c');
      await nilai("document.querySelector('#tb-jalan').click()");
      await sampai("document.querySelector('#hasil a[data-berkas]') && document.querySelector('#status').getAttribute('data-s') === 'gagal'", 'galat C bertaut');
      hasil.bahasaC = c1.trim() + '|' + await nilai("(() => { var a = document.querySelector('#hasil a[data-berkas]'); return a ? a.getAttribute('data-berkas') + ':' + a.getAttribute('data-baris') : 'tanpa tautan'; })()");
    } else hasil.bahasaC = 'tanpa';
    // 0.9: simbol semua berkas (Ctrl+T), perintah proyek di terminal, ukuran huruf (Ctrl+= dan cubit),
    // rapikan saat simpan, cadangan pengaturan, tutup tab lain.
    await nilai("document.querySelector('#bilah [data-alat=keluar]').click()");
    await simpan('tg/jrgm.toml', '[package]\nname = "tg"\nentry = "main.jrg"\n');
    await simpan('tg/main.jrg', 'package main\n\nimport "alat"\n\npub fn main() {\n    wok("tugas", alat::kali_dua(21))\n}\n');
    await simpan('tg/alat.jrg', 'package alat\n\npub fn kali_dua_unik(x) { x * 2 }\npub fn kali_dua(x) { kali_dua_unik(x) }\n');
    await simpan('tg/acak.jrg', 'package main\npub fn main() {\nif true {\nwok("a")\n}\n}\n');
    await nilai('location.reload()');
    await new Promise((r) => setTimeout(r, 800));
    await sampai("document.querySelectorAll('#tab [role=tab]').length > 0 && !!document.querySelector('#laci [data-buka=\"tg/acak.jrg\"]')", 'muat ulang tg');
    await nilai("document.dispatchEvent(new KeyboardEvent('keydown', { key: 't', ctrlKey: true, bubbles: true }))");
    await sampai("document.querySelector('#pl-isi') && document.querySelector('#pl-isi').value === '#' && document.querySelectorAll('#pl-hasil [data-pl]').length > 0", 'simbol semua berkas');
    await nilai("var i = document.querySelector('#pl-isi'); i.value = '#kali_dua_unik'; i.oninput(); document.querySelector('#pl-hasil [data-pl]').click()");
    await sampai("document.querySelector('#j-nama').textContent === 'tg/alat.jrg'", 'simbol dibuka');
    hasil.simbolSemua = await nilai("document.querySelector('#j-nama').textContent + '|' + document.querySelector('#st-pos').textContent");
    await nilai("document.querySelector('#tb-lain').click()");
    await nilai("document.querySelector('#o-tugas').click()");
    await sampai("document.querySelectorAll('[data-tg-saran]').length > 0", 'saran perintah proyek');
    await nilai("document.querySelector('#tg-baris').value = 'jrgc main.jrg --vm'; document.querySelector('#tg-tambah').click()");
    await sampai("!!document.querySelector('[data-tg=\"0\"]')", 'perintah tersimpan');
    await nilai("document.querySelector('[data-tg=\"0\"]').click()");
    await sampai("[...document.querySelectorAll('.t-layar')].some((e) => /tugas 42/.test(e.textContent))", 'perintah proyek di terminal');
    hasil.tugas = await nilai("(function (t) { return Object.keys(t).join() + ':' + t.tg.map(function (d) { return d.c; }).join(); })(JSON.parse(localStorage.getItem('jrgide.atur')).tugas) + '|' + [...document.querySelectorAll('.t-layar')].some((e) => /cd ~\\/'tg' && jrgc main\\.jrg --vm/.test(e.textContent))");
    await nilai("document.querySelector('#bilah [data-alat=keluar]').click()");
    const huruf = "JSON.parse(localStorage.getItem('jrgide.atur')).huruf + '/' + getComputedStyle(document.querySelector('.ed-ta')).fontSize";
    await nilai("document.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true, bubbles: true }))");
    await nilai("document.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true, bubbles: true }))");
    const hurufNaik = await nilai(huruf);
    await nilai("document.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true, bubbles: true }))");
    await kirim('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 }, sid);
    const kotak = JSON.parse(await nilai("JSON.stringify(document.querySelector('#ed').getBoundingClientRect())"));
    const cx = Math.round(kotak.x + kotak.width / 2), cy = Math.round(kotak.y + 100);
    const sentuh = (type, d) => kirim('Input.dispatchTouchEvent', { type, touchPoints: type === 'touchEnd' ? [] : [{ x: cx - d, y: cy, id: 1 }, { x: cx + d, y: cy, id: 2 }] }, sid);
    await sentuh('touchStart', 50);
    for (let d = 55; d <= 75; d += 5) { await sentuh('touchMove', d); await new Promise((r) => setTimeout(r, 40)); }
    await sentuh('touchEnd');
    await new Promise((r) => setTimeout(r, 300));
    hasil.huruf = hurufNaik + '|' + await nilai(huruf);
    await nilai("document.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true, bubbles: true }))");
    await kirim('Emulation.setTouchEmulationEnabled', { enabled: false }, sid);
    await buka('tg/acak.jrg');
    await nilai("document.querySelector('#tb-lain').click()");
    await nilai("document.querySelector('#o-atur').click()");
    await nilai("document.querySelector('[data-k=rapiSimpan][data-v=true]').click()");
    await nilai("document.querySelector('#le-tutup').click()");
    await nilai("document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true, bubbles: true, cancelable: true }))");
    await sampai("fetch('/api/baca?nama=tg/acak.jrg').then((r) => r.json()).then((d) => /\\n    if true/.test(d.isi))", 'rapikan saat simpan');
    hasil.rapiSimpan = await nilai("fetch('/api/baca?nama=tg/acak.jrg').then((r) => r.json()).then((d) => d.isi.split('\\n').length + ':' + d.isi.split('\\n')[3])");
    await nilai("document.querySelector('#tb-lain').click()");
    await nilai("document.querySelector('#o-atur').click()");
    await nilai("document.querySelector('[data-k=rapiSimpan][data-v=false]').click()");
    await nilai("document.querySelector('#at-cadang').click()");
    await sampai("fetch('/api/baca?nama=pengaturan-ide.json').then((r) => r.ok)", 'cadangan pengaturan');
    const temaAwal = await nilai("document.documentElement.getAttribute('data-tema')");
    await nilai("document.querySelector('[data-k=tema][data-v=senja]').click()");
    await nilai("document.querySelector('#at-pulih').click()");
    await sampai("document.documentElement.getAttribute('data-tema') === '" + temaAwal + "'", 'pengaturan dipulihkan');
    hasil.cadangan = temaAwal + '|' + await nilai("fetch('/api/baca?nama=pengaturan-ide.json').then((r) => r.json()).then((d) => { var a = JSON.parse(d.isi); return ('tugas' in a) + ',' + ('kursor' in a); })");
    await nilai("document.querySelector('#le-tutup').click()");
    await nilai("document.querySelector('#j-judul').click()");
    await nilai("var i = document.querySelector('#pl-isi'); i.value = '>tutup tab lain'; i.oninput(); document.querySelector('#pl-hasil [data-pl]').click()");
    await sampai("document.querySelectorAll('#tab [role=tab]').length === 1", 'tutup tab lain');
    hasil.tutupLain = await nilai("document.querySelector('#tab [role=tab]').textContent.trim()");
    // Jalankan pilihan (import ikut), F8 ke masalah berikutnya, bandingkan dua berkas.
    await simpan('tg/pilih.jrg', 'package main\n\nimport "alat"\n\npub fn main() {\n    wok("semua")\n    let x = alat::kali_dua(5)\n    wok("pilihan", x + 1)\n}\n');
    await simpan('tg/salah.jrg', 'package main\n\npub fn main() {\n    wok(1 +)\n}\n');
    await nilai('location.reload()');
    await new Promise((r) => setTimeout(r, 800));
    await sampai("!!document.querySelector('#laci [data-buka=\"tg/pilih.jrg\"]')", 'muat ulang pilihan');
    await buka('tg/salah.jrg');
    await sampai("document.querySelector('#bl-masalah-n').textContent !== ''", 'masalah salah.jrg');
    await buka('tg/pilih.jrg');
    await nilai("(() => { const ta = document.querySelector('.ed-ta'), v = ta.value, a = v.indexOf('    let x'); ta.focus(); ta.setSelectionRange(a, v.indexOf('}\\n', a)); })()");
    await nilai(setel + "document.querySelector('.ed-ta').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, altKey: true, bubbles: true, cancelable: true }))");
    await sampai(selesai, 'jalankan pilihan');
    hasil.pilihan = await nilai("document.querySelector('#status').getAttribute('data-s') + '|' + document.querySelector('#hasil').textContent.trim()");
    await nilai("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'F8', bubbles: true }))");
    await sampai("document.querySelector('#j-nama').textContent === 'tg/salah.jrg'", 'F8');
    hasil.f8 = await nilai("document.querySelector('#st-pos').textContent");
    await nilai("document.querySelector('#tb-lain').click()");
    await nilai("document.querySelector('#o-banding').click()");
    await nilai("var i = document.querySelector('#bd-cari'); i.value = 'tg/pilih'; i.oninput(); document.querySelector('#bd-daftar [data-bd]').click()");
    await sampai("!!document.querySelector('#lembar .beda')", 'bandingkan');
    hasil.banding = await nilai("document.querySelector('#lembar .catatan').textContent.replace(/\\s+/g, ' ')");
    await nilai("document.querySelector('#le-tutup').click()");
    hasil.galat = galat;
    ws.close();
    akhiri(galat.length ? 1 : 0, JSON.stringify(hasil));
  };
});
