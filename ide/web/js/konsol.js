/* JRG IDE — js/konsol.js: konsol di panel bawah. Satu shell di perangkat tetap hidup di pelayan;
   tiap baris dikirim ke stdin-nya, dan penanda sesudah tiap perintah membawa kode keluar dan
   direktori kini. Rancangannya: ide/README.md, "Konsol". */
(function (IDE) {
  'use strict';
  var U = IDE.util, esc = U.esc, $ = U.$;
  var BUKA = '\x1e', SEKAT = '\x1f';
  var BATAS = 200000;                 /* huruf yang disimpan di layar */
  /* Perintah yang menjalankan bahasa-halaman kalau penjalannya tidak ada di perangkat. */
  var PERINTAH_PAKET = { python: 'py', python3: 'py', ruby: 'rb', ts: 'ts', coffee: 'coffee', scheme: 'scm', prolog: 'pl' };
  var BANTUAN =
    'Ini shell perangkat ini: ls, cd, cat, mkdir, cp, mv, rm, echo, grep dan sejenisnya jalan seperti biasa.\n' +
    'Tambahan dari IDE:\n' +
    '  bahasa            bahasa yang bisa dijalankan, dan yang bisa dipasang\n' +
    '  pasang <bahasa>   memasang bahasa (mis. pasang python); butuh internet sekali\n' +
    '  copot <bahasa>    mencopotnya lagi\n' +
    '  buka <berkas>     membuka berkas di penyunting\n' +
    '  bersih            mengosongkan layar konsol\n' +
    'Tombol kotak menghentikan perintah yang sedang jalan.\n';

  /* `o`: { api, info(), buka(nama), segarkan(), jalankanHalaman(nama, tambah, kabar) } dari ide.js. */
  IDE.konsol = function (o) {
    var K = { id: 0, sibuk: false, dir: '', sisa: '', riwayat: [], pos: 0, halaman: null, perintah: [], memulai: null, terakhir: '' };
    try { K.riwayat = JSON.parse(localStorage.getItem('jrgide.konsol') || '[]'); } catch (e) { /* kosong */ }
    K.pos = K.riwayat.length;
    var pre = $('#k-layar'), inp = $('#k-masuk');

    function cetak(html) {
      var diBawah = pre.scrollHeight - pre.scrollTop - pre.clientHeight < 60;
      pre.insertAdjacentHTML('beforeend', html);
      while (pre.textContent.length > BATAS && pre.firstChild) pre.removeChild(pre.firstChild);
      if (diBawah) pre.scrollTop = pre.scrollHeight;
    }
    function teks(t) { if (t) cetak(esc(t.replace(/\x1b\[[0-9;?]*[A-Za-z]/g, '').replace(/\r\n?/g, '\n'))); }
    function ket(t) { cetak('<span class="kt">' + esc(t) + '</span>'); }
    function galat(t) { cetak('<span class="gl">' + esc(t) + '</span>'); }
    function label() { return K.luar ? K.dir : '~' + (K.dir ? '/' + K.dir : ''); }
    function keadaan(sibuk) {
      K.sibuk = sibuk;
      $('#k-label').textContent = sibuk ? '…' : label() + ' $';
      $('#k-henti').hidden = !sibuk;
      inp.placeholder = sibuk ? 'masukan untuk program yang sedang jalan' : 'perintah, atau ketik bantuan';
    }
    /* Direktori mutlak dari shell jadi relatif akar berkas pemakai; di luarnya ditampilkan apa adanya. */
    function catatDir(mutlak) {
      var akar = K.akar;
      K.luar = !(mutlak === akar || mutlak.indexOf(akar + '/') === 0);
      K.dir = K.luar ? mutlak : mutlak.slice(akar.length + 1);
    }
    function mulai() {
      if (K.id) return Promise.resolve();
      if (K.memulai) return K.memulai;
      K.memulai = o.api('/api/konsol/mulai', { dir: K.luar ? '' : K.dir }).then(function (d) {
        K.id = d.id; K.perintah = d.perintah; K.akar = d.akar; K.sisa = ''; K.memulai = null;
        if (K.luar) { K.luar = false; K.dir = ''; }
      }, function (e) { K.memulai = null; throw e; });
      return K.memulai;
    }
    function saring(keluaran) {
      var s = K.sisa + keluaran, m;
      K.sisa = '';
      while ((m = /\x1e(\d+)\x1f([^\x1e]*)\x1e\n?/.exec(s))) {
        teks(s.slice(0, m.index));
        s = s.slice(m.index + m[0].length);
        catatDir(m[2]);
        selesaiPerintah(+m[1]);
      }
      var i = s.indexOf(BUKA);
      if (i >= 0) { K.sisa = s.slice(i); s = s.slice(0, i); }
      teks(s);
    }
    function selesaiPerintah(kode) {
      if (pre.textContent && !/\n$/.test(pre.textContent)) cetak('\n');
      if (kode === 127) petunjuk(K.terakhir.split(/\s+/)[0]);
      else if (kode) ket('[kode ' + kode + ']\n');
      keadaan(false);
      o.segarkan();
    }
    /* "sh: go: not found" tidak menolong siapa pun: sebut apakah bahasanya bisa dipasang, atau kenapa belum. */
    function petunjuk(kata) {
      var k = kata.toLowerCase(), P = IDE.paket, id;
      for (id in P.daftar) if (id === k || (' ' + P.daftar[id].alias + ' ').indexOf(' ' + k + ' ') >= 0) {
        ket(P.daftar[id].label + (P.terpasang(id) ? ' sudah dipasang: jalankan berkasnya, mis. `' + perintahUntuk(id) + ' nama.' + P.daftar[id].ekst[0] + '`.\n' : ' bisa dipasang: ketik `pasang ' + id + '`.\n'));
        return;
      }
      var belum = P.belumDi(o.info());
      for (var i = 0; i < belum.length; i++) if ((' ' + belum[i].alias + ' ').indexOf(' ' + k + ' ') >= 0) {
        ket(belum[i].label + ' belum bisa dijalankan di sini. ' + belum[i].sebab + '\n');
        return;
      }
      ket('[kode 127] perintah itu tidak ada di perangkat ini. Ketik `bantuan` atau `bahasa`.\n');
    }
    function perintahUntuk(id) { for (var p in PERINTAH_PAKET) if (IDE.paket.untuk(PERINTAH_PAKET[p]) === id) return p; return id; }
    function pantau(jeda) {
      var id = K.id;
      if (!id) return;
      o.api('/api/baca', { id: id }).then(function (h) {
        if (id !== K.id) return;
        saring((h.keluaran || '') + (h.galat || ''));
        if (h.selesai) {
          K.id = 0;
          ket(K.dihentikan ? '[dihentikan; shell dimulai lagi di ' + label() + ']\n' : '[shell selesai, kode ' + h.kode + '; perintah berikutnya memulai yang baru]\n');
          K.dihentikan = false;
          keadaan(false);
          o.segarkan();
          return;
        }
        if (!K.sibuk) return;
        var j = (h.keluaran || h.galat) ? 50 : Math.min(jeda * 1.4, 350);
        setTimeout(function () { pantau(j); }, j);
      }, function () {
        /* Pelayan lupa nomornya (dimulai ulang): shell berikutnya dibuat saat perintah berikutnya. */
        if (id === K.id) { K.id = 0; keadaan(false); }
      });
    }
    function kutip(s) { return "'" + s.replace(/'/g, "'\\''") + "'"; }
    function keShell(baris, ulang) {
      keadaan(true);
      mulai().then(function () {
        /* Sintaksnya diperiksa dulu di subshell (sebagai badan fungsi, jadi tidak ada yang jalan):
           salah sintaks di `eval` mematikan shell non-interaktif — mksh di Android juga lewat `command`. */
        var periksa = '(eval ' + kutip('jrg_periksa() { ' + baris + '\n}') + ')';
        return o.api('/api/tulis', { id: K.id, teks: 'if ' + periksa + '; then eval ' + kutip(baris) + "; else (exit 2); fi; printf '\\036%s\\037%s\\036\\n' \"$?\" \"$PWD\"\n" });
      }).then(function () { pantau(50); }, function (e) {
        /* Shell yang mati selagi menganggur baru ketahuan di sini: coba sekali lagi dengan yang baru. */
        var lama = K.id;
        K.id = 0;
        if (lama && !ulang) { keShell(baris, true); return; }
        galat(String(e.message || e) + '\n');
        keadaan(false);
      });
    }
    function jalur(nama) {
      if (K.luar || nama.charAt(0) === '/') return null;
      var bagian = (K.dir ? K.dir.split('/') : []).concat(nama.split('/')), keluar = [];
      for (var i = 0; i < bagian.length; i++) {
        if (bagian[i] === '' || bagian[i] === '.') continue;
        if (bagian[i] === '..') { if (!keluar.length) return null; keluar.pop(); } else keluar.push(bagian[i]);
      }
      return keluar.join('/');
    }
    function daftarBahasa() {
      var P = IDE.paket, baris = o.info().bahasa.map(function (b) {
        return '  ' + b.label + (b.program ? ' — ada di perangkat' : P.untuk(b.ekst[0]) ? '' : ' — ' + b.sebab);
      }).filter(function (x) { return / — /.test(x); });
      Object.keys(P.daftar).forEach(function (id) {
        baris.push('  ' + P.daftar[id].label + ' — ' + (P.terpasang(id) ? 'terpasang' : 'bisa dipasang: pasang ' + id + ' (' + P.daftar[id].ukuran + ')'));
      });
      var belum = P.belumDi(o.info());
      if (belum.length) baris.push('  Belum bisa: ' + belum.map(function (b) { return b.label; }).join(', ') + ' (ketik namanya untuk melihat sebabnya)');
      teks(baris.join('\n') + '\n');
    }
    function cariPaket(kata) {
      var P = IDE.paket, k = (kata || '').toLowerCase();
      for (var id in P.daftar) if (id === k || (' ' + P.daftar[id].alias + ' ').indexOf(' ' + k + ' ') >= 0) return id;
      return null;
    }
    function pasang(kata, copot) {
      var id = cariPaket(kata), P = IDE.paket;
      if (!kata) { ket('Tulis bahasanya, mis. `pasang python`. Ketik `bahasa` untuk daftarnya.\n'); return; }
      if (!id) { petunjuk(kata); return; }
      if (copot) { keadaan(true); P.copot(id).then(function () { ket(P.daftar[id].label + ' dicopot.\n'); }, function (e) { galat(e.message + '\n'); }).then(function () { keadaan(false); o.segarkan(); }); return; }
      if (P.terpasang(id)) { ket(P.daftar[id].label + ' sudah terpasang.\n'); return; }
      keadaan(true);
      var kini = '';
      P.pasang(id, function (t) { var a = t.replace(/ \d+%$/, ''); if (a !== kini) { kini = a; ket(a + '\n'); } }, o.info().cermin_paket).then(function () {
        ket(P.daftar[id].label + ' terpasang. Jalankan dengan `' + perintahUntuk(id) + ' nama.' + P.daftar[id].ekst[0] + '`.\n');
      }, function (e) { galat(String(e.message || e) + '\n'); }).then(function () { keadaan(false); o.segarkan(); });
    }
    function halaman(nama) {
      var j = nama ? jalur(nama) : null;
      if (!j) { ket('Tulis berkasnya (di dalam folder berkas IDE), mis. `python halo.py`.\n'); return; }
      keadaan(true);
      K.halaman = o.jalankanHalaman(j, function (h) { teks(h.keluaran || ''); if (h.galat) galat(h.galat); }, function (t) { if (t) $('#k-label').textContent = t; });
      K.halaman.janji.then(function (h) {
        if (pre.textContent && !/\n$/.test(pre.textContent)) cetak('\n');
        if (h.dihentikan) ket('[dihentikan]\n'); else if (!h.sukses) ket('[kode ' + h.kode + ']\n');
      }, function (e) { galat(String(e.message || e) + '\n'); }).then(function () { K.halaman = null; keadaan(false); o.segarkan(); });
    }
    function jalankan(baris) {
      var kata = baris.trim().split(/\s+/), k = kata[0];
      K.terakhir = baris.trim();
      if (k === 'bantuan' || k === 'help' || k === '?') { teks(BANTUAN); return; }
      if (k === 'bersih' || k === 'clear' || k === 'cls') { pre.innerHTML = ''; return; }
      if (k === 'bahasa') { daftarBahasa(); return; }
      if (k === 'pasang' || k === 'copot') { pasang(kata[1], k === 'copot'); return; }
      if (k === 'buka') {
        var j = kata[1] && jalur(kata[1]);
        if (j) o.buka(j); else ket('Tulis berkasnya, mis. `buka halo.jrg`.\n');
        return;
      }
      if (PERINTAH_PAKET[k] && K.perintah.indexOf(k) < 0 && !/[|;&<>]/.test(baris)) { halaman(kata[1]); return; }
      keShell(baris);
    }
    /* Tempelan banyak baris tetap banyak baris (dulu `<input>` membuang baris barunya): kotaknya tumbuh. */
    function tinggi() { inp.style.height = 'auto'; if (inp.value.indexOf('\n') >= 0) inp.style.height = Math.min(inp.scrollHeight, 132) + 'px'; else inp.style.height = ''; }
    function kirim() {
      var baris = inp.value.replace(/\r\n?/g, '\n');
      inp.value = '';
      tinggi();
      if (K.sibuk) {
        if (!K.id || K.halaman) return;
        cetak('<span class="kt">' + esc(baris) + '\n</span>');
        o.api('/api/tulis', { id: K.id, teks: baris + '\n' }).catch(function (e) { galat(e.message + '\n'); });
        return;
      }
      cetak('<span class="pr">' + esc(label()) + ' $</span> ' + esc(baris) + '\n');
      pre.scrollTop = pre.scrollHeight;
      if (!baris.trim()) return;
      if (K.riwayat[K.riwayat.length - 1] !== baris) K.riwayat.push(baris);
      K.riwayat = K.riwayat.slice(-100); K.pos = K.riwayat.length;
      try { localStorage.setItem('jrgide.konsol', JSON.stringify(K.riwayat)); } catch (e) { /* penuh */ }
      if (baris.indexOf('\n') >= 0) { keShell(baris); return; }
      /* Shell pertama dimulai dulu, supaya daftar perintah perangkatnya sudah diketahui. */
      mulai().then(function () { jalankan(baris); }, function (e) { galat(String(e.message || e) + '\n'); });
    }
    function mundur(arah) {
      if (!K.riwayat.length) return;
      K.pos = U.clamp(K.pos + arah, 0, K.riwayat.length);
      inp.value = K.riwayat[K.pos] || '';
      inp.setSelectionRange(inp.value.length, inp.value.length);
      tinggi();
    }
    function hentikan() {
      if (K.halaman) { K.halaman.hentikan(); return; }
      if (!K.id) return;
      K.dihentikan = true;
      o.api('/api/hentikan', { id: K.id }).then(function () { pantau(50); }, function () { K.id = 0; keadaan(false); });
    }

    inp.addEventListener('keydown', function (e) {
      K.barisBaru = e.key === 'Enter' && e.shiftKey;
      if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); kirim(); }
      else if (e.key === 'ArrowUp' && inp.value.indexOf('\n') < 0) { e.preventDefault(); mundur(-1); }
      else if (e.key === 'ArrowDown' && inp.value.indexOf('\n') < 0) { e.preventDefault(); mundur(1); }
      else if (e.key === 'c' && e.ctrlKey && K.sibuk && inp.selectionStart === inp.selectionEnd) { e.preventDefault(); hentikan(); }
    });
    /* Enter dari papan ketik HP yang tidak lewat keydown tiba sebagai sisipan baris baru. */
    inp.addEventListener('input', function (e) {
      if (e.inputType === 'insertLineBreak' && !K.barisBaru) {
        var i = inp.selectionStart;
        if (inp.value.charAt(i - 1) === '\n') inp.value = inp.value.slice(0, i - 1) + inp.value.slice(i);
        kirim(); return;
      }
      K.barisBaru = false;
      tinggi();
    });
    $('#k-kirim').onclick = function () { kirim(); inp.focus(); };
    $('#k-atas').onclick = function () { mundur(-1); inp.focus(); };
    $('#k-henti').onclick = function () { hentikan(); inp.focus(); };
    pre.addEventListener('click', function () { if (String(window.getSelection()) === '') inp.focus(); });
    keadaan(false);
    ket('Konsol JRG IDE. Ketik `bantuan` untuk perintahnya.\n');

    return {
      fokus: function () { try { inp.focus({ preventScroll: true }); } catch (e) { inp.focus(); } },
      /* Dipakai pita simbol: menulis ke baris perintah kalau itulah yang sedang diketik. */
      sisip: function (s) {
        var a = inp.selectionStart, b = inp.selectionEnd;
        inp.value = inp.value.slice(0, a) + s + inp.value.slice(b);
        inp.setSelectionRange(a + s.length, a + s.length);
        tinggi();
      },
      aktif: function () { return document.activeElement === inp; },
      perintah: function (baris) { inp.value = baris; kirim(); },
      sibuk: function () { return K.sibuk; }
    };
  };
})(window.IDE);
