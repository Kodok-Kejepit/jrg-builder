// perkakas/ide/uji_peramban.js — JRG IDE di chromium headless lewat DevTools Protocol, ukuran HP:
// jalankan berkas pertama, lalu `masukan.jrg` dengan isi stdin; laporkan keluaran dan galat JS.
// Pakai: node uji_peramban.js <chromium> <url> [tangkapan.png]
'use strict';
const { spawn } = require('child_process');
const fs = require('fs'), os = require('os'), path = require('path');
const [krom, url, png] = process.argv.slice(2);
const profil = fs.mkdtempSync(path.join(os.tmpdir(), 'jrg-ide-'));
// `--password-store=basic`: tanpa itu chromium menunggu keyring desktop dan bisa menggantung.
const proses = spawn(krom, ['--headless', '--disable-gpu', '--no-sandbox', '--password-store=basic', '--remote-debugging-port=0',
  '--user-data-dir=' + profil, 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
const akhiri = (kode, pesan) => {
  if (pesan) process.stdout.write(pesan + '\n');
  proses.kill('SIGKILL');
  fs.rmSync(profil, { recursive: true, force: true });
  process.exit(kode);
};
const batas = setTimeout(() => akhiri(2, 'BATAS WAKTU'), 480000);
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
    const nilai = async (expr) => {
      const r = await kirim('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }, sid);
      return r.result && r.result.result ? r.result.result.value : undefined;
    };
    const sampai = async (expr, apa) => {
      for (let i = 0; i < 160; i++) {
        if (await nilai(expr)) return;
        await new Promise((r) => setTimeout(r, 250));
      }
      akhiri(1, JSON.stringify({ gagal: 'menunggu ' + apa, galat }));
    };
    await kirim('Runtime.enable', {}, sid);
    await kirim('Emulation.setDeviceMetricsOverride', { width: 390, height: 800, deviceScaleFactor: 2, mobile: true }, sid);
    await kirim('Page.navigate', { url }, sid);
    const selesai = "['ok','gagal'].indexOf(document.querySelector('#status').getAttribute('data-s')) >= 0";
    await sampai("document.querySelectorAll('#tab [role=tab]').length > 0", 'tab pertama');
    const hasil = { berkas: await nilai("document.querySelector('#j-nama').textContent"), ket: await nilai("document.querySelector('#j-ket').textContent") };
    hasil.sorot = await nilai("document.querySelectorAll('.ed-hl .hk').length");
    await nilai("document.querySelector('#tb-jalan').click()");
    await sampai(selesai, 'jalankan pertama');
    hasil.keluaran1 = await nilai("document.querySelector('#hasil').textContent");
    hasil.status1 = await nilai("document.querySelector('#status').textContent");
    await nilai("document.querySelector('#laci [data-buka=\"masukan.jrg\"]').click()");
    await sampai("document.querySelector('#j-nama').textContent === 'masukan.jrg'", 'buka masukan.jrg');
    await nilai("document.querySelector('#masukan').value = 'Budi'; document.querySelector('#status').setAttribute('data-s', ''); document.querySelector('#tb-jalan').click()");
    await sampai(selesai, 'jalankan kedua');
    hasil.keluaran2 = await nilai("document.querySelector('#hasil').textContent");
    // Masukan di TENGAH jalan: pertanyaannya harus tampil selagi program menunggu.
    await nilai("fetch('/api/simpan', { method: 'POST', body: JSON.stringify({ nama: 'putar.sh', isi: 'while true; do echo x; sleep 0.2; done\\n' }) }).then((r) => r.ok)");
    await nilai("fetch('/api/simpan', { method: 'POST', body: JSON.stringify({ nama: 'uji_six.py', isi: 'import six\\nprint(six.__version__)\\n' }) }).then((r) => r.ok)");
    const setel = "document.querySelector('#status').setAttribute('data-s', ''); document.querySelector('#hasil').textContent = ''; ";
    await nilai(setel + "document.querySelector('#masukan').value = ''; document.querySelector('#tb-jalan').click()");
    await sampai("document.querySelector('#hasil').textContent.indexOf('Siapa namamu?') >= 0 && !document.querySelector('#tb-kirim').hidden", 'pertanyaan tampil selagi jalan');
    hasil.tombol = await nilai("document.querySelector('#tb-jalan').getAttribute('aria-label')");
    await nilai("document.querySelector('#masukan').value = 'Sari'; document.querySelector('#tb-kirim').click()");
    await sampai(selesai, 'jalankan ketiga');
    hasil.keluaran3 = await nilai("document.querySelector('#hasil').textContent");
    // Hentikan: program yang berputar selamanya berhenti lewat tombolnya.
    await sampai("!!document.querySelector('#laci [data-buka=\"putar.sh\"]')", 'putar.sh di laci');
    await nilai("document.querySelector('#laci [data-buka=\"putar.sh\"]').click()");
    await sampai("document.querySelector('#j-nama').textContent === 'putar.sh'", 'buka putar.sh');
    await nilai(setel + "document.querySelector('#tb-jalan').click()");
    await sampai("document.querySelector('#hasil').textContent.indexOf('x\\nx') >= 0", 'keluaran mengalir');
    await nilai("document.querySelector('#tb-jalan').click()");
    await sampai(selesai, 'berhenti sesudah Hentikan');
    hasil.status4 = await nilai("document.querySelector('#status').textContent.split(' · ')[0]");
    hasil.cari = await nilai("fetch('/api/cari?kata=NAMAMU').then((r) => r.json()).then((d) => d.map((x) => x.nama + ':' + x.baris).join(','))");
    // Bahasa lain, kalau penjalannya ada di mesin ini.
    hasil.lua = 'tanpa';
    if (await nilai("fetch('/api/info').then((r) => r.json()).then((d) => d.bahasa.some((b) => b.id === 'lua' && b.program))")) {
      await sampai("!document.querySelector('#tb-jalan').classList.contains('is-henti')", 'tombol kembali');
      await nilai("document.querySelector('#laci [data-buka=\"halo.lua\"]').click()");
      await sampai("document.querySelector('#j-nama').textContent === 'halo.lua'", 'buka halo.lua');
      await nilai(setel + "document.querySelector('#tb-jalan').click()");
      await sampai(selesai, 'jalankan Lua');
      hasil.lua = await nilai("document.querySelector('#hasil').textContent.trim().split('\\n').pop()");
    }
    // Bahasa yang DIPASANG dari halaman, kalau pelayannya punya cermin paket (tanpa internet).
    const pasang = async (id, berkas, masukan) => {
      await sampai("!document.querySelector('#tb-jalan').classList.contains('is-henti')", 'tombol kembali');
      await nilai("document.querySelector('#tb-lain').click()");
      await sampai("!!document.querySelector('#o-bahasa')", 'lembar Lainnya');
      await nilai("document.querySelector('#o-bahasa').click()");
      await sampai("!!document.querySelector('#lembar [data-paket=\"" + id + "\"][data-aksi=\"pasang\"]')", 'tombol Pasang ' + id);
      await nilai("document.querySelector('#lembar [data-paket=\"" + id + "\"]').click()");
      await sampai("IDE.paket.terpasang('" + id + "') || /Gagal/.test(document.querySelector('#p-kabar').textContent)", 'memasang ' + id);
      if (!await nilai("IDE.paket.terpasang('" + id + "')")) return 'gagal pasang: ' + await nilai("document.querySelector('#p-kabar').textContent");
      await nilai("document.querySelector('#le-tutup').click()");
      await nilai("document.querySelector('#laci [data-buka=\"" + berkas + "\"]').click()");
      await sampai("document.querySelector('#j-nama').textContent === '" + berkas + "'", 'buka ' + berkas);
      await nilai(setel + "document.querySelector('#masukan').value = '" + masukan + "'; document.querySelector('#tb-jalan').click()");
      await sampai(selesai, 'jalankan ' + berkas);
      return await nilai("document.querySelector('#hasil').textContent.trim().split('\\n')[0]");
    };
    hasil.ts = hasil.py = hasil.lain = 'tanpa';
    const cermin = await nilai("fetch('/api/info').then((r) => r.json()).then((d) => !!d.cermin_paket)");
    const jalankan = async (berkas, apa, tunggu) => {
      await sampai("!document.querySelector('#tb-jalan').classList.contains('is-henti')", 'tombol kembali');
      await sampai("!!document.querySelector('#laci [data-buka=\"" + berkas + "\"]')", berkas + ' di laci');
      await nilai("document.querySelector('#laci [data-buka=\"" + berkas + "\"]').click()");
      await sampai("document.querySelector('#j-nama').textContent === '" + berkas + "'", 'buka ' + berkas);
      await nilai(setel + "document.querySelector('#masukan').value = ''; document.querySelector('#tb-jalan').click()");
      await sampai(tunggu || selesai, apa);
      return (await nilai("document.querySelector('#status').textContent.split(' · ')[0]")) + '|' + (await nilai("document.querySelector('#hasil').textContent.trim()"));
    };
    if (cermin) {
      hasil.ts = await pasang('ts', 'halo.ts', '');
      hasil.py = await pasang('python', 'halo.py', 'Budi');
      // Pustaka yang diimpor tapi belum ada dipasang sendiri dari daftar Pyodide (sidik diperiksa).
      hasil.six = (await jalankan('uji_six.py', 'impor six')).split('|')[1];
    }
    if (cermin && await nilai("fetch('/cermin/ruby/opal.min.js').then((r) => r.ok && fetch('/cermin/prolog/core.js')).then((r) => r.ok)")) {
      hasil.lain = [await pasang('ruby', 'halo.rb', 'Budi'), await pasang('coffee', 'halo.coffee', ''),
        await pasang('scheme', 'halo.scm', ''), await pasang('prolog', 'halo.pl', '')].join(' / ');
    }
    // Proyek: Jalankan dari berkas mana pun di dalamnya menjalankan berkas UTAMA proyeknya.
    const proyek = async (jenis, nama, pembantu, tunggu) => {
      await sampai("!document.querySelector('#tb-jalan').classList.contains('is-henti')", 'tombol kembali');
      await nilai("document.querySelector('#l-baru').click()");
      await sampai("!!document.querySelector('#lembar [data-proyek=\"" + jenis + "\"]')", 'lembar Baru');
      await nilai("document.querySelector('#b-nama').value = '" + nama + "'; document.querySelector('#lembar [data-proyek=\"" + jenis + "\"]').click()");
      await sampai("document.querySelector('#j-nama').textContent.indexOf('" + nama + "/') === 0", 'proyek ' + nama + ' terbuka');
      return await jalankan(nama + '/' + pembantu, 'jalankan proyek ' + nama, tunggu);
    };
    hasil.proyek = await proyek('jrg', 'cobajrg', 'src/hitung.jrg');
    if (cermin) hasil.proyekPy = await proyek('py', 'cobapy', 'alat/hitung.py');
    if (cermin) hasil.proyekTs = (await nilai("fetch('/api/simpan', { method: 'POST', body: JSON.stringify({ nama: 'cobats/main.ts', isi: 'import { dua } from \"./bantu\";\\nconsole.log(\"dua kali 21 =\", dua(21));\\n' }) }).then(() => fetch('/api/simpan', { method: 'POST', body: JSON.stringify({ nama: 'cobats/bantu.ts', isi: 'export function dua(x: number): number { return x * 2; }\\n' }) })).then(() => fetch('/api/proyek', { method: 'POST', body: JSON.stringify({ akar: 'cobats', utama: 'main.ts' }) })).then((r) => r.ok)"))
      ? (await jalankan('halo.jrg', 'segarkan laci'), await jalankan('cobats/bantu.ts', 'jalankan proyek TypeScript')) : 'gagal menyiapkan';
    // Pratinjau web memuat berkas tetangganya; pita simbol: menggulir tidak mengetik, mengetuk mengetik.
    await proyek('web', 'cobaweb', 'gaya.css', "document.querySelector('#hasil').tagName === 'IFRAME'");
    hasil.web = await nilai("(document.querySelector('#hasil').tagName + ' ' + document.querySelector('#hasil').getAttribute('src')).split('?')[0]") +
      ' ' + await nilai("fetch('/berkas/cobaweb/gaya.css').then((r) => r.text()).then((t) => t.indexOf('font-family') >= 0)");
    hasil.simbol = await nilai(`(function () {
      var b = document.querySelectorAll('#simbol button')[3], ta = document.querySelector('#ed textarea'), k = b.getBoundingClientRect();
      function pe(jenis, x) { b.dispatchEvent(new PointerEvent(jenis, { bubbles: true, cancelable: true, clientX: x, clientY: k.top + 5 })); }
      ta.focus();
      var awal = ta.value.length;
      pe('pointerdown', k.left + 5); pe('pointermove', k.left - 40); pe('pointerup', k.left - 40);
      var gulir = ta.value.length - awal;
      pe('pointerdown', k.left + 5); pe('pointerup', k.left + 6);
      return 'gulir ' + gulir + ', ketuk ' + (ta.value.length - awal - gulir);
    })()`);
    // Pencarian bahasa menjawab jujur untuk yang belum bisa.
    await nilai("document.querySelector('#tb-lain').click()");
    await sampai("!!document.querySelector('#o-bahasa')", 'lembar Lainnya');
    await nilai("document.querySelector('#o-bahasa').click()");
    await sampai("!!document.querySelector('#bh-cari')", 'lembar Bahasa');
    hasil.cariBahasa = await nilai("(function () { var i = document.querySelector('#bh-cari'); i.value = 'golang'; i.oninput(); return Array.prototype.map.call(document.querySelectorAll('#bh-isi .baris'), function (b) { return b.querySelector('b').textContent + ':' + b.querySelector('.cip').textContent; }).join(','); })()");
    await nilai("document.querySelector('#le-tutup').click()");
    hasil.tab = await nilai("document.querySelectorAll('#tab [role=tab]').length");
    // Konsol: shell tetap hidup antar-perintah, `cd` diingat, salah sintaks tidak mematikannya,
    // Hentikan mematikan perintahnya lalu shell baru mulai di direktori yang sama.
    await nilai("document.querySelector('#bilah [data-alat=konsol]').click()");
    const layar = "document.querySelector('#k-layar').textContent";
    const siap = "document.querySelector('#k-henti').hidden && /\\$$/.test(document.querySelector('#k-label').textContent)";
    const perintah = async (baris, tanda) => {
      await nilai("document.querySelector('#k-masuk').value = " + JSON.stringify(baris) + "; document.querySelector('#k-kirim').click()");
      await sampai(siap + ' && ' + layar + '.indexOf(' + JSON.stringify(tanda) + ') >= 0', 'konsol: ' + baris);
    };
    await perintah('echo sa"tu"; cd cobajrg', 'satu');
    const k = [await nilai("document.querySelector('#k-label').textContent")];
    await perintah('jrgc src/main.jrg', '5! = 120');
    await perintah('if', 'yntax');
    await perintah('echo masih-$((20 + 22))', 'masih-42');
    await perintah('golang version', 'Go belum bisa');
    await perintah('bahasa', 'Belum bisa: Go');
    await nilai("document.querySelector('#k-masuk').value = 'sleep 30'; document.querySelector('#k-kirim').click()");
    await sampai("!document.querySelector('#k-henti').hidden", 'konsol: perintah berjalan');
    await nilai("document.querySelector('#k-henti').click()");
    await sampai(siap + ' && ' + layar + ".indexOf('[dihentikan') >= 0", 'konsol: dihentikan');
    await perintah('basename "$PWD"; echo se"su"dah', 'sesudah');
    k.push(await nilai(layar + ".split('\\n').filter(function (b) { return b === 'cobajrg'; }).length"));
    hasil.konsol = k.join('|');
    hasil.tema = await nilai("document.documentElement.getAttribute('data-tema') + ' ' + getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() + ' ' + document.querySelector('#st-bhs').textContent");
    if (png) {
      const g = await kirim('Page.captureScreenshot', { format: 'png' }, sid);
      fs.writeFileSync(png, Buffer.from(g.result.data, 'base64'));
    }
    hasil.galat = galat;
    clearTimeout(batas);
    ws.close();
    akhiri(galat.length ? 1 : 0, JSON.stringify(hasil));
  };
});
