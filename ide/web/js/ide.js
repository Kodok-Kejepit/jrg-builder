/* JRG IDE — js/ide.js
   Aplikasinya: berkas, tab, penyunting, jalankan, keluaran, bahasa, kamus, pengaturan.
   Halaman ini TIDAK menjalankan kode sendiri: semuanya diminta ke pelayan JRG lokal
   (`/api/...`), yang menjalankannya sebagai proses sungguhan di perangkat. */
(function (IDE) {
  'use strict';
  var U = IDE.util, esc = U.esc, $ = U.$, ikon = IDE.ikon;

  /* ---------- keadaan ---------- */
  var S = {
    info: { bahasa: [], versi: '', jrgc: '', android: false },
    berkas: [],            /* [{nama, ukuran}] */
    folder: [],            /* ['proyek', 'proyek/src'] — termasuk yang kosong */
    tab: [],               /* [{nama, isi, kotor}] */
    aktif: -1,
    jalan: false,
    masalah: { daftar: [], teks: '', nama: '' },   /* hasil pemeriksa terakhir */
    atur: bacaAtur()
  };
  var ed = null, timerSimpan = 0, konsol = null, term = null, saran = null, kode = null;

  function bacaAtur() {
    var a = { huruf: 14, bungkus: false, mode: 'dark', tema: 'mint', mesin: 'interp', terakhir: '', tutup: {}, sisi: 'berkas', argumen: {}, saran: true, periksa: true,
      latarRedup: 35, latarTembus: 25, latarKabur: 0, latarIsi: 'penuh', latarGerak: true, logo: true, rapiSimpan: false, tugas: {}, jalanDi: 'keluar', konsol: 'terminal' };
    try { var s = JSON.parse(localStorage.getItem('jrgide.atur') || '{}'); for (var k in s) a[k] = s[k]; } catch (e) { /* bawaan */ }
    return a;
  }
  function simpanAtur() {
    try { localStorage.setItem('jrgide.atur', JSON.stringify(S.atur)); } catch (e) { /* penyimpanan penuh atau dimatikan */ }
  }

  /* ---------- pelayan ---------- */
  function api(jalur, badan) {
    var o = badan === undefined ? {} : { method: 'POST', body: JSON.stringify(badan) };
    return fetch(jalur, o).then(function (r) {
      return r.text().then(function (t) {
        var d;
        try { d = JSON.parse(t); } catch (e) { throw new Error(t || ('pelayan menjawab ' + r.status)); }
        if (!r.ok) throw new Error(d.galat || ('pelayan menjawab ' + r.status));
        return d;
      });
    });
  }
  function gagal(e) { IDE.toast(String(e && e.message || e)); }

  /* ---------- bahasa ---------- */
  var PRATINJAU = { html: 1, htm: 1, svg: 1, md: 1 };
  function ekst(nama) { var m = /\.([a-z0-9]+)$/i.exec(nama || ''); return m ? m[1].toLowerCase() : ''; }
  function bahasaDari(nama) {
    var e = ekst(nama);
    for (var i = 0; i < S.info.bahasa.length; i++) if (S.info.bahasa[i].ekst.indexOf(e) >= 0) return S.info.bahasa[i];
    return null;
  }
  function labelBahasa(nama) {
    var b = bahasaDari(nama), pk = IDE.paket.untuk(ekst(nama));
    return b ? b.label : pk ? IDE.paket.daftar[pk].label : IDE.hl.label(IDE.hl.langOf(nama));
  }

  /* ---------- kerangka ---------- */
  var SIMBOL = ['    ', '{', '}', '(', ')', '[', ']', '"', '\'', '=', ':', ';', '.', ',', '<', '>', '|', '&', '!', '+', '-', '*', '/', '$', '_', '#'];
  function bangun() {
    $('#app').innerHTML =
      '<header class="atas">' +
        '<button class="tb" id="tb-menu" aria-label="Daftar berkas">' + ikon('menu') + '</button>' +
        '<button class="judul" id="j-judul" title="Buka cepat (Ctrl+P)"><img class="logo" src="' + IDE.logo.url() + '" alt=""><span class="jt"><b id="j-nama">JRG IDE</b><span id="j-ket"></span></span></button>' +
        '<button class="tb" id="tb-cari" aria-label="Kamus fungsi">' + ikon('buku') + '</button>' +
        '<button class="tb" id="tb-lain" aria-label="Lainnya">' + ikon('lain') + '</button>' +
        '<button class="tb tb-jalan" id="tb-jalan" aria-label="Jalankan" title="Jalankan">' + ikon('jalan') + '</button>' +
      '</header>' +
      '<nav class="tab" id="tab" role="tablist" aria-label="Berkas terbuka"></nav>' +
      '<div class="tengah">' +
        '<aside class="laci" id="laci" aria-label="Berkas"></aside>' +
        '<main class="sunting"><div id="ed" style="position:absolute;inset:0"></div><div class="kosong" id="kosong"></div>' +
          '<div class="petunjuk" id="petunjuk" hidden></div></main>' +
      '</div>' +
      '<div class="simbol" id="simbol" aria-label="Tombol simbol">' +
        '<button tabindex="-1" data-cmd="urung" aria-label="Urungkan">' + ikon('urung') + '</button>' +
        '<button tabindex="-1" data-cmd="ulang" aria-label="Ulangi">' + ikon('ulang') + '</button>' + SIMBOL.map(function (s, i) {
        return '<button tabindex="-1" data-i="' + i + '">' + (i === 0 ? 'Tab' : esc(s)) + '</button>';
      }).join('') + '</div>' +
      '<section class="alat" id="alat" hidden>' +
        '<div class="keluar-atas"><b id="alat-judul">Keluaran</b><span class="status" id="status"></span>' +
          '<button class="tb" id="tb-penuh" aria-label="Besarkan atau kecilkan panel">' + ikon('besar') + '</button>' +
          '<button class="tb" id="tb-tutup-keluar" aria-label="Tutup panel">' + ikon('bawah') + '</button></div>' +
        '<div class="keluar" id="keluar" hidden>' +
          '<pre id="hasil" tabindex="0"></pre>' +
          '<div class="masukan"><textarea id="masukan" placeholder="Masukan program (stdin)" aria-label="Masukan program"></textarea>' +
            '<button class="tb-teks" id="tb-kirim" hidden>Kirim</button><button class="tb-teks" id="tb-akhiri" hidden title="Tutup stdin: program yang membaca sampai habis baru selesai">Akhiri</button></div>' +
        '</div>' +
        '<div class="masalah" id="masalah" hidden><div id="m-isi"></div></div>' +
        '<div class="konsol" id="konsol" hidden>' +
          '<div class="t" id="term"></div>' +
          '<div class="k-mode" id="k-mode" hidden>' +
          '<div class="k-ganti"><span>Konsol baris</span><button class="tb-teks" id="k-ke-term">Terminal</button></div>' +
          '<pre id="k-layar" aria-label="Layar konsol" aria-live="off"></pre>' +
          '<div class="konsol-baris"><label id="k-label" for="k-masuk"></label>' +
            '<textarea id="k-masuk" rows="1" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" enterkeyhint="send" aria-label="Baris perintah"></textarea>' +
            '<button class="tb kecil" id="k-henti" aria-label="Hentikan perintah" hidden>' + ikon('henti') + '</button>' +
            '<button class="tb kecil" id="k-atas" aria-label="Perintah sebelumnya">' + ikon('atas') + '</button>' +
            '<button class="tb-teks" id="k-kirim">Kirim</button></div>' +
        '</div></div>' +
      '</section>' +
      '<footer class="bilah" id="bilah">' +
        '<button data-alat="keluar" aria-pressed="false">' + ikon('jalan') + 'Keluaran</button>' +
        '<button data-alat="konsol" aria-pressed="false">' + ikon('konsol') + 'Konsol</button>' +
        '<button data-alat="masalah" aria-pressed="false">' + ikon('masalah') + 'Masalah<span class="hitung" id="bl-masalah-n" hidden></span></button>' +
        '<button id="bl-cari">' + ikon('cari') + 'Cari</button>' +
        '<span class="st"><span class="titik" id="st-jalan" hidden></span><span id="st-pos"></span><span id="st-pos-k"></span><span id="st-bhs"></span></span>' +
      '</footer>' +
      '<div class="tirai untuk-laci" id="tirai-laci"></div><div class="tirai" id="tirai"></div>' +
      '<div class="lembar" id="lembar" role="dialog" aria-modal="true"></div>' +
      '<input type="file" id="unggah" multiple hidden><input type="file" id="unggah-folder" webkitdirectory multiple hidden>';

    ed = new IDE.Editor($('#ed'), {
      fontSize: S.atur.huruf, wrap: S.atur.bungkus, label: 'Kode',
      onChange: function (v) {
        var t = S.tab[S.aktif];
        if (!t || t.isi === v) return;
        t.isi = v;
        if (!t.kotor) { t.kotor = true; gambarTab(); }
        clearTimeout(timerSimpan);
        timerSimpan = setTimeout(function () { simpan(true); }, 1500);
      },
      onCursor: function (p) {
        $('#st-pos').textContent = 'Br ' + p.baris + ', Kol ' + p.kolom + (p.pilih ? ' (' + p.pilih + ' dipilih)' : '');
        $('#st-pos-k').textContent = p.baris + ':' + p.kolom + (p.pilih ? ' +' + p.pilih : '');
      },
      onRun: function () { jalankan(false); },
      onSave: function () { simpan(false); },
      onFind: function () { lembarCari(); },
      onTanda: function (n, pesan) { IDE.toast('Baris ' + n + ': ' + pesan); }
    });
    pasangCubit($('#ed'));
    saran = new IDE.saran.Saran(ed, {
      api: api,
      aktif: function () { return S.atur.saran && !!S.tab[S.aktif]; },
      tanda: tandaTangan
    });
    kode = IDE.kode({ api: api, ed: ed, S: S, buka: buka, lembar: lembar, tutupLembar: tutupLembar, gagal: gagal,
      simpanSemua: simpanSemua, muatBerkas: muatBerkas, segarkan: function () { gambarTab(); muatBerkas(); } });
    $('#j-judul').onclick = function () { palet(''); };
    $('#unggah').onchange = function (e) { unggahDari(e.target.files, S.unggahKe || ''); e.target.value = ''; };
    $('#unggah-folder').onchange = function (e) { unggahDari(e.target.files, S.unggahKe || ''); e.target.value = ''; };
    document.addEventListener('keydown', pintasan);

    $('#tb-menu').onclick = function () { laci(true); };
    $('#tirai-laci').onclick = function () { laci(false); };
    $('#tirai').onclick = tutupLembar;
    $('#tb-jalan').onclick = function () { if (S.jalan) hentikan(); else jalankan(false); };
    $('#tb-kirim').onclick = kirimMasukan;
    $('#tb-akhiri').onclick = function () {
      if (S.sumber && S.sumber.interaktif) S.sumber.akhiri().then(function () { $('#tb-akhiri').hidden = true; $('#tb-kirim').hidden = true; }, gagal);
    };
    $('#masukan').addEventListener('keydown', function (e) {
      if (S.jalan && e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); kirimMasukan(); }
    });
    $('#tb-lain').onclick = lembarLain;
    $('#tb-cari').onclick = lembarKamus;
    $('#tb-tutup-keluar').onclick = function () { alat(null); };
    $('#tb-penuh').onclick = function () { $('#app').classList.toggle('alat-penuh'); };
    U.$$('#bilah [data-alat]').forEach(function (b) {
      b.onclick = function () { var n = b.getAttribute('data-alat'); alat(S.alat === n ? null : n); if (S.alat === 'konsol') fokusKonsol(); };
    });
    $('#bl-cari').onclick = lembarCariSemua;
    konsol = IDE.konsol({
      api: api, info: function () { return S.info; },
      buka: function (nama) { buka(nama); }, segarkan: muatBerkas, jalankanHalaman: jalankanHalaman
    });
    term = IDE.terminal({
      api: api, akar: $('#term'), atur: S.atur, simpanAtur: simpanAtur, info: function () { return S.info; },
      buka: function (nama) { muatBerkas().then(function () { return buka(nama); }).then(function () { keSunting(true); }); }, segarkan: muatBerkas, jalankanHalaman: jalankanHalaman,
      lembar: lembar, tutupLembar: tutupLembar, keBaris: function () { modeKonsol('baris'); }, ubahPenuh: penuhTerminal,
      laci: function () { laci(true); }, menuIde: function () { $('#tb-lain').click(); },
      jalankan: function () { if (S.tab[S.aktif]) jalankan(false, '', true); else IDE.toast('Belum ada berkas yang dibuka'); }
    });
    IDE.term = term;
    $('#k-ke-term').onclick = function () { modeKonsol('terminal'); };
    $('#term').hidden = S.atur.konsol === 'baris';
    $('#k-mode').hidden = S.atur.konsol !== 'baris';
    /* Simbol ditulis saat jari DIANGKAT di tombol yang sama tanpa bergeser; menggulir pitanya
       (geser lebih dari 8 px, atau peramban mengambil alih: pointercancel) tidak menulis apa pun. */
    var tekan = null;
    $('#simbol').addEventListener('pointerdown', function (e) {
      var b = e.target.closest('button');
      tekan = b ? { b: b, x: e.clientX, y: e.clientY } : null;
      if (b) e.preventDefault();     /* fokus tetap di penyunting, papan ketik tidak menutup */
    });
    $('#simbol').addEventListener('pointermove', function (e) {
      if (tekan && (Math.abs(e.clientX - tekan.x) > 8 || Math.abs(e.clientY - tekan.y) > 8)) tekan = null;
    });
    $('#simbol').addEventListener('pointercancel', function () { tekan = null; });
    $('#simbol').addEventListener('scroll', function () { tekan = null; });
    $('#simbol').addEventListener('pointerup', function (e) {
      var b = tekan && tekan.b;
      tekan = null;
      if (!b || e.target.closest('button') !== b) return;
      e.preventDefault();
      var cmd = b.getAttribute('data-cmd');
      if (cmd) { if (S.tab[S.aktif]) ed[cmd === 'urung' ? 'undo' : 'redo'](); return; }
      if (term.aktif()) term.sisip(SIMBOL[+b.getAttribute('data-i')].trim() || ' ');
      else if (konsol.aktif()) konsol.sisip(SIMBOL[+b.getAttribute('data-i')].trim() || ' ');
      else if (S.tab[S.aktif]) ed.insert(SIMBOL[+b.getAttribute('data-i')]);
    });
    $('#hasil').addEventListener('click', function (e) {
      var a = e.target.closest('a[data-baris]');
      if (!a) return;
      e.preventDefault();
      var b = +a.getAttribute('data-baris'), k = +a.getAttribute('data-kolom') || 1, f = a.getAttribute('data-berkas');
      if (f && (!S.tab[S.aktif] || S.tab[S.aktif].nama !== f)) buka(f).then(function () { ed.goto(b, k); });
      else ed.goto(b, k);
    });
    /* Papan ketik terbuka = tinggi jendela turun jauh dari tinggi terbesar di lebar yang sama. */
    var papan = { lebar: innerWidth, tinggi: innerHeight };
    window.addEventListener('resize', function () {
      if (innerWidth !== papan.lebar) papan = { lebar: innerWidth, tinggi: innerHeight };
      papan.tinggi = Math.max(papan.tinggi, innerHeight);
      document.documentElement.classList.toggle('papan', innerHeight < papan.tinggi * 0.75);
    });
    window.addEventListener('beforeunload', function () { ingatSesi(); simpan(true); });
    document.addEventListener('visibilitychange', function () { if (document.hidden) { ingatSesi(); simpan(true); } });
  }

  /* ---------- panel bawah ---------- */
  /* Konsol punya dua mode: terminal (PTY, bawaan) dan baris (shell lewat pipa, mode lama). */
  function modeKonsol(m) {
    S.atur.konsol = m; simpanAtur();
    $('#term').hidden = m === 'baris';
    $('#k-mode').hidden = m !== 'baris';
    $('#app').classList.toggle('di-terminal', S.alat === 'konsol' && m !== 'baris');
    if (S.alat === 'konsol') fokusKonsol();
  }
  /* Satu sesi baru di panel Konsol (`linux`: Ubuntu). Sesi diminta SEBELUM panel dibuka, supaya
     pembukaan panel tidak membuat sesi bawaan lebih dulu. */
  function sesiTerminal(dir, linux) {
    var j = term.tampil(dir, linux);
    alat('konsol'); modeKonsol('terminal');
    return j.then(function (dibuat) { if (!dibuat) return term.baru(dir, linux); }).catch(gagal);
  }
  function fokusKonsol() {
    if (S.atur.konsol === 'baris') { konsol.fokus(); return; }
    term.tampil().then(function () { term.fokus(); }, function () {});
  }
  /* Di layar sempit terminal mengisi layar (bisa dimatikan di menu terminal). */
  function penuhTerminal() {
    if (S.alat === 'konsol' && S.atur.konsol !== 'baris' && S.atur.tPenuh !== false && innerWidth <= 700) $('#app').classList.add('alat-penuh');
  }
  /* `nama`: 'keluar', 'konsol', atau null untuk menutup panelnya. */
  function alat(nama) {
    S.alat = nama;
    $('#alat').hidden = !nama;
    $('#keluar').hidden = nama !== 'keluar';
    $('#konsol').hidden = nama !== 'konsol';
    $('#masalah').hidden = nama !== 'masalah';
    $('#alat-judul').textContent = ({ konsol: 'Konsol', masalah: 'Masalah' })[nama] || 'Keluaran';
    $('#status').style.visibility = nama === 'keluar' ? '' : 'hidden';
    if (nama === 'masalah') gambarMasalah();
    if (!nama || (S.penuhOtomatis && nama !== 'konsol')) { $('#app').classList.remove('alat-penuh'); S.penuhOtomatis = false; }
    $('#app').classList.toggle('di-terminal', nama === 'konsol' && S.atur.konsol !== 'baris');
    if (nama === 'konsol' && !$('#app').classList.contains('alat-penuh')) { penuhTerminal(); S.penuhOtomatis = $('#app').classList.contains('alat-penuh'); }
    U.$$('#bilah [data-alat]').forEach(function (b) { b.setAttribute('aria-pressed', String(b.getAttribute('data-alat') === nama)); });
  }

  /* ---------- laci berkas ---------- */
  /* Berkas dipilih dari laci saat panel memenuhi layar: penyuntingnya ditampilkan lagi. */
  function keSunting(tetap) {
    if (!$('#app').classList.contains('alat-penuh')) return;
    if (S.penuhOtomatis && tetap !== true) alat(null);
    else { $('#app').classList.remove('alat-penuh'); S.penuhOtomatis = false; if (term) term.sesuaikan(); }
  }
  function laci(buka) {
    $('#laci').classList.toggle('is-on', buka);
    $('#tirai-laci').classList.toggle('is-on', buka);
  }
  /* Pohon: folder bisa dilipat (ingatannya di pengaturan); folder kosong ikut tampil. Garis
     penghubung seperti `tree`: tiap tingkat leluhur yang masih punya saudara di bawahnya menurunkan
     garis tegak, dan tiap anak bercabang (├) atau berujung (└) dari garis induknya. */
  function induk(n) { var i = n.lastIndexOf('/'); return i < 0 ? '' : n.slice(0, i); }
  function ekor(n) { return n.slice(n.lastIndexOf('/') + 1); }
  function garis(lanjut, akhir) {
    return '<span class="pohon" aria-hidden="true">' + lanjut.map(function (l) { return '<i' + (l ? ' class="t"' : '') + '></i>'; }).join('') +
      (lanjut.length || akhir !== null ? '<i class="c' + (akhir ? ' u' : '') + '"></i>' : '') + '</span>';
  }
  function gambarPohon(aktif) {
    function isi(dir, lanjut) {
      var anak = S.folder.filter(function (f) { return induk(f) === dir; }).map(function (f) { return { folder: f }; })
        .concat(S.berkas.filter(function (f) { return induk(f.nama) === dir; }).map(function (f) { return { berkas: f }; }));
      return anak.map(function (a, i) {
        var akhir = i === anak.length - 1, g = dir ? garis(lanjut, akhir) : '';
        if (a.folder) {
          var f = a.folder, lipat = !!S.atur.tutup[f];
          return '<div class="baris folder">' + g + '<button class="nm-folder" data-folder="' + esc(f) + '" aria-expanded="' + !lipat + '">' +
            '<span class="panah' + (lipat ? '' : ' buka') + '">' + ikon('bawah') + '</span>' + ikon('folder') + '<b>' + esc(ekor(f)) + '</b></button>' +
            '<button class="tb kecil" data-aksi-folder="' + esc(f) + '" aria-label="Tindakan folder ' + esc(f) + '">' + ikon('lain') + '</button></div>' +
            (lipat ? '' : isi(f, dir ? lanjut.concat([!akhir]) : []));
        }
        var b = a.berkas, n = jumlahMasalah(b.nama);
        return '<div class="baris folder berkas"' + (b.nama === aktif ? ' aria-current="true"' : '') + '>' + g +
          '<button class="nm-berkas" data-buka="' + esc(b.nama) + '">' + (ekst(b.nama) === 'jrg' ? '<img class="ik ik-jrg" src="/web/aset/berkas-jrg.png" alt="">' : ikon('berkas')) +
          '<span class="nm"><b>' + esc(ekor(b.nama)) + '</b><span>' + esc(labelBahasa(b.nama)) + ' · ' + ukuran(b.ukuran) + (n ? ' · <em class="gl">' + n + ' masalah</em>' : '') + '</span></span></button>' +
          '<button class="tb kecil" data-aksi-berkas="' + esc(b.nama) + '" aria-label="Tindakan berkas ' + esc(b.nama) + '">' + ikon('lain') + '</button></div>';
      }).join('');
    }
    return isi('', []);
  }
  /* Simbol berkas aktif (fn, struct, kelas, judul Markdown…), diurai di halaman. */
  function gambarSimbol() {
    var t = S.tab[S.aktif];
    if (!t) return '<p class="catatan">Buka sebuah berkas untuk melihat simbolnya.</p>';
    var d = IDE.saran.simbol(t.isi, IDE.hl.langOf(t.nama));
    if (!d.length) return '<p class="catatan">Tidak ada fungsi, tipe, atau judul yang dikenali di ' + esc(ekor(t.nama)) + '.</p>';
    return '<input class="isian kecil" id="sb-cari" placeholder="Saring simbol" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Saring simbol">' +
      '<div id="sb-isi">' + d.map(function (x) {
        return '<button class="baris simbolnya" data-baris="' + x.baris + '" data-nama="' + esc(x.nama.toLowerCase()) + '" style="padding-left:' + (8 + Math.min(x.dalam, 6) * 14) + 'px">' +
          '<span class="jenis" data-j="' + esc(x.jenis.split(' ')[0]) + '">' + esc(x.jenis) + '</span><span class="nm"><b>' + esc(x.nama) + '</b></span><span class="no">' + x.baris + '</span></button>';
      }).join('') + '</div>';
  }
  function gambarLaci() {
    var aktif = S.tab[S.aktif] ? S.tab[S.aktif].nama : '';
    /* Tiga daftar seperti panel sisi Kate: pohon berkas, dokumen yang terbuka, dan simbol berkas aktif. */
    var sisi = S.atur.sisi === 'dokumen' || S.atur.sisi === 'simbol' ? S.atur.sisi : 'berkas';
    var dokumen = S.tab.length ? S.tab.map(function (t, i) {
      return '<div class="baris folder"' + (i === S.aktif ? ' aria-current="true"' : '') + '><button class="nm-folder" data-dok="' + i + '">' + ikon('berkas') +
        '<b>' + esc(t.nama) + (t.kotor ? ' •' : '') + '</b></button><button class="tb kecil" data-tutup-dok="' + i + '" aria-label="Tutup ' + esc(t.nama) + '">' + ikon('tutup') + '</button></div>';
    }).join('') : '<p class="catatan">Belum ada dokumen terbuka.</p>';
    var tab = function (k, label) { return '<button data-sisi="' + k + '" aria-pressed="' + (sisi === k) + '">' + label + '</button>'; };
    $('#laci').innerHTML =
      '<div class="kepala"><div class="sisi">' + tab('berkas', 'Berkas') + tab('dokumen', 'Dokumen') + tab('simbol', 'Simbol') + '</div>' +
        (sisi === 'berkas' ? '<button class="tb" id="l-unggah" aria-label="Unggah berkas dari perangkat">' + ikon('unggah') + '</button>' +
          '<button class="tb" id="l-baru" aria-label="Berkas atau proyek baru">' + ikon('tambah') + '</button>' : '') + '</div>' +
      '<div class="isi">' + (sisi === 'dokumen' ? dokumen : sisi === 'simbol' ? gambarSimbol() :
        S.berkas.length || S.folder.length ? gambarPohon(aktif) : '<p class="catatan">Belum ada berkas. Ketuk tanda tambah untuk membuat yang pertama, atau panah atas untuk mengambil berkas dari perangkat.</p>') + '</div>';
    if ($('#l-baru')) $('#l-baru').onclick = function () { lembarBaru(''); };
    if ($('#l-unggah')) $('#l-unggah').onclick = function () { lembarUnggah(''); };
    U.$$('#laci [data-sisi]').forEach(function (b) {
      b.onclick = function () { S.atur.sisi = b.getAttribute('data-sisi'); simpanAtur(); gambarLaci(); };
    });
    U.$$('#laci [data-dok]').forEach(function (b) { b.onclick = function () { pilih(+b.getAttribute('data-dok')); laci(false); keSunting(); }; });
    U.$$('#laci [data-tutup-dok]').forEach(function (b) { b.onclick = function () { tutupTab(+b.getAttribute('data-tutup-dok')); }; });
    U.$$('#laci [data-buka]').forEach(function (b) {
      b.onclick = function () { buka(b.getAttribute('data-buka')); laci(false); keSunting(); };
    });
    U.$$('#laci [data-folder]').forEach(function (b) {
      b.onclick = function () {
        var f = b.getAttribute('data-folder');
        if (S.atur.tutup[f]) delete S.atur.tutup[f]; else S.atur.tutup[f] = 1;
        simpanAtur(); gambarLaci();
      };
    });
    U.$$('#laci [data-aksi-folder]').forEach(function (b) {
      b.onclick = function () { lembarFolder(b.getAttribute('data-aksi-folder')); };
    });
    U.$$('#laci [data-aksi-berkas]').forEach(function (b) {
      b.onclick = function () { lembarBerkas(b.getAttribute('data-aksi-berkas')); };
    });
    U.$$('#laci [data-baris]').forEach(function (b) {
      b.onclick = function () { laci(false); keSunting(); ed.goto(+b.getAttribute('data-baris'), 1); };
    });
    if ($('#sb-cari')) $('#sb-cari').oninput = function () {
      var q = this.value.trim().toLowerCase();
      U.$$('#sb-isi [data-nama]').forEach(function (b) { b.hidden = !!q && b.getAttribute('data-nama').indexOf(q) < 0; });
    };
  }
  function ukuran(n) { return n < 1024 ? n + ' B' : (n / 1024).toFixed(1).replace('.', ',') + ' KB'; }
  function muatBerkas() {
    return Promise.all([api('/api/berkas'), api('/api/folder')]).then(function (d) { S.berkas = d[0]; S.folder = d[1]; gambarLaci(); });
  }

  /* ---------- tab ---------- */
  function gambarTab() {
    $('#tab').innerHTML = S.tab.map(function (t, i) {
      var nm = t.nama.slice(t.nama.lastIndexOf('/') + 1);
      return '<button role="tab" aria-selected="' + (i === S.aktif) + '" data-i="' + i + '" class="' + (t.kotor ? 'kotor' : '') + '">' +
        esc(nm) + '<span class="x" data-tutup="' + i + '" role="button" aria-label="Tutup ' + esc(nm) + '">' + ikon('tutup') + '</span></button>';
    }).join('');
    U.$$('#tab [role=tab]').forEach(function (b) {
      b.onclick = function (e) {
        var x = e.target.closest('[data-tutup]');
        if (x) tutupTab(+x.getAttribute('data-tutup')); else pilih(+b.getAttribute('data-i'));
      };
    });
    var t = S.tab[S.aktif];
    $('#j-nama').textContent = t ? t.nama : 'JRG IDE';
    var b = t && bahasaDari(t.nama);
    $('#j-ket').textContent = t ? labelBahasa(t.nama) + (b && !b.program && !IDE.paket.terpasang(IDE.paket.untuk(ekst(t.nama))) ? ' · belum bisa dijalankan di sini' : '') : (S.info.jrgc || '');
    $('#st-bhs').textContent = t ? labelBahasa(t.nama) : '';
    if (!t) { $('#st-pos').textContent = ''; $('#st-pos-k').textContent = ''; }
    $('#kosong').hidden = !!t;
    $('#ed').style.visibility = t ? 'visible' : 'hidden';
    if (!t) $('#kosong').innerHTML = '<div><img class="logo" src="' + IDE.logo.url() + '" alt="JRG"><b>Belum ada berkas terbuka</b>Buka daftar berkas, atau buat berkas baru.' +
      '<div class="deret" style="justify-content:center"><button class="tombol utama" id="k-baru">Berkas baru</button></div></div>';
    if (!t) $('#k-baru').onclick = function () { lembarBaru(''); };
  }
  /* Tab yang ditinggalkan menyimpan riwayat urung, kursor dan gulirnya (`ed.keadaan`), jadi kembali
     ke tab itu tidak memulai dari baris 1 dengan riwayat kosong. */
  function pilih(i) {
    if (ed.milik && S.tab.indexOf(ed.milik) >= 0) ed.milik.keadaan = ed.keadaan();
    S.aktif = i;
    var t = S.tab[i];
    ed.milik = t || null;
    if (t) {
      ed.setLang(IDE.hl.langOf(t.nama));
      ed.setValue(t.isi);
      ed.pulihkan(t.keadaan);
      pasangTanda(t.nama);
      S.atur.terakhir = t.nama;
    }
    ingatSesi();
    if (saran) saran.tutup();
    gambarTab(); gambarLaci();
  }
  function buka(nama) {
    for (var i = 0; i < S.tab.length; i++) if (S.tab[i].nama === nama) { pilih(i); return Promise.resolve(); }
    return api('/api/baca?nama=' + encodeURIComponent(nama)).then(function (d) {
      S.tab.push(tabBaru(d));
      pilih(S.tab.length - 1);
      periksaNanti(d.nama, 0);
    }).catch(gagal);
  }
  /* Berkas yang pernah dibuka kembali ke kursor dan gulir terakhirnya. */
  function tabBaru(d) {
    var k = (S.atur.kursor || {})[d.nama], tab = { nama: d.nama, isi: d.isi, kotor: false };
    if (k && k[0] <= d.isi.length) tab.keadaan = { riwayat: [], maju: [], sel: [k[0], k[0]], atas: k[1] || 0, kiri: 0, isi: d.isi };
    return tab;
  }
  function tutupTab(i) {
    var t = S.tab[i];
    var lanjut = function () {
      S.tab.splice(i, 1);
      pilih(S.aktif > i ? S.aktif - 1 : Math.min(S.aktif, S.tab.length - 1));
    };
    if (t && t.kotor) simpanTab(t).then(lanjut, gagal); else lanjut();
  }

  /* Tab terbuka dan kursornya diingat di perangkat; dibuka lagi saat IDE dimulai. */
  function ingatSesi() {
    if (ed && ed.milik && S.tab.indexOf(ed.milik) >= 0) ed.milik.keadaan = ed.keadaan();
    var kursor = S.atur.kursor || {};
    S.tab.forEach(function (t) {
      if (!t.keadaan) return;
      delete kursor[t.nama];
      kursor[t.nama] = [t.keadaan.sel[0], Math.round(t.keadaan.atas)];
    });
    var semua = Object.keys(kursor);
    semua.slice(0, Math.max(0, semua.length - 80)).forEach(function (n) { delete kursor[n]; });
    S.atur.kursor = kursor;
    S.atur.sesi = S.tab.map(function (t) { return t.nama; });
    S.atur.diKonsol = S.alat === 'konsol';
    simpanAtur();
  }

  /* ---------- simpan ---------- */
  function simpanTab(t) {
    var isi = t.isi;
    return api('/api/simpan', { nama: t.nama, isi: isi }).then(function () {
      if (t.isi === isi) t.kotor = false;
      periksaNanti(t.nama, 0);
    });
  }

  /* ---------- pemeriksa (galat tanpa menjalankan) ---------- */
  /* Berkas JRG diperiksa `jrgc --periksa` sesudah disimpan dan saat dibuka; di dalam proyek, berkas
     utamanya juga, karena galat di modul baru kelihatan dari yang mengimpornya. Tiap temuan ingat
     `sumber` (berkas yang diperiksa), jadi pemeriksaan ulang mengganti temuannya sendiri saja. */
  var timerPeriksa = {};
  function periksaNanti(nama, jeda) {
    if (!S.atur.periksa || ekst(nama) !== 'jrg') return;
    clearTimeout(timerPeriksa[nama]);
    timerPeriksa[nama] = setTimeout(function () {
      periksa(nama);
      if (nama.indexOf('/') > 0) api('/api/proyek?nama=' + encodeURIComponent(nama)).then(function (p) {
        if (p.utama && p.utama !== nama && ekst(p.utama) === 'jrg') periksa(p.utama);
      }).catch(function () { /* bukan proyek */ });
    }, jeda || 0);
  }
  function periksa(nama) {
    return api('/api/periksa', { nama: nama }).then(function (h) {
      S.masalah.daftar = S.masalah.daftar.filter(function (m) { return m.sumber !== nama; }).concat(h.masalah.map(function (m) {
        m.sumber = nama; return m;
      }));
      /* Gagal tanpa satu pun galat berlokasi (mis. modul tidak ketemu): teksnya ditampilkan apa adanya. */
      S.masalah.umum = S.masalah.umum || {};
      if (!h.sukses && !h.masalah.length) S.masalah.umum[nama] = h.habis_waktu ? 'pemeriksaan lewat 15 detik' : h.teks.trim(); else delete S.masalah.umum[nama];
      segarkanMasalah();
    }, function () { /* pelayan lama tanpa /api/periksa: diam bukan pilihan, tapi tidak ada yang bisa dilaporkan */ });
  }
  /* Galat menang atas peringatan di baris yang sama; `lemah` menandai baris yang cuma berperingatan. */
  function tandaUntuk(nama) {
    var t = {}, lemah = {};
    S.masalah.daftar.forEach(function (m) {
      if (m.nama !== nama) return;
      var p = m.jenis === 'peringatan';
      if (!t[m.baris] || (lemah[m.baris] && !p)) { t[m.baris] = m.pesan; if (p) lemah[m.baris] = true; else delete lemah[m.baris]; }
    });
    return [t, lemah];
  }
  function pasangTanda(nama) { var t = tandaUntuk(nama); ed.setTanda(t[0], t[1]); }
  function jumlahMasalah(nama) {
    var lihat = {};
    S.masalah.daftar.forEach(function (m) { if (m.nama === nama) lihat[m.baris + ':' + m.kolom + m.pesan] = 1; });
    return Object.keys(lihat).length + (S.masalah.umum && S.masalah.umum[nama] ? 1 : 0);
  }
  /* Satu temuan per (berkas, baris, kolom, pesan): berkas utama dan modulnya bisa melaporkan galat yang sama. */
  function semuaMasalah() {
    var lihat = {}, d = [];
    S.masalah.daftar.forEach(function (m) {
      var k = m.nama + ':' + m.baris + ':' + m.kolom + ':' + m.pesan;
      if (!lihat[k]) { lihat[k] = 1; d.push(m); }
    });
    Object.keys(S.masalah.umum || {}).forEach(function (n) { d.push({ nama: n, baris: 0, kolom: 0, pesan: S.masalah.umum[n] }); });
    return d.sort(function (a, b) { return a.nama < b.nama ? -1 : a.nama > b.nama ? 1 : a.baris - b.baris; });
  }
  function segarkanMasalah() {
    var n = semuaMasalah().length, b = $('#bl-masalah-n');
    b.hidden = !n; b.textContent = n;
    var t = S.tab[S.aktif];
    if (t) pasangTanda(t.nama);
    if (S.alat === 'masalah') gambarMasalah();
    if (S.atur.sisi === 'berkas' || !S.atur.sisi) gambarLaci();
  }
  function gambarMasalah() {
    var d = semuaMasalah();
    $('#m-isi').innerHTML = d.length ? d.map(function (m, i) {
      return '<button class="baris' + (m.jenis === 'peringatan' ? ' peringatan' : '') + '" data-m="' + i + '">' + ikon('masalah') + '<span class="nm"><b>' + esc(m.nama) + (m.baris ? ':' + m.baris + ':' + m.kolom : '') + '</b>' +
        '<span class="panjang mono-kecil">' + esc(m.pesan) + '</span></span></button>';
    }).join('') : '<p class="catatan">Tidak ada masalah di berkas JRG yang sudah diperiksa. Berkas diperiksa (tanpa dijalankan) saat dibuka dan disimpan; peringatan dari jrgm lint ikut tampil.</p>';
    U.$$('#m-isi [data-m]').forEach(function (b) {
      b.onclick = function () {
        var m = d[+b.getAttribute('data-m')];
        buka(m.nama).then(function () { if (m.baris) ed.goto(m.baris, m.kolom); });
      };
    });
  }

  /* Tanda tangan fungsi bawaan di atas penyunting sesudah namanya dipilih dari saran (`jrgc --doc`). */
  var timerPetunjuk = 0;
  function tandaTangan(nama) {
    api('/api/dok?nama=' + encodeURIComponent(nama)).then(function (d) {
      var teks = (d.teks || '').trim();
      if (!teks || /tidak (ada|dikenal|ketemu)/.test(teks.split('\n')[0])) return;
      var p = $('#petunjuk');
      p.textContent = teks.split('\n').slice(0, 2).join('\n');
      p.hidden = false;
      clearTimeout(timerPetunjuk);
      timerPetunjuk = setTimeout(function () { p.hidden = true; }, 7000);
      p.onclick = function () { p.hidden = true; };
    }).catch(function () { /* kamus tidak tersedia: tanpa petunjuk */ });
  }
  /* Semua tab yang berubah — proyek dijalankan dari berkas di perangkat, bukan dari isi tab. */
  function simpanSemua() {
    clearTimeout(timerSimpan);
    return Promise.all(S.tab.filter(function (t) { return t.kotor; }).map(simpanTab)).then(gambarTab);
  }
  function simpan(diam) {
    clearTimeout(timerSimpan);
    var t = S.tab[S.aktif];
    if (!diam && t && S.atur.rapiSimpan) return kode.rapikanDiam().then(function () { return simpanSaja(diam); });
    return simpanSaja(diam);
  }
  function simpanSaja(diam) {
    var t = S.tab[S.aktif];
    if (!t || !t.kotor) return Promise.resolve();
    return simpanTab(t).then(function () {
      gambarTab();
      if (!diam) IDE.toast('Tersimpan');
      return muatBerkas();
    }, gagal);
  }

  /* ---------- jalankan ---------- */
  function keluaran(html, status, s) {
    alat('keluar');
    $('#hasil').innerHTML = html;
    $('#status').textContent = status;
    $('#status').setAttribute('data-s', s || '');
  }
  /* Baris galat JRG (` --> berkas:12:5`) jadi tautan yang melompat ke barisnya. */
  function tandaiGalat(teks) {
    return esc(teks).replace(/(--&gt; [^\n]*?:)(\d+):(\d+)/g, function (_, depan, b, k) {
      return '<a href="#" data-baris="' + b + '" data-kolom="' + k + '">' + depan + b + ':' + k + '</a>';
    }).replace(/\(baris (\d+)\)/g, '(<a href="#" data-baris="$1">baris $1</a>)')
      /* Kompilator lain (gcc/clang, go, javac, gfortran): `berkas:baris:kolom:` di awal baris. */
      .replace(/(^|\n)([^\s:<>&]+\.(?:c|h|cc|cpp|cxx|hpp|hh|s|S|go|zig|f90|f95|f03|f|java)):(\d+):(?:(\d+):)?/g, function (_, awal, berkas, b, k) {
        var r = berkasDariGalat(berkas);
        return awal + '<a href="#" data-baris="' + b + '" data-kolom="' + (k || 1) + '"' + (r ? ' data-berkas="' + esc(r) + '"' : '') + '>' + berkas + ':' + b + (k ? ':' + k : '') + '</a>:';
      });
  }
  /* Jalur di pesan kompilator jadi nama berkas pemakai: mutlak di bawah folder data, atau akhiran yang cocok satu berkas. */
  function berkasDariGalat(j) {
    var akar = S.info.data || '';
    if (akar && j.indexOf(akar + '/') === 0) return j.slice(akar.length + 1);
    j = j.replace(/^\.\//, '');
    var cocok = S.berkas.filter(function (f) { return f.nama === j || f.nama.slice(-j.length - 1) === '/' + j; });
    return cocok.length === 1 ? cocok[0].nama : '';
  }
  /* `sendiri` true: berkas di tab ini saja. Selain itu, berkas di dalam proyek menjalankan berkas
     UTAMA proyeknya (`.utama` atau `entry` di jrgm.toml), dengan akar proyek sebagai direktori kerja. */
  function jalankan(sendiri, mode, diTerminal) {
    var t = S.tab[S.aktif];
    if (!t || S.jalan) return;
    simpanSemua().then(function () {
      return sendiri === true || t.nama.indexOf('/') < 0 ? { akar: '', utama: '' } : api('/api/proyek?nama=' + encodeURIComponent(t.nama));
    }).then(function (p) {
      if (!p.utama || p.utama === t.nama) return { nama: t.nama, isi: t.isi, akar: p.utama ? p.akar : '' };
      var terbuka = S.tab.filter(function (x) { return x.nama === p.utama; })[0];
      if (terbuka) return { nama: p.utama, isi: terbuka.isi, akar: p.akar };
      return api('/api/baca?nama=' + encodeURIComponent(p.utama)).then(function (d) { return { nama: p.utama, isi: d.isi, akar: p.akar }; });
    }).then(function (x) { x.mode = mode || ''; x.terminal = !!diTerminal; return jalankanSasaran(x); }).catch(gagal);
  }
  /* Jalankan pilihan (atau baris kursor): ditulis ke `.pilihan.<ekst>` tersembunyi di folder berkasnya,
     jadi import relatif tetap ketemu; JRG tanpa `package` menjadikan pernyataannya isi `main`. */
  function jalankanPilihan() {
    var t = S.tab[S.aktif];
    if (!t || S.jalan) return;
    var teks = ed.terpilih(), e = ekst(t.nama);
    if (!teks) { var p = ed.pos(); teks = ed.getValue().split('\n')[p.baris - 1] || ''; }
    if (!teks.trim()) { IDE.toast('Pilih kode yang mau dijalankan dulu'); return; }
    var dir = t.nama.indexOf('/') > 0 ? t.nama.slice(0, t.nama.lastIndexOf('/') + 1) : '';
    /* Baris import berkas aslinya ikut, supaya `alat::f(..)` di pilihan tetap ketemu modulnya. */
    var pola = e === 'jrg' ? /^\s*import\s+"/ : e === 'py' ? /^\s*(import|from)\s+\S/ : null;
    var impor = pola ? ed.getValue().split('\n').filter(function (l) { return pola.test(l) && teks.indexOf(l) < 0; }) : [];
    var nama = dir + '.pilihan.' + e, isi = (impor.length ? impor.join('\n') + '\n' : '') + teks.replace(/\n?$/, '\n');
    var b64 = btoa(unescape(encodeURIComponent(isi)));
    api('/api/unggah', { nama: nama, isi: b64, awal: true }).then(function () {
      return jalankanSasaran({ nama: nama, isi: isi, akar: '', mode: '', terminal: false });
    }).catch(gagal);
  }
  /* F8 / Shift+F8: galat berikutnya / sebelumnya, lintas berkas, urut berkas lalu baris. */
  function lompatMasalah(arah) {
    var d = semuaMasalah().filter(function (m) { return m.baris > 0; });
    if (!d.length) { IDE.toast('Tidak ada masalah'); return; }
    var t = S.tab[S.aktif], nama = t ? t.nama : '', baris = t ? ed.pos().baris : 0;
    var lebih = function (m) { return m.nama > nama || (m.nama === nama && m.baris > baris); };
    var kurang = function (m) { return m.nama < nama || (m.nama === nama && m.baris < baris); };
    var m = arah > 0 ? (d.filter(lebih)[0] || d[0]) : (d.filter(kurang).pop() || d[d.length - 1]);
    buka(m.nama).then(function () { ed.goto(m.baris, Math.max(1, m.kolom || 1)); IDE.toast('Baris ' + m.baris + ': ' + m.pesan); });
  }
  /* Argumen program per berkas (pengaturan), dipecah seperti shell: spasi memisah, kutip mengelompokkan. */
  function pecahArgumen(teks) {
    var hasil = [], kini = '', ada = false, kutip = '', i, c;
    for (i = 0; i < (teks || '').length; i++) {
      c = teks[i];
      if (kutip) { if (c === kutip) kutip = ''; else if (c === '\\' && kutip === '"' && i + 1 < teks.length) kini += teks[++i]; else kini += c; }
      else if (c === '"' || c === "'") { kutip = c; ada = true; }
      else if (/\s/.test(c)) { if (ada || kini) hasil.push(kini); kini = ''; ada = false; }
      else if (c === '\\' && i + 1 < teks.length) { kini += teks[++i]; ada = true; }
      else { kini += c; ada = true; }
    }
    if (ada || kini) hasil.push(kini);
    return hasil;
  }
  function jalurUrl(nama) { return nama.split('/').map(encodeURIComponent).join('/'); }
  function jalankanSasaran(t) {
    var e = ekst(t.nama);
    if (PRATINJAU[e]) {
      alat('keluar');
      $('#status').textContent = 'Pratinjau' + (t.akar ? ' · ' + t.nama : ''); $('#status').setAttribute('data-s', '');
      $('#hasil').outerHTML = '<iframe id="hasil" sandbox="allow-scripts" title="Pratinjau"></iframe>';
      /* HTML dimuat dari berkasnya, jadi CSS, skrip dan gambar di sebelahnya ikut termuat. */
      if (e === 'md') $('#hasil').srcdoc = halamanMd(t.isi); else $('#hasil').src = '/berkas/' + jalurUrl(t.nama) + '?t=' + Date.now();
      return;
    }
    if ($('#hasil').tagName !== 'PRE') $('#hasil').outerHTML = '<pre id="hasil" tabindex="0"></pre>';
    if (t.mode === 'profil' && e !== 'jrg') {
      keluaran('<span class="gl">Profil cuma untuk berkas JRG (`jrgc --profile`): waktu dan jumlah panggilan tiap fungsi. ' + esc(t.nama) + ' bukan berkas JRG.</span>', 'Tidak dijalankan', 'gagal');
      return;
    }
    if (t.mode === 'uji' && e !== 'jrg') {
      keluaran('<span class="gl">Mode uji cuma untuk berkas JRG: ia memanggil tiap fungsi `test_*`/`uji_*`. ' + esc(t.nama) + ' bukan berkas JRG.</span>', 'Tidak dijalankan', 'gagal');
      return;
    }
    if (e === 'json') {
      try { JSON.parse(t.isi); keluaran('JSON sah.', 'Diperiksa', 'ok'); }
      catch (x) { keluaran('<span class="gl">' + esc(x.message) + '</span>', 'JSON tidak sah', 'gagal'); }
      return;
    }
    var b = bahasaDari(t.nama), pk = IDE.paket.untuk(e);
    /* Penjalan di perangkat menang; paket halaman dipakai kalau penjalannya tidak ada. */
    if (b && b.program && (t.terminal || (S.atur.jalanDi === 'terminal' && !t.mode))) return jalankanDiTerminal(t);
    if (b && b.program) return jalankanDengan(t, sumberProses(t));
    if (pk && !IDE.paket.terpasang(pk)) {
      keluaran('<span class="gl">' + esc(IDE.paket.daftar[pk].label) + ' belum dipasang. Buka Lainnya, lalu Bahasa, untuk memasangnya (' + IDE.paket.daftar[pk].ukuran + ', sekali saja).</span>', 'Tidak dijalankan', 'gagal');
      return;
    }
    if (pk) return jalankanDengan(t, sumberPaket(pk, t));
    if (!b) { keluaran('<span class="gl">Berkas .' + esc(e) + ' bisa ditulis dan disorot di sini, tapi tidak ada penjalannya.</span>', 'Tidak dijalankan', 'gagal'); return; }
    keluaran('<span class="gl">' + esc(b.label + ': ' + b.sebab) + '</span>', 'Tidak dijalankan', 'gagal');
  }

  /* Markdown yang umum: judul, daftar, kutipan, blok kode, tebal/miring/kode/tautan. */
  function md(teks) {
    function dalam(x) {
      return esc(x).replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>').replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<i>$2</i>')
        .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>');
    }
    var keluar = [], baris = teks.replace(/\r/g, '').split('\n'), daftar = '', i = 0, m;
    function tutup() { if (daftar) { keluar.push('</' + daftar + '>'); daftar = ''; } }
    for (; i < baris.length; i++) {
      var l = baris[i];
      if (/^```/.test(l)) {
        tutup();
        var kode = [];
        for (i++; i < baris.length && !/^```/.test(baris[i]); i++) kode.push(baris[i]);
        keluar.push('<pre><code>' + esc(kode.join('\n')) + '</code></pre>');
      } else if ((m = /^(#{1,6})\s+(.*)$/.exec(l))) { tutup(); keluar.push('<h' + m[1].length + '>' + dalam(m[2]) + '</h' + m[1].length + '>'); }
      else if ((m = /^\s*([-*+]|\d+\.)\s+(.*)$/.exec(l))) {
        var jenis = /\d/.test(m[1]) ? 'ol' : 'ul';
        if (daftar !== jenis) { tutup(); daftar = jenis; keluar.push('<' + jenis + '>'); }
        keluar.push('<li>' + dalam(m[2]) + '</li>');
      } else if ((m = /^>\s?(.*)$/.exec(l))) { tutup(); keluar.push('<blockquote>' + dalam(m[1]) + '</blockquote>'); }
      else if (/^\s*(---+|\*\*\*+)\s*$/.test(l)) { tutup(); keluar.push('<hr>'); }
      else if (!l.trim()) tutup();
      else {
        tutup();
        var par = [l];
        while (i + 1 < baris.length && baris[i + 1].trim() && !/^(#{1,6}\s|```|>|\s*([-*+]|\d+\.)\s)/.test(baris[i + 1])) par.push(baris[++i]);
        keluar.push('<p>' + dalam(par.join(' ')) + '</p>');
      }
    }
    tutup();
    return keluar.join('\n');
  }
  function halamanMd(teks) {
    return '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><style>' +
      'body{font:16px/1.6 system-ui,sans-serif;margin:16px;color:#1c1b22;background:#fff}pre{background:#f3f2f7;padding:12px;border-radius:8px;overflow:auto}' +
      'code{font-family:ui-monospace,monospace;font-size:.92em;background:#f3f2f7;padding:1px 4px;border-radius:4px}pre code{padding:0}' +
      'blockquote{margin:0;padding-left:12px;border-left:3px solid #c9c6d8;color:#55526a}h1,h2{border-bottom:1px solid #e4e2ee;padding-bottom:4px}</style>' + md(teks);
  }

  /* Sebuah SUMBER menjalankan satu berkas: `mulai(masukan)` lalu `tunggu(tambah, kabar)` yang
     selesai dengan {sukses, kode}; `hentikan()`; `interaktif` kalau stdin-nya bisa diisi sambil jalan. */
  /* Program dapat terminal sungguhan (term::raw, warna, layar penuh): perintahnya diketik ke sesi aktif. */
  function jalankanDiTerminal(t) {
    api('/api/perintah', { nama: t.nama, akar: t.akar || '', mesin: S.atur.mesin, mode: t.mode || '', argumen: pecahArgumen(S.atur.argumen[t.nama] || '') }).then(function (d) {
      if (S.alat !== 'konsol') alat('konsol');
      if (S.atur.konsol === 'baris') modeKonsol('terminal');
      return term.tampil().then(function () { term.kirim(d.baris + '\r'); term.fokus(); });
    }).catch(gagal);
  }
  function sumberProses(t, nama, akar) {
    var id = 0;
    function pantau(tambah, jeda) {
      return api('/api/baca', { id: id }).then(function (h) {
        tambah(h);
        if (h.selesai) return h;
        var j = (h.keluaran || h.galat) ? 60 : Math.min(jeda * 1.5, 400);
        return new Promise(function (ok) { setTimeout(ok, j); }).then(function () { return pantau(tambah, j); });
      });
    }
    return {
      interaktif: true,
      mulai: function (masukan) {
        var sasaran = nama || t.nama;
        return api('/api/mulai', { nama: sasaran, akar: akar === undefined ? t.akar || '' : akar, masukan: masukan, mesin: S.atur.mesin,
          mode: t.mode || '', argumen: pecahArgumen(S.atur.argumen[t.nama] || '') }).then(function (d) { id = d.id; });
      },
      tunggu: function (tambah) { return pantau(tambah, 60); },
      hentikan: function () { if (id) api('/api/hentikan', { id: id }).catch(gagal); },
      tulis: function (teks) { return api('/api/tulis', { id: id, teks: teks }); },
      akhiri: function () { return api('/api/akhiri', { id: id }); }
    };
  }
  /* Berkas proyek (nama relatif akarnya) untuk bahasa yang jalan di halaman; di luar proyek kosong. */
  function berkasProyek(t) {
    return t.akar ? api('/api/isi_folder?dir=' + encodeURIComponent(t.akar)) : Promise.resolve([]);
  }
  function relatif(t) { return t.akar ? t.nama.slice(t.akar.length + 1) : t.nama; }
  function sumberPython(t) {
    var jalan = null, masuk = '', berkas = [];
    return {
      interaktif: false,
      mulai: function (masukan) { masuk = masukan; return berkasProyek(t).then(function (b) { berkas = b; }); },
      tunggu: function (tambah, kabar) {
        jalan = IDE.paket.python({ kode: t.isi, nama: relatif(t), masukan: masuk, berkas: berkas, cermin: S.info.cermin_paket, tambah: tambah, kabar: kabar });
        return jalan.janji;
      },
      hentikan: function () { if (jalan) jalan.hentikan(); }
    };
  }
  function sumberPekerja(id, t) {
    var jalan = null, masuk = '';
    return {
      interaktif: false,
      mulai: function (masukan) { masuk = masukan; return Promise.resolve(); },
      tunggu: function (tambah) {
        jalan = IDE.paket.pekerja(id, { kode: t.isi, nama: relatif(t), masukan: masuk, tambah: tambah });
        return jalan.janji;
      },
      hentikan: function () { if (jalan) jalan.hentikan(); }
    };
  }
  /* TypeScript/CoffeeScript dikompilasi di halaman ke `.ts-keluar/`, lalu JavaScript-nya dijalankan
     penjalan JS sebagai proses. Di dalam proyek SEMUA berkasnya ikut, supaya `import "./bantu"` jalan. */
  function sumberKompilasi(id, t) {
    var js = bahasaDari('x.js'), dalam = null, ekstIni = IDE.paket.daftar[id].ekst;
    function keJs(nama) { return nama.replace(/\.[a-z0-9]+$/i, '.js'); }
    function kompilasi(nama, isi) {
      return IDE.paket.kompilasi(id, { kode: isi, nama: nama }).then(function (h) {
        if (h.galat.length) throw new Error(h.galat.join('\n'));
        return h.js;
      });
    }
    return {
      interaktif: !!(js && js.program),
      mulai: function (masukan) {
        if (!js || !js.program) return Promise.reject(new Error(IDE.paket.daftar[id].label + ' butuh penjalan JavaScript, dan ' + (js ? js.sebab : 'itu tidak ada di sini')));
        var dasar = '.ts-keluar/' + (t.akar ? t.akar + '/' : '');
        return berkasProyek(t).then(function (semua) {
          if (!t.akar) semua = [{ nama: t.nama, isi: t.isi }];
          var rantai = api('/api/simpan', { nama: '.ts-keluar/package.json', isi: '{"type": "module"}\n' });
          semua.forEach(function (f) {
            var e = ekst(f.nama);
            if (ekstIni.indexOf(e) < 0 && e !== 'js' && e !== 'mjs' && e !== 'json') return;
            rantai = rantai.then(function () { return ekstIni.indexOf(e) >= 0 ? kompilasi(f.nama, f.isi) : f.isi; })
              .then(function (isi) { return api('/api/simpan', { nama: dasar + (ekstIni.indexOf(e) >= 0 ? keJs(f.nama) : f.nama), isi: isi }); });
          });
          return rantai;
        }).then(function () {
          dalam = sumberProses(t, dasar + keJs(relatif(t)), t.akar ? '.ts-keluar/' + t.akar : '');
          return dalam.mulai(masukan);
        });
      },
      tunggu: function (tambah) { return dalam.tunggu(tambah); },
      hentikan: function () { if (dalam) dalam.hentikan(); },
      tulis: function (teks) { return dalam.tulis(teks); },
      akhiri: function () { return dalam.akhiri(); }
    };
  }

  function sumberPaket(pk, t) {
    var cara = IDE.paket.daftar[pk].cara;
    return cara === 'python' ? sumberPython(t) : cara === 'kompilasi' ? sumberKompilasi(pk, t) : sumberPekerja(pk, t);
  }
  /* Untuk konsol (`python halo.py`): berkas bahasa-halaman dijalankan dengan aturan proyek yang sama
     dengan tombol Jalankan, tapi keluarannya ke `tambah`. Jawabannya {janji, hentikan}. */
  function jalankanHalaman(nama, tambah, kabar) {
    var sumber = null, batal = false;
    var janji = simpanSemua().then(function () {
      return Promise.all([api('/api/baca?nama=' + encodeURIComponent(nama)), nama.indexOf('/') < 0 ? { akar: '' } : api('/api/proyek?nama=' + encodeURIComponent(nama))]);
    }).then(function (d) {
      var pk = IDE.paket.untuk(ekst(nama));
      if (!pk) throw new Error('berkas .' + ekst(nama) + ' bukan bahasa yang dipasang dari IDE');
      if (!IDE.paket.terpasang(pk)) throw new Error(IDE.paket.daftar[pk].label + ' belum dipasang. Ketik: pasang ' + pk);
      if (batal) return { sukses: false, kode: -1, dihentikan: true };
      sumber = sumberPaket(pk, { nama: nama, isi: d[0].isi, akar: d[1].akar || '' });
      return sumber.mulai('').then(function () { return sumber.tunggu(tambah, kabar); });
    });
    return { janji: janji, hentikan: function () { batal = true; if (sumber) sumber.hentikan(); } };
  }

  function jalankanDengan(t, sumber) {
    S.jalan = true;
    S.sumber = sumber;
    tombolJalan(true);
    $('#st-jalan').hidden = false;
    var mulai = Date.now(), adaIsi = false, kabarKini = '';
    keluaran('', 'Berjalan…', 'jalan');
    var detak = setInterval(function () {
      $('#status').textContent = kabarKini || ('Berjalan… ' + Math.round((Date.now() - mulai) / 1000) + ' dtk');
    }, 1000);
    function kabar(teks) { kabarKini = teks; if (teks) $('#status').textContent = teks; else mulai = Date.now(); }
    var laporan = null;
    function tambah(h) {
      if (!h.keluaran && !h.galat) return;
      adaIsi = true;
      var kel = h.keluaran || '', gal = h.galat || '';
      /* Mode profil: laporan `jrgc --profile` (di stdout, sesudah program) ditahan lalu digambar utuh. */
      if (t.mode === 'profil') {
        gal = gal.replace(/^\[jrgc\] profiler aktif\n*/m, '');
        if (!$('#hasil').textContent) kel = kel.replace(/^\n+/, '');
        if (laporan !== null) { laporan += kel; kel = ''; }
        else { var i = kel.indexOf('JRG PROFILER REPORT'); if (i >= 0) { i = kel.lastIndexOf('\n', i) + 1; laporan = kel.slice(i); kel = kel.slice(0, i).replace(/\n+$/, '\n'); } }
        if (!kel && !gal) return;
      }
      var pre = $('#hasil'), diBawah = pre.scrollHeight - pre.scrollTop - pre.clientHeight < 40;
      pre.insertAdjacentHTML('beforeend', (t.mode === 'uji' ? tandaiGalat(kel) : esc(kel)) + (gal ? '<span class="gl">' + tandaiGalat(gal) + '</span>' : ''));
      if (diBawah) pre.scrollTop = pre.scrollHeight;
    }
    var masukanAwal = $('#masukan').value, awalan = t.akar ? 'Proyek ' + t.akar + ' · ' : '';
    Promise.resolve().then(function () {
      return sumber.mulai(masukanAwal ? masukanAwal.replace(/\n?$/, '\n') : '');
    }).then(function () {
      if (sumber.interaktif) {
        $('#masukan').value = '';
        $('#masukan').placeholder = 'Ketik masukan lalu Kirim (Enter)';
        $('#tb-kirim').hidden = false; $('#tb-akhiri').hidden = false;
      }
      return sumber.tunggu(tambah, kabar);
    }).then(function (h) {
      if (laporan !== null) { $('#hasil').insertAdjacentHTML('beforeend', gambarProfil(laporan)); $('#hasil').scrollTop = 0; }
      else if (!adaIsi) $('#hasil').innerHTML = '<span class="hc">(program tidak mencetak apa pun)</span>';
      var md = Date.now() - mulai;
      if (t.mode === 'uji') awalan = 'Uji · ' + awalan;
      if (t.mode === 'profil') awalan = 'Profil · ' + awalan;
      var st = (S.dihentikan || h.dihentikan) ? 'Dihentikan' : h.habis_waktu ? 'Dihentikan: lewat batas 10 menit' : (h.sukses ? 'Selesai' : 'Gagal, kode ' + h.kode);
      $('#status').textContent = awalan + st + ' · ' + (md < 10000 ? md + ' md' : Math.round(md / 1000) + ' dtk');
      $('#status').setAttribute('data-s', h.sukses ? 'ok' : 'gagal');
    }).catch(function (x) {
      $('#hasil').insertAdjacentHTML('beforeend', '<span class="gl">' + tandaiGalat(String(x.message || x)) + '</span>');
      $('#status').textContent = 'Tidak dijalankan'; $('#status').setAttribute('data-s', 'gagal');
    }).then(function () {
      clearInterval(detak);
      S.jalan = false; S.sumber = null; S.dihentikan = false;
      tombolJalan(false);
      $('#st-jalan').hidden = true;
      $('#tb-kirim').hidden = true; $('#tb-akhiri').hidden = true;
      $('#masukan').placeholder = 'Masukan program (stdin)';
      muatBerkas();
    });
  }
  /* Laporan profiler jadi kartu: tabel teks 108 kolom tidak terbaca di layar HP. Barisnya dipisah
     dua spasi atau lebih; yang tidak dikenali tetap tampil sebagai teks, jadi tidak ada yang hilang. */
  var LABEL_PROFIL = { 'Calls': 'Panggilan', 'Cumulative': 'Kumulatif', 'Self': 'Self', 'Avg/call': 'Rata-rata', 'Min': 'Min', 'Max': 'Maks' };
  function gambarProfil(teks) {
    var baris = teks.replace(/\x1b\[[0-9;]*m/g, '').split('\n'), i = 0, n = baris.length;
    function garis(b) { return /^\s*─{8,}\s*$/.test(b); }
    while (i < n && baris[i].indexOf('JRG PROFILER REPORT') < 0) i++;
    i++;
    while (i < n && !baris[i].trim()) i++;
    var ringkas = i < n ? baris[i].trim().split(' · ') : [];
    i++;
    while (i < n && !/^\s*(Fungsi|Function)\s{2,}/.test(baris[i])) i++;
    if (i >= n) return '<div class="profil"><pre>' + esc(teks) + '</pre></div>';
    var kolom = baris[i].trim().split(/\s{2,}/).filter(function (k) { return LABEL_PROFIL[k]; });
    i++; if (i < n && garis(baris[i])) i++;
    var fungsi = [], total = '';
    for (; i < n && !garis(baris[i]); i++) {
      var b = baris[i].trim(); if (!b) continue;
      var m = b.match(/\s([\d.]+)%\s*$/), pct = m ? parseFloat(m[1]) : 0;
      var sel = b.replace(/\s+[█▓▒░]+\s+[\d.]+%\s*$/, '').replace(/\s+[\d.]+%\s*$/, '').split(/\s{2,}/);
      fungsi.push({ nama: sel[0], pct: pct, angka: sel.slice(1) });
    }
    if (i < n && garis(baris[i])) i++;
    if (i < n && /^\s*TOTAL\s/.test(baris[i])) { total = baris[i].trim().split(/\s{2,}/).slice(1); i++; }
    var catatan = [], kini = '';
    for (; i < n && !/^\s*Cara baca\s*$/.test(baris[i]); i++) {
      var c = baris[i].trim();
      if (!c) { if (kini) catatan.push(kini); kini = ''; continue; }
      kini = kini && !/^⚠/.test(c) ? kini + ' ' + c : (kini && catatan.push(kini), c);
    }
    if (kini) catatan.push(kini);
    var cara = [];
    baris.slice(i + 1).forEach(function (b) {
      if (garis(b) || !b.trim()) return;
      if (/^ {2}\S/.test(b)) cara.push([b.slice(2, 13).trim(), b.slice(13).trim()]);
      else if (cara.length) cara[cara.length - 1][1] += ' ' + b.trim();
    });
    function angka(d) {
      return '<dl class="pf-angka">' + kolom.map(function (k, j) {
        return d[j] === undefined ? '' : '<div><dt>' + LABEL_PROFIL[k] + '</dt><dd>' + esc(d[j]) + '</dd></div>';
      }).join('') + '</dl>';
    }
    var maks = fungsi.reduce(function (a, f) { return Math.max(a, f.pct); }, 0) || 100;
    return '<div class="profil">' +
      '<div class="pf-kepala"><b>Profil</b>' + ringkas.map(function (r) { return '<span>' + esc(r) + '</span>'; }).join('') + '</div>' +
      fungsi.map(function (f, j) {
        var nm = f.nama.replace(/\s*↻(\d+)$/, ''), dalam = f.nama.match(/↻(\d+)$/);
        return '<div class="pf-fn' + (j === 0 && f.pct > 50 ? ' panas' : '') + '"><div class="pf-nama"><code>' + esc(nm) + '</code>' +
          (dalam ? '<span class="pf-rek" title="Rekursi terdalam">rekursi ' + dalam[1] + '</span>' : '') +
          '<span class="pf-pct">' + f.pct.toFixed(1) + '%</span></div>' +
          '<div class="pf-batang"><i style="width:' + Math.max(1, f.pct / maks * 100).toFixed(1) + '%"></i></div>' + angka(f.angka) + '</div>';
      }).join('') +
      (total ? '<div class="pf-fn pf-total"><div class="pf-nama"><b>Total</b></div>' + angka(total) + '</div>' : '') +
      catatan.map(function (c) { return '<p class="pf-catat' + (/^⚠/.test(c) ? ' awas' : '') + '">' + esc(c.replace(/^⚠\s*/, '')).replace(/^BOTTLENECK\s+/, '<b>Bottleneck</b> ') + '</p>'; }).join('') +
      (cara.length ? '<details class="pf-cara"><summary>Cara baca</summary><dl>' + cara.map(function (c) {
        return '<dt>' + esc(c[0]) + '</dt><dd>' + esc(c[1]) + '</dd>'; }).join('') + '</dl></details>' : '') +
    '</div>';
  }
  function tombolJalan(jalan) {
    var b = $('#tb-jalan');
    b.classList.toggle('is-henti', jalan);
    b.setAttribute('aria-label', jalan ? 'Hentikan' : 'Jalankan');
    b.title = jalan ? 'Hentikan' : 'Jalankan';
    b.innerHTML = ikon(jalan ? 'henti' : 'jalan');
  }
  function hentikan() {
    if (!S.sumber) return;
    S.dihentikan = true;
    S.sumber.hentikan();
  }
  function kirimMasukan() {
    if (!S.sumber || !S.sumber.interaktif) return;
    var m = $('#masukan'), teks = m.value;
    m.value = '';
    $('#hasil').insertAdjacentHTML('beforeend', '<span class="ms">' + esc(teks) + '\n</span>');
    S.sumber.tulis(teks + '\n').catch(gagal);
    m.focus();
  }

  /* ---------- lembar ---------- */
  function lembar(judul, isi) {
    $('#lembar').innerHTML = '<div class="kepala"><b>' + esc(judul) + '</b><button class="tb" id="le-tutup" aria-label="Tutup">' + ikon('tutup') + '</button></div><div class="isi">' + isi + '</div>';
    $('#lembar').classList.add('is-on');
    $('#tirai').classList.add('is-on');
    $('#le-tutup').onclick = tutupLembar;
  }
  function tutupLembar() {
    $('#lembar').classList.remove('is-on');
    $('#tirai').classList.remove('is-on');
  }
  /* Tombol kembali Android (android/Aktivitas.java bertanya ke sini dulu): menutup yang paling atas
     dan menjawab true; false kalau tidak ada yang terbuka, dan aplikasinya pindah ke latar belakang. */
  window.jrgKembali = function () {
    if (saran && !saran.el.hidden) { saran.tutup(); return true; }
    if ($('#lembar').classList.contains('is-on')) { tutupLembar(); return true; }
    if ($('#laci').classList.contains('is-on') && innerWidth < 900) { laci(false); return true; }
    if ($('#app').classList.contains('alat-penuh')) { $('#app').classList.remove('alat-penuh'); return true; }
    if (S.alat) { alat(null); return true; }
    return false;
  };

  var CONTOH = {
    jrg: 'package main\n\npub fn main() {\n    wok("halo")\n}\n',
    sh: 'echo "halo"\n',
    lua: 'print("halo")\n',
    js: 'console.log("halo");\n',
    py: 'print("halo")\n',
    sql: 'create table t(a, b);\ninsert into t values (1, \'satu\'), (2, \'dua\');\nselect * from t;\n',
    ts: 'const nama: string = "halo";\nconsole.log(nama);\n',
    coffee: 'kuadrat = (x) -> x * x\nconsole.log kuadrat 7\n',
    rb: 'puts "halo"\n',
    scm: '(display "halo")\n(newline)\n',
    pl: 'main :- write(halo), nl.\n:- initialization(main).\n',
    css: 'body {\n    font-family: sans-serif;\n}\n',
    html: '<!doctype html>\n<title>Halaman</title>\n<h1>Halo</h1>\n',
    md: '# Judul\n\nIsi.\n',
    json: '{\n    "nama": "nilai"\n}\n'
  };
  /* Proyek contoh: tiap jenis memperlihatkan berkas yang saling memakai. `utama` kosong berarti
     ditentukan berkasnya sendiri (jrgm.toml). */
  var PROYEK = {
    jrg: { label: 'JRG', utama: '', berkas: {
      'jrgm.toml': '[package]\nname    = "NAMA"\nversion = "0.1.0"\nentry   = "src/main.jrg"\n',
      'src/main.jrg': 'package main\n\nimport "hitung"\n\npub fn main() {\n    wok("3 + 4 = " + str(hitung::tambah(3, 4)))\n    wok("5! = " + str(hitung::faktorial(5)))\n}\n',
      'src/hitung.jrg': 'package hitung\n\npub fn tambah(a, b) { a + b }\n\npub fn faktorial(n) {\n    if n <= 1 { return 1 }\n    return n * faktorial(n - 1)\n}\n' } },
    py: { label: 'Python', utama: 'main.py', berkas: {
      'main.py': 'from alat.hitung import faktorial\nimport sapa\n\nprint(sapa.halo("dunia"))\nprint("5! =", faktorial(5))\n',
      'sapa.py': 'def halo(nama):\n    return f"Halo, {nama}!"\n',
      'alat/__init__.py': '',
      'alat/hitung.py': 'def faktorial(n):\n    return 1 if n <= 1 else n * faktorial(n - 1)\n' } },
    web: { label: 'Web', utama: 'index.html', berkas: {
      'index.html': '<!doctype html>\n<meta charset="utf-8">\n<meta name="viewport" content="width=device-width, initial-scale=1">\n<title>Halaman</title>\n<link rel="stylesheet" href="gaya.css">\n<h1>Halo</h1>\n<button id="tombol">Ketuk</button>\n<p id="hitung">0 kali</p>\n<script src="skrip.js"></script>\n',
      'gaya.css': 'body {\n    font-family: sans-serif;\n    margin: 24px;\n}\nbutton {\n    font-size: 18px;\n    padding: 8px 20px;\n}\n',
      'skrip.js': 'var n = 0;\ndocument.getElementById("tombol").onclick = function () {\n    n++;\n    document.getElementById("hitung").textContent = n + " kali";\n};\n' } },
    js: { label: 'JavaScript', utama: 'main.js', berkas: {
      'package.json': '{\n    "type": "module"\n}\n',
      'main.js': 'import { faktorial } from "./hitung.js";\n\nconsole.log("5! =", faktorial(5));\n',
      'hitung.js': 'export function faktorial(n) {\n    return n <= 1 ? 1 : n * faktorial(n - 1);\n}\n' } },
    lua: { label: 'Lua', utama: 'main.lua', berkas: {
      'main.lua': 'local hitung = require("hitung")\n\nprint("5! = " .. hitung.faktorial(5))\n',
      'hitung.lua': 'local M = {}\n\nfunction M.faktorial(n)\n    if n <= 1 then return 1 end\n    return n * M.faktorial(n - 1)\nend\n\nreturn M\n' } }
  };
  function buatProyek(jenis, nama) {
    var p = PROYEK[jenis], rantai = Promise.resolve(), pertama = p.utama || 'src/main.jrg';
    if (S.folder.indexOf(nama) >= 0 || S.berkas.some(function (f) { return f.nama === nama; })) { IDE.toast('`' + nama + '` sudah ada'); return; }
    Object.keys(p.berkas).forEach(function (f) {
      rantai = rantai.then(function () { return api('/api/simpan', { nama: nama + '/' + f, isi: p.berkas[f].replace('NAMA', nama) }); });
    });
    rantai.then(function () { return p.utama ? api('/api/proyek', { akar: nama, utama: p.utama }) : null; }).then(function () {
      tutupLembar();
      return muatBerkas();
    }).then(function () { return buka(nama + '/' + pertama); }).then(function () { IDE.toast('Proyek ' + nama + ' dibuat. Jalankan menjalankan ' + pertama); }).catch(gagal);
  }
  /* Rangka `jrgm new`: jrgm.toml, sumber, dan uji yang langsung lulus; `jrgm run`/`test` di terminal memakainya. */
  var RANGKA = [['jrg', 'JRG'], ['c', 'C'], ['cpp', 'C++'], ['rust', 'Rust'], ['go', 'Go'], ['java', 'Java'], ['php', 'PHP'], ['ruby', 'Ruby'], ['python', 'Python'], ['sh', 'Shell']];
  function buatProyekJrgm(bahasa, nama) {
    if (S.folder.indexOf(nama) >= 0 || S.berkas.some(function (f) { return f.nama === nama; })) { IDE.toast('`' + nama + '` sudah ada'); return; }
    api('/api/proyek/baru', { nama: nama, bahasa: bahasa }).then(function (h) {
      tutupLembar();
      return muatBerkas().then(function () { return h.utama ? buka(nama + '/' + h.utama) : null; });
    }).then(function () { IDE.toast('Proyek ' + nama + ' dibuat. Perintah proyek: jrgm run, jrgm test'); }).catch(gagal);
  }
  /* `awalan`: folder tempat berkas barunya ('' = akar). */
  function lembarBaru(awalan) {
    laci(false);
    awalan = typeof awalan === 'string' && awalan ? awalan + '/' : '';
    lembar('Baru',
      '<input class="isian" id="b-nama" placeholder="nama.jrg atau folder/nama.jrg" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Nama berkas, folder atau proyek">' +
      '<div class="deret">' + ['jrg', 'py', 'js', 'ts', 'lua', 'sh', 'sql', 'rb', 'html', 'css', 'md', 'json'].map(function (e) {
        return '<button class="tombol" data-e="' + e + '">.' + e + '</button>';
      }).join('') + '</div>' +
      '<div class="deret"><button class="tombol utama" id="b-buat">Buat berkas</button><button class="tombol" id="b-folder">Buat folder</button></div>' +
      '<div class="grup">Proyek contoh (beberapa berkas yang saling memakai)</div>' +
      '<div class="deret">' + Object.keys(PROYEK).map(function (k) { return '<button class="tombol" data-proyek="' + k + '">' + PROYEK[k].label + '</button>'; }).join('') + '</div>' +
      '<div class="grup">Proyek jrgm (dengan uji, untuk jrgm run dan jrgm test)</div>' +
      '<div class="deret">' + RANGKA.map(function (r) { return '<button class="tombol" data-rangka="' + r[0] + '">' + r[1] + '</button>'; }).join('') + '</div>' +
      '<p class="catatan">Ekstensi menentukan bahasanya. Untuk proyek, yang ditulis di atas jadi nama foldernya. Semuanya disimpan di perangkat ini.</p>');
    var inp = $('#b-nama');
    inp.value = awalan;
    U.$$('#lembar [data-e]').forEach(function (b) {
      b.onclick = function () {
        var v = inp.value.replace(/\.[a-z0-9]*$/i, '');
        if (!v || /\/$/.test(v)) v += 'baru';
        inp.value = v + '.' + b.getAttribute('data-e');
        inp.focus();
      };
    });
    $('#b-buat').onclick = function () {
      var nama = inp.value.trim();
      if (!nama || /\/$/.test(nama)) { IDE.toast('Tulis nama berkasnya dulu'); return; }
      if (S.berkas.some(function (f) { return f.nama === nama; })) { IDE.toast('Berkas itu sudah ada'); return; }
      api('/api/simpan', { nama: nama, isi: CONTOH[ekst(nama)] || '' }).then(function () {
        tutupLembar();
        return muatBerkas();
      }).then(function () { return buka(nama); }).catch(gagal);
    };
    $('#b-folder').onclick = function () {
      var nama = inp.value.trim().replace(/\/+$/, '');
      if (!nama) { IDE.toast('Tulis nama foldernya dulu'); return; }
      api('/api/folder', { nama: nama }).then(function () { tutupLembar(); return muatBerkas(); }).then(function () { laci(true); }).catch(gagal);
    };
    U.$$('#lembar [data-proyek]').forEach(function (b) {
      b.onclick = function () {
        var jenis = b.getAttribute('data-proyek'), nama = inp.value.trim().replace(/\/+$/, '').replace(/\.[a-z0-9]*$/i, '') || 'proyek-' + jenis;
        buatProyek(jenis, nama);
      };
    });
    U.$$('#lembar [data-rangka]').forEach(function (b) {
      b.onclick = function () {
        var bhs = b.getAttribute('data-rangka'), nama = inp.value.trim().replace(/\/+$/, '').replace(/\.[a-z0-9]*$/i, '') || 'proyek-' + bhs;
        buatProyekJrgm(bhs, nama);
      };
    });
    setTimeout(function () { inp.focus(); }, 250);
  }

  function lembarFolder(dir) {
    laci(false);
    var isi = S.berkas.filter(function (f) { return f.nama.indexOf(dir + '/') === 0; });
    lembar('Folder ' + dir,
      '<button class="baris" id="f-baru">' + ikon('tambah') + '<span class="nm"><b>Berkas atau folder baru di sini</b></span></button>' +
      '<button class="baris" id="f-jalan">' + ikon('jalan') + '<span class="nm"><b>Jalankan proyek ini</b><span id="f-utama">Mencari berkas utamanya…</span></span></button>' +
      '<button class="baris" id="f-term">' + ikon('konsol') + '<span class="nm"><b>Terminal di sini</b></span></button>' +
      '<button class="baris" id="f-unggah">' + ikon('unggah') + '<span class="nm"><b>Ambil berkas dari perangkat ke sini</b></span></button>' +
      '<button class="baris" id="f-ekspor">' + ikon('unduh') + '<span class="nm"><b>' + (IDE.berkas.android() ? 'Simpan folder ke penyimpanan HP' : 'Unduh folder (.zip)') + '</b><span>' + isi.length + ' berkas</span></span></button>' +
      '<button class="baris" id="f-ganti">' + ikon('ubah') + '<span class="nm"><b>Ganti nama atau pindahkan</b></span></button>' +
      '<button class="baris" id="f-hapus">' + ikon('hapus') + '<span class="nm"><b>Hapus folder</b><span>' + isi.length + ' berkas di dalamnya ikut terhapus</span></span></button>' +
      '<div id="f-lanjut"></div>');
    var utama = '';
    $('#f-baru').onclick = function () { lembarBaru(dir); };
    $('#f-unggah').onclick = function () { lembarUnggah(dir); };
    $('#f-term').onclick = function () {
      tutupLembar();
      sesiTerminal(dir);
    };
    $('#f-ekspor').onclick = function () { ekspor(dir, true); };
    /* Ditanyakan lewat sebuah berkas khayal di dalamnya: pelayan mencari proyek dari berkas ke atas. */
    api('/api/proyek?nama=' + encodeURIComponent(dir + '/x')).then(function (p) {
      utama = p.utama;
      if ($('#f-utama')) $('#f-utama').textContent = utama ? 'Berkas utama: ' + utama : 'Belum ada berkas utama';
    }).catch(gagal);
    $('#f-jalan').onclick = function () {
      if (!utama) { IDE.toast('Folder ini belum punya berkas utama'); return; }
      tutupLembar();
      buka(utama).then(function () { jalankan(false); });
    };
    $('#f-ganti').onclick = function () {
      $('#f-lanjut').innerHTML = '<input class="isian" id="f-nama" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Nama baru folder">' +
        '<div class="deret"><button class="tombol utama" id="f-ok">Ganti</button></div>';
      $('#f-nama').value = dir;
      $('#f-ok').onclick = function () {
        var ke = $('#f-nama').value.trim().replace(/\/+$/, '');
        if (!ke || ke === dir) { tutupLembar(); return; }
        simpanSemua().then(function () { return api('/api/ganti', { dari: dir, ke: ke }); }).then(function () {
          S.tab.forEach(function (t) { if (t.nama.indexOf(dir + '/') === 0) t.nama = ke + t.nama.slice(dir.length); });
          tutupLembar(); pilih(S.aktif);
          return muatBerkas();
        }).catch(gagal);
      };
    };
    $('#f-hapus').onclick = function () {
      $('#f-lanjut').innerHTML = '<p class="catatan">Folder <b>' + esc(dir) + '</b> dan seluruh isinya dihapus dari perangkat ini dan tidak bisa dikembalikan.</p>' +
        '<div class="deret"><button class="tombol bahaya" id="f-ya">Hapus</button><button class="tombol" id="f-batal">Batal</button></div>';
      $('#f-batal').onclick = tutupLembar;
      $('#f-ya').onclick = function () {
        api('/api/hapus', { nama: dir }).then(function () {
          S.tab = S.tab.filter(function (t) { return t.nama.indexOf(dir + '/') !== 0; });
          tutupLembar(); pilih(Math.min(S.aktif, S.tab.length - 1));
          return muatBerkas();
        }).catch(gagal);
      };
    };
  }

  /* ---------- berkas dari dan ke luar IDE ---------- */
  function lembarUnggah(dir) {
    laci(false);
    var A = IDE.berkas.android();
    lembar('Ambil dari perangkat' + (dir ? ' ke ' + dir : ''),
      '<button class="baris" id="u-berkas">' + ikon('unggah') + '<span class="nm"><b>Pilih berkas</b><span>' +
        (A ? 'Dari penyimpanan HP, Unduhan, Drive, dan lainnya; boleh beberapa sekaligus' : 'Boleh beberapa sekaligus') + '</span></span></button>' +
      '<button class="baris" id="u-folder">' + ikon('folder') + '<span class="nm"><b>Pilih folder</b><span>Seluruh isinya disalin, termasuk subfolder</span></span></button>' +
      '<p class="catatan" id="u-kabar"></p>' +
      '<p class="catatan">Yang dipilih DISALIN ke folder berkas IDE' + (dir ? ' (' + esc(dir) + ')' : '') + '; berkas aslinya tidak diubah. ' +
        (S.info.android ? 'Android tidak meminta izin penyimpanan untuk ini: kamu sendiri yang memilih berkasnya lewat pemilih sistem.' : '') + '</p>');
    S.unggahKe = dir || '';
    if (IDE.berkas.apkLama(S.info)) $('#u-kabar').textContent = IDE.berkas.BELUM_APK;
    $('#u-berkas').onclick = function () {
      if (IDE.berkas.apkLama(S.info)) { $('#u-kabar').textContent = IDE.berkas.BELUM_APK; return; }
      $('#unggah').click();
    };
    $('#u-folder').onclick = function () {
      if (!A) { $('#unggah-folder').click(); return; }
      IDE.berkas.imporFolder(S.info, dir).then(function (j) {
        if (j === 'ditanyakan') $('#u-kabar').textContent = 'Pilih foldernya di pemilih Android…';
        else if (j) $('#u-kabar').textContent = 'Tidak bisa: ' + j;
        else $('#u-kabar').textContent = 'Aplikasi ini dibangun sebelum impor folder ada; perbarui APK-nya.';
      });
    };
  }
  function unggahDari(daftar, dir) {
    if (!daftar || !daftar.length) return;
    var kabar = function (t) { if ($('#u-kabar')) $('#u-kabar').textContent = t; else IDE.toast(t); };
    IDE.berkas.unggah(api, daftar, dir, kabar).then(function (h) {
      kabar(h.jumlah + ' berkas diambil' + (h.gagal.length ? ', ' + h.gagal.length + ' gagal: ' + h.gagal.join('; ') : ''));
      return muatBerkas().then(function () {
        if (h.gagal.length) return;
        tutupLembar();
        IDE.toast(h.jumlah + ' berkas diambil' + (dir ? ' ke ' + dir : ''));
        if (daftar.length === 1 && !daftar[0].webkitRelativePath) buka((dir ? dir + '/' : '') + daftar[0].name);
        else laci(true);
      });
    }).catch(gagal);
  }
  /* Jawaban pemilih Android (ekspor/impor) datang sebagai peristiwa, karena dialognya milik sistem. */
  window.addEventListener('jrg-berkas', function (e) {
    var d = e.detail || {};
    if (!d.ok) { IDE.toast(d.galat === 'dibatalkan' ? 'Dibatalkan' : 'Gagal: ' + d.galat); return; }
    if (d.aksi === 'impor_folder') {
      tutupLembar();
      muatBerkas().then(function () { laci(true); });
      IDE.toast('Folder ' + d.nama + ' diambil (' + d.jumlah + ' berkas)');
    } else IDE.toast((d.aksi === 'ekspor' ? d.nama : 'Folder ' + d.nama + ', ' + d.jumlah + ' berkas,') + ' tersimpan di perangkat');
  });
  function ekspor(nama, folder) {
    simpanSemua().then(function () {
      return IDE.berkas.ekspor(api, S.info, nama, folder, S.berkas);
    }).then(function (j) {
      if (j === 'diunduh') { IDE.toast('Diunduh'); tutupLembar(); }
      else if (j === 'ditanyakan') tutupLembar();
      else IDE.toast('Tidak bisa: ' + j);
    }).catch(gagal);
  }

  /* Tindakan satu berkas dari pohonnya (bukan cuma berkas yang sedang terbuka). */
  function lembarBerkas(nama) {
    laci(false);
    var simpanKe = IDE.berkas.android() ? 'Simpan ke penyimpanan HP' : 'Unduh';
    lembar(ekor(nama),
      '<p class="catatan mono-kecil">' + esc(nama) + '</p>' +
      '<button class="baris" id="bb-buka">' + ikon('berkas') + '<span class="nm"><b>Buka</b></span></button>' +
      (bahasaDari(nama) || IDE.paket.untuk(ekst(nama)) || PRATINJAU[ekst(nama)] ? '<button class="baris" id="bb-jalan">' + ikon('jalan') + '<span class="nm"><b>Jalankan berkas ini saja</b></span></button>' : '') +
      '<button class="baris" id="bb-ganti">' + ikon('ubah') + '<span class="nm"><b>Ganti nama atau pindahkan</b></span></button>' +
      '<button class="baris" id="bb-salin">' + ikon('salin') + '<span class="nm"><b>Gandakan</b><span>Salinan di folder yang sama</span></span></button>' +
      '<button class="baris" id="bb-ekspor">' + ikon('unduh') + '<span class="nm"><b>' + simpanKe + '</b></span></button>' +
      '<button class="baris" id="bb-hapus">' + ikon('hapus') + '<span class="nm"><b>Hapus</b></span></button>' +
      '<div id="bb-lanjut"></div>');
    $('#bb-buka').onclick = function () { tutupLembar(); buka(nama); };
    if ($('#bb-jalan')) $('#bb-jalan').onclick = function () { tutupLembar(); buka(nama).then(function () { jalankan(true); }); };
    $('#bb-ekspor').onclick = function () { ekspor(nama, false); };
    $('#bb-salin').onclick = function () {
      var m = /^(.*?)(\.[^./]*)?$/.exec(nama), ke = '', i = 1;
      do { ke = m[1] + (i === 1 ? '-salinan' : '-salinan' + i) + (m[2] || ''); i++; } while (S.berkas.some(function (f) { return f.nama === ke; }));
      simpanSemua().then(function () { return fetch('/berkas/' + jalurUrl(nama)); }).then(function (r) { return r.blob(); })
        .then(function (b) { return IDE.berkas.unggah(api, [new File([b], ke.slice(ke.lastIndexOf('/') + 1))], induk(ke)); })
        .then(function (h) { if (h.gagal.length) throw new Error(h.gagal[0]); tutupLembar(); return muatBerkas(); })
        .then(function () { IDE.toast('Digandakan: ' + ke); laci(true); }).catch(gagal);
    };
    $('#bb-ganti').onclick = function () {
      $('#bb-lanjut').innerHTML = '<input class="isian" id="bb-nama" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Nama baru">' +
        '<div class="deret"><button class="tombol utama" id="bb-ok">Ganti</button></div>';
      $('#bb-nama').value = nama;
      $('#bb-ok').onclick = function () {
        var ke = $('#bb-nama').value.trim();
        if (!ke || ke === nama) { tutupLembar(); return; }
        simpanSemua().then(function () { return api('/api/ganti', { dari: nama, ke: ke }); }).then(function () {
          S.tab.forEach(function (t) { if (t.nama === nama) t.nama = ke; });
          gantiNamaMasalah(nama, ke);
          tutupLembar(); pilih(S.aktif);
          return muatBerkas();
        }).catch(gagal);
      };
      $('#bb-nama').focus();
    };
    $('#bb-hapus').onclick = function () {
      $('#bb-lanjut').innerHTML = '<p class="catatan">Berkas <b>' + esc(nama) + '</b> dihapus dari perangkat ini dan tidak bisa dikembalikan.</p>' +
        '<div class="deret"><button class="tombol bahaya" id="bb-ya">Hapus</button><button class="tombol" id="bb-batal">Batal</button></div>';
      $('#bb-batal').onclick = tutupLembar;
      $('#bb-ya').onclick = function () {
        api('/api/hapus', { nama: nama }).then(function () {
          var i = -1;
          S.tab.forEach(function (t, k) { if (t.nama === nama) { t.kotor = false; i = k; } });
          gantiNamaMasalah(nama, null);
          tutupLembar();
          if (i >= 0) tutupTab(i);
          return muatBerkas();
        }).catch(gagal);
      };
    };
  }
  /* Temuan pemeriksa ikut pindah nama, atau hilang bersama berkasnya. */
  function gantiNamaMasalah(dari, ke) {
    S.masalah.daftar = S.masalah.daftar.filter(function (m) { return ke || (m.nama !== dari && m.sumber !== dari); });
    S.masalah.daftar.forEach(function (m) { if (m.nama === dari) m.nama = ke; if (m.sumber === dari) m.sumber = ke; });
    if (S.masalah.umum && S.masalah.umum[dari] != null) { if (ke) S.masalah.umum[ke] = S.masalah.umum[dari]; delete S.masalah.umum[dari]; }
    segarkanMasalah();
  }

  function lembarArgumen() {
    var t = S.tab[S.aktif];
    if (!t) return;
    lembar('Argumen program',
      '<input class="isian mono" id="ar-isi" placeholder="mis. data.csv --cepat &quot;dua kata&quot;" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Argumen">' +
      '<p class="catatan">Untuk <b>' + esc(t.nama) + '</b> (atau berkas utama proyeknya kalau dijalankan dari sini). Spasi memisah, kutip mengelompokkan. ' +
        'Program JRG menerimanya lewat <code>env::args()</code>; bahasa lain sesudah nama berkasnya. Tersimpan di perangkat ini.</p>' +
      '<div class="deret"><button class="tombol utama" id="ar-jalan">Simpan dan jalankan</button><button class="tombol" id="ar-simpan">Simpan</button><button class="tombol" id="ar-hapus">Kosongkan</button></div>');
    var inp = $('#ar-isi');
    inp.value = S.atur.argumen[t.nama] || '';
    function simpanArg() {
      var v = inp.value.trim();
      if (v) S.atur.argumen[t.nama] = v; else delete S.atur.argumen[t.nama];
      simpanAtur();
    }
    $('#ar-simpan').onclick = function () { simpanArg(); tutupLembar(); IDE.toast('Argumen disimpan'); };
    $('#ar-jalan').onclick = function () { simpanArg(); tutupLembar(); jalankan(true); };
    $('#ar-hapus').onclick = function () { inp.value = ''; simpanArg(); tutupLembar(); };
    setTimeout(function () { inp.focus(); }, 250);
  }

  /* ---------- ukuran huruf: Ctrl+=/−/0, dan cubit dua jari di penyunting ---------- */
  function ubahHuruf(d, mutlak) {
    var n = mutlak ? d : d === 0 ? 14 : S.atur.huruf + d;
    n = Math.max(10, Math.min(28, Math.round(n)));
    if (n === S.atur.huruf) return;
    S.atur.huruf = n; ed.setFont(n);
    if (!mutlak) { simpanAtur(); IDE.toast('Huruf ' + n); }
  }
  function pasangCubit(el) {
    var awal = 0, dasar = 0, rencana = 0;
    function jarak(e) { var a = e.touches[0], b = e.touches[1]; return Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY); }
    el.addEventListener('touchstart', function (e) { if (e.touches.length === 2) { awal = jarak(e); dasar = S.atur.huruf; } }, { passive: true });
    el.addEventListener('touchmove', function (e) {
      if (e.touches.length !== 2 || !awal) return;
      e.preventDefault();
      var n = dasar * jarak(e) / awal;
      if (!rencana) rencana = requestAnimationFrame(function () { rencana = 0; ubahHuruf(n, true); });
    }, { passive: false });
    el.addEventListener('touchend', function (e) {
      if (!awal || e.touches.length) return;
      awal = 0;
      if (S.atur.huruf !== dasar) { simpanAtur(); IDE.toast('Huruf ' + S.atur.huruf); }
    }, { passive: true });
  }
  function tutupLain() {
    var t = S.tab[S.aktif];
    simpanSemua().then(function () { S.tab = t ? [t] : []; pilih(t ? 0 : -1); gambarTab(); }, gagal);
  }

  /* ---------- perintah proyek: baris shell per proyek, dijalankan di terminal dari akar proyeknya ---------- */
  function kutipShell(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
  function proyekKini() {
    var t = S.tab[S.aktif];
    if (!t) return Promise.resolve({ akar: '', utama: '', berkas: '' });
    return api('/api/proyek?nama=' + encodeURIComponent(t.nama)).then(function (p) {
      var akar = p.akar || (t.nama.indexOf('/') > 0 ? t.nama.slice(0, t.nama.lastIndexOf('/')) : '');
      return { akar: akar, utama: p.utama ? p.utama.slice(akar.length + 1) : '', berkas: akar ? t.nama.slice(akar.length + 1) : t.nama };
    }, function () { return { akar: '', utama: '', berkas: t.nama }; });
  }
  /* $HOME shell terminal adalah folder berkas IDE (src/terminal.jrg), jadi jalurnya cukup ~/<akar>. */
  function jalankanTugas(akar, baris) {
    if (S.alat !== 'konsol') alat('konsol');
    if (S.atur.konsol === 'baris') modeKonsol('terminal');
    return term.tampil().then(function () { term.kirim('cd ~' + (akar ? '/' + kutipShell(akar) : '') + ' && ' + baris + '\r'); term.fokus(); });
  }
  function lembarTugas() {
    proyekKini().then(function (p) {
      var daftar = (S.atur.tugas[p.akar] || []).slice();
      var utama = p.utama || p.berkas, saran = [];
      if (/\.jrg$/.test(utama)) saran = ['jrgc ' + utama, 'jrgc ' + utama + ' --vm', 'jrgc ' + utama + ' --uji', 'jrgc ' + utama + ' --periksa', 'jrgc ' + utama + ' --profile'];
      /* jrgm ada juga di terminal HP (jrgc --jrgm); cuma `jrgm apk` yang butuh perkakas Android laptop atau Termux. */
      if (S.berkas.some(function (f) { return f.nama === (p.akar ? p.akar + '/' : '') + 'jrgm.toml'; })) saran = saran.concat(['jrgm run', 'jrgm test', 'jrgm lint', 'jrgm fmt', 'jrgm build'].concat(S.info.android ? [] : ['jrgm apk']));
      saran = saran.filter(function (c) { return !daftar.some(function (d) { return d.c === c; }); });
      function simpanDaftar() { S.atur.tugas[p.akar] = daftar; if (!daftar.length) delete S.atur.tugas[p.akar]; simpanAtur(); }
      lembar('Perintah proyek',
        '<p class="catatan">Dijalankan di terminal dari <span class="mono">' + esc(p.akar || 'folder berkas') + '</span>.</p>' +
        (daftar.length ? daftar.map(function (d, i) {
          return '<div class="baris tg"><button class="tg-jalan" data-tg="' + i + '">' + ikon('jalan') + '<span class="nm"><b>' + esc(d.n || d.c) + '</b>' + (d.n ? '<span class="mono">' + esc(d.c) + '</span>' : '') + '</span></button>' +
            '<button class="tb" data-tg-hapus="' + i + '" aria-label="Hapus ' + esc(d.n || d.c) + '">' + ikon('hapus') + '</button></div>';
        }).join('') : '<p class="catatan">Belum ada perintah untuk proyek ini.</p>') +
        '<p class="subjudul">Tambah</p>' +
        '<input class="isian kecil" id="tg-nama" placeholder="Nama (boleh kosong), mis. Bangun" autocapitalize="off" aria-label="Nama perintah">' +
        '<input class="isian kecil mono" id="tg-baris" placeholder="Perintah shell, mis. jrgc main.jrg --vm" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Perintah shell">' +
        '<div class="deret"><button class="tombol utama" id="tg-tambah">Tambah</button><button class="tombol" id="tg-coba">Jalankan sekali</button></div>' +
        (saran.length ? '<p class="subjudul">Saran</p><div class="deret">' + saran.map(function (c, i) { return '<button class="tombol kecil mono" data-tg-saran="' + i + '">' + esc(c) + '</button>'; }).join('') + '</div>' : ''));
      U.$$('#lembar [data-tg]').forEach(function (b) { b.onclick = function () { tutupLembar(); jalankanTugas(p.akar, daftar[+b.getAttribute('data-tg')].c); }; });
      U.$$('#lembar [data-tg-hapus]').forEach(function (b) { b.onclick = function () { daftar.splice(+b.getAttribute('data-tg-hapus'), 1); simpanDaftar(); lembarTugas(); }; });
      U.$$('#lembar [data-tg-saran]').forEach(function (b) { b.onclick = function () { daftar.push({ n: '', c: saran[+b.getAttribute('data-tg-saran')] }); simpanDaftar(); lembarTugas(); }; });
      function isian() { return { n: $('#tg-nama').value.trim(), c: $('#tg-baris').value.trim() }; }
      $('#tg-tambah').onclick = function () { var d = isian(); if (!d.c) { $('#tg-baris').focus(); return; } daftar.push(d); simpanDaftar(); lembarTugas(); };
      $('#tg-coba').onclick = function () { var d = isian(); if (!d.c) { $('#tg-baris').focus(); return; } tutupLembar(); jalankanTugas(p.akar, d.c); };
    });
  }

  /* ---------- palet: buka cepat, perintah (`>`), ke baris (`:`) ---------- */
  /* Folder proyek jrgm terdekat di atas berkas (dari daftar berkas, tanpa bertanya ke pelayan); null kalau bukan. */
  function akarJrgm(nama) {
    var d = nama.split('/').slice(0, -1);
    for (; ; d.pop()) {
      var a = d.join('/');
      if (S.berkas.some(function (f) { return f.nama === (a ? a + '/' : '') + 'jrgm.toml'; })) return a;
      if (!d.length) return null;
    }
  }
  function perintahSemua() {
    var t = S.tab[S.aktif], jrg = t && ekst(t.nama) === 'jrg', pj = t ? akarJrgm(t.nama) : null;
    return [
      ['Proyek: uji semua (jrgm test)', '', function () { jalankanTugas(pj, 'jrgm test'); }, pj !== null],
      ['Proyek: periksa dan peringatan (jrgm lint)', '', function () { jalankanTugas(pj, 'jrgm lint'); }, pj !== null],
      ['Proyek: bangun (jrgm build)', '', function () { jalankanTugas(pj, 'jrgm build'); }, pj !== null],
      ['Jalankan', 'Ctrl+Enter', function () { jalankan(false); }, !!t],
      ['Jalankan di terminal', 'Ctrl+Shift+Enter', function () { jalankan(false, '', true); }, !!t],
      ['Jalankan berkas ini saja', '', function () { jalankan(true); }, !!t],
      ['Jalankan pilihan (atau baris ini)', 'Ctrl+Alt+Enter', jalankanPilihan, !!t],
      ['Masalah berikutnya', 'F8', function () { lompatMasalah(1); }, true],
      ['Masalah sebelumnya', 'Shift+F8', function () { lompatMasalah(-1); }, true],
      ['Bandingkan dengan berkas lain…', '', kode.bandingkan, !!t],
      ['Jalankan uji (test_* / uji_*)', '', function () { jalankan(true, 'uji'); }, jrg],
      ['Jalankan dengan profil (waktu tiap fungsi)', '', function () { jalankan(false, 'profil'); }, jrg && S.atur.mesin !== 'llvm'],
      ['Ke definisi', 'F12', kode.definisi, !!t],
      ['Cari rujukan', 'Shift+F12', kode.rujukan, !!t],
      ['Ganti nama simbol di semua berkas', 'F2', kode.gantiNama, !!t],
      ['Dokumentasi nama di kursor', 'F1', kode.dokumentasi, !!t],
      ['Rapikan berkas', 'Shift+Alt+F', kode.rapikan, !!t],
      ['Riwayat berkas ini', '', kode.riwayat, !!t],
      ['Berkas terhapus', '', kode.terhapus, true],
      ['Daftar TODO', '', kode.todo, true],
      ['Pindahkan baris ke atas', 'Alt+Atas', function () { ed.pindahBaris(-1); }, !!t],
      ['Pindahkan baris ke bawah', 'Alt+Bawah', function () { ed.pindahBaris(1); }, !!t],
      ['Hapus baris', 'Ctrl+Shift+K', function () { ed.hapusBaris(); }, !!t],
      ['Argumen program…', '', lembarArgumen, !!t],
      ['Periksa galat sekarang', '', function () { periksaNanti(t.nama, 0); alat('masalah'); }, jrg],
      ['Simpan', 'Ctrl+S', function () { simpan(false); }, !!t],
      ['Simpan semua', '', function () { simpanSemua().then(function () { IDE.toast('Semua tersimpan'); }, gagal); }, S.tab.length > 0],
      ['Cari dan ganti', 'Ctrl+F', lembarCari, !!t],
      ['Cari di semua berkas', 'Ctrl+Shift+F', lembarCariSemua, true],
      ['Ke baris…', 'Ctrl+G', function () { palet(':'); }, !!t],
      ['Ke simbol di berkas ini…', 'Ctrl+Shift+O', function () { palet('@'); }, !!t],
      ['Ke simbol di semua berkas…', 'Ctrl+T', function () { palet('#'); }, true],
      ['Perintah proyek…', '', lembarTugas, true],
      ['Perbesar huruf', 'Ctrl+=', function () { ubahHuruf(1); }, true],
      ['Perkecil huruf', 'Ctrl+-', function () { ubahHuruf(-1); }, true],
      ['Ukuran huruf semula', 'Ctrl+0', function () { ubahHuruf(0); }, true],
      ['Tutup tab lain', '', function () { tutupLain(); }, S.tab.length > 1],
      ['Komentar/hapus komentar baris', 'Ctrl+/', function () { ed.toggleKomentar(); }, !!t],
      ['Gandakan baris', 'Ctrl+D', function () { ed.focus(); ed.ta.dispatchEvent(new KeyboardEvent('keydown', { key: 'd', ctrlKey: true, bubbles: true })); }, !!t],
      ['Urungkan', 'Ctrl+Z', function () { ed.undo(); }, !!t],
      ['Ulangi', 'Ctrl+Y', function () { ed.redo(); }, !!t],
      ['Tutup tab', '', function () { tutupTab(S.aktif); }, !!t],
      ['Tutup semua tab', '', function () { simpanSemua().then(function () { S.tab = []; pilih(-1); }, gagal); }, S.tab.length > 0],
      ['Berkas atau proyek baru', '', function () { lembarBaru(''); }, true],
      ['Ambil berkas dari perangkat', '', function () { lembarUnggah(''); }, true],
      [IDE.berkas.android() ? 'Simpan berkas ini ke penyimpanan HP' : 'Unduh berkas ini', '', function () { ekspor(t.nama, false); }, !!t],
      ['Tindakan berkas ini', '', function () { lembarBerkas(t.nama); }, !!t],
      ['Panel Keluaran', '', function () { alat('keluar'); }, true],
      ['Panel Konsol (terminal)', 'Ctrl+`', function () { alat('konsol'); fokusKonsol(); }, true],
      ['Terminal: sesi baru', '', function () { sesiTerminal(); }, true],
      ['Panel Masalah', '', function () { alat('masalah'); }, true],
      ['Simbol berkas ini', '', function () { S.atur.sisi = 'simbol'; simpanAtur(); gambarLaci(); laci(true); }, !!t],
      ['Kamus fungsi JRG', '', lembarKamus, true],
      ['Bahasa', '', function () { lembarBahasa(); }, true],
      ['Linux (Ubuntu)', '', function () { lembarLinux(); }, !!(S.info.linux && S.info.linux.bisa)],
      ['Terminal: sesi Ubuntu', '', function () { sesiTerminal(undefined, true); }, !!(S.info.linux && S.info.linux.terpasang)],
      ['Perangkat', '', function () { lembarPerangkat(); }, true],
      ['Pengaturan', '', lembarAtur, true],
      ['Ganti tampilan siang/malam', '', function () { S.atur.mode = S.atur.mode === 'dark' ? 'light' : 'dark'; simpanAtur(); terapkanAtur(); }, true]
    ].filter(function (p) { return p[3]; });
  }
  function palet(awal) {
    lembar('Buka cepat',
      '<input class="isian" id="pl-isi" placeholder="berkas · > perintah · : baris · @ simbol · # semua simbol" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Cari berkas atau perintah">' +
      '<div id="pl-hasil"></div>');
    var inp = $('#pl-isi'), pilihan = 0, daftar = [], simbolSemua = null;
    inp.value = awal || '';
    /* `#`: simbol di semua berkas teks (isi tab yang terbuka lebih baru dari yang tersimpan). */
    function muatSimbolSemua() {
      simbolSemua = 'memuat';
      api('/api/teks_semua').then(function (d) {
        var tab = {};
        S.tab.forEach(function (t) { tab[t.nama] = t.isi; });
        simbolSemua = [];
        d.forEach(function (f) {
          var lang = IDE.hl.langOf(f.nama);
          if (!lang || lang === 'text') return;
          IDE.saran.simbol(tab[f.nama] !== undefined ? tab[f.nama] : f.isi, lang).forEach(function (x) {
            simbolSemua.push({ nama: x.nama, jenis: x.jenis, baris: x.baris, berkas: f.nama });
          });
        });
        if ($('#pl-isi') === inp) gambar();
      }).catch(function (e) { simbolSemua = null; $('#pl-hasil').innerHTML = '<p class="catatan">Tidak bisa: ' + esc(e.message || e) + '</p>'; });
    }
    function cocok(teks, q) {
      teks = teks.toLowerCase();
      if (!q) return 1;
      var i = teks.indexOf(q);
      if (i >= 0) return 1000 - i - teks.length / 100;
      var j = 0;
      for (var k = 0; k < teks.length && j < q.length; k++) if (teks[k] === q[j]) j++;
      return j === q.length ? 100 - teks.length / 100 : 0;
    }
    function gambar() {
      var v = inp.value, q = v.replace(/^[>:@#]\s*/, '').toLowerCase();
      if (v[0] === '#') {
        if (!simbolSemua) muatSimbolSemua();
        if (simbolSemua === 'memuat') { daftar = []; $('#pl-hasil').innerHTML = '<p class="catatan">Membaca simbol semua berkas…</p>'; return; }
        daftar = simbolSemua.map(function (x) {
          return { label: x.nama, ket: x.jenis + ' · ' + x.berkas + ':' + x.baris, n: cocok(x.nama, q), jalan: function () { buka(x.berkas).then(function () { ed.goto(x.baris, 1); }); } };
        }).filter(function (p) { return p.n > 0; }).sort(function (a, b) { return q ? b.n - a.n : 0; }).slice(0, 300);
      } else if (v[0] === '@') {
        var ts = S.tab[S.aktif];
        daftar = ts ? IDE.saran.simbol(ts.isi, IDE.hl.langOf(ts.nama)).map(function (x) {
          return { label: x.nama, ket: x.jenis + ' · baris ' + x.baris, n: cocok(x.nama, q), jalan: function () { ed.goto(x.baris, 1); } };
        }).filter(function (p) { return p.n > 0; }).sort(function (a, b) { return q ? b.n - a.n : 0; }).slice(0, 200) : [];
      } else if (v[0] === ':') {
        var n = parseInt(q, 10), t = S.tab[S.aktif];
        daftar = t && n > 0 ? [{ label: 'Ke baris ' + n + (ed.nLines ? ' dari ' + ed.nLines : ''), jalan: function () { ed.goto(n, 1); } }] : [];
        $('#pl-hasil').innerHTML = daftar.length ? '' : '<p class="catatan">' + (t ? 'Tulis nomor baris (1 sampai ' + ed.nLines + ').' : 'Belum ada berkas terbuka.') + '</p>';
      } else if (v[0] === '>') {
        daftar = perintahSemua().map(function (p) { return { label: p[0], ket: p[1], jalan: p[2], n: cocok(p[0], q) }; })
          .filter(function (p) { return p.n > 0; }).sort(function (a, b) { return b.n - a.n; });
      } else {
        var terbuka = {};
        S.tab.forEach(function (t) { terbuka[t.nama] = 1; });
        daftar = S.berkas.map(function (f) { return { label: f.nama, ket: terbuka[f.nama] ? 'terbuka' : labelBahasa(f.nama), nama: f.nama, n: cocok(f.nama, q) * (terbuka[f.nama] ? 1.001 : 1) }; })
          .filter(function (p) { return p.n > 0; }).sort(function (a, b) { return b.n - a.n; }).slice(0, 60);
        daftar.forEach(function (d) { d.jalan = function () { buka(d.nama); }; });
      }
      pilihan = Math.min(pilihan, Math.max(0, daftar.length - 1));
      if (daftar.length || v[0] !== ':') $('#pl-hasil').innerHTML = daftar.length ? daftar.map(function (d, i) {
        return '<button class="baris" data-pl="' + i + '"' + (i === pilihan ? ' aria-current="true"' : '') + '>' +
          ikon(v[0] === '>' ? 'perintah' : v[0] === ':' ? 'bawah' : v[0] === '@' || v[0] === '#' ? 'simbol' : 'berkas') + '<span class="nm"><b>' + esc(d.label) + '</b>' + (d.ket ? '<span>' + esc(d.ket) + '</span>' : '') + '</span></button>';
      }).join('') : '<p class="catatan">Tidak ada yang cocok.' + (/^[>@#]/.test(v) ? '' : ' Awali dengan &gt; untuk perintah, : untuk nomor baris, @ untuk simbol berkas ini, # untuk simbol semua berkas.') + '</p>';
      U.$$('#pl-hasil [data-pl]').forEach(function (b) { b.onclick = function () { pakai(+b.getAttribute('data-pl')); }; });
    }
    function pakai(i) { var d = daftar[i]; if (!d) return; tutupLembar(); d.jalan(); }
    inp.oninput = function () { pilihan = 0; gambar(); };
    inp.onkeydown = function (e) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        pilihan = (pilihan + (e.key === 'ArrowDown' ? 1 : -1) + daftar.length) % Math.max(1, daftar.length);
        gambar();
        var b = $('#pl-hasil [aria-current]'); if (b) b.scrollIntoView({ block: 'nearest' });
      } else if (e.key === 'Enter') { e.preventDefault(); pakai(pilihan); }
      else if (e.key === 'Escape') { e.preventDefault(); tutupLembar(); }
    };
    gambar();
    setTimeout(function () { inp.focus(); }, 200);
  }
  /* Pintasan seperti di IDE desktop; Ctrl+S, Ctrl+Enter, Ctrl+F dll. di dalam penyunting ada di penyunting.js. */
  function pintasan(e) {
    var mod = e.ctrlKey || e.metaKey, k = (e.key || '').toLowerCase();
    /* Selagi mengetik di terminal, tombol milik programnya (F5, Ctrl+P, Ctrl+B ...); cuma Ctrl+` yang tetap.
       Fokus yang tertinggal di terminal sesudah panelnya diganti tidak dihitung. */
    if (term && term.aktif() && S.alat === 'konsol') {
      if (mod && k === '`') { e.preventDefault(); alat(null); }
      return;
    }
    if (!mod) {
      var t = S.tab[S.aktif], f = !t ? null : e.key === 'F12' ? (e.shiftKey ? kode.rujukan : kode.definisi) : e.key === 'F2' ? kode.gantiNama :
        e.key === 'F1' ? kode.dokumentasi : e.shiftKey && e.altKey && k === 'f' ? kode.rapikan : null;
      if (e.key === 'F5') { e.preventDefault(); if (S.jalan) hentikan(); else jalankan(false); }
      else if (e.key === 'F8') { e.preventDefault(); lompatMasalah(e.shiftKey ? -1 : 1); }
      else if (f) { e.preventDefault(); f(); }
      return;
    }
    if (k === 'p' && e.shiftKey) { e.preventDefault(); palet('>'); }
    else if (k === 'p') { e.preventDefault(); palet(''); }
    else if (k === 'g' && S.tab[S.aktif]) { e.preventDefault(); palet(':'); }
    else if (k === 'o' && e.shiftKey && S.tab[S.aktif]) { e.preventDefault(); palet('@'); }
    else if (k === 't' && !e.shiftKey) { e.preventDefault(); palet('#'); }
    else if ((k === '=' || k === '+') && !e.altKey) { e.preventDefault(); ubahHuruf(1); }
    else if (k === '-' && !e.altKey) { e.preventDefault(); ubahHuruf(-1); }
    else if (k === '0' && !e.altKey) { e.preventDefault(); ubahHuruf(0); }
    else if (k === 'f' && e.shiftKey) { e.preventDefault(); lembarCariSemua(); }
    else if (k === 'b') { e.preventDefault(); laci(!$('#laci').classList.contains('is-on')); }
    else if (k === 'j') { e.preventDefault(); alat(S.alat ? null : 'keluar'); }
    else if (k === '`') { e.preventDefault(); if (S.alat === 'konsol') alat(null); else { alat('konsol'); fokusKonsol(); } }
    else if (k === 'enter' && e.shiftKey && S.tab[S.aktif]) { e.preventDefault(); jalankan(false, '', true); }
    else if (k === 'enter' && e.altKey && S.tab[S.aktif]) { e.preventDefault(); jalankanPilihan(); }
  }

  function lembarLain() {
    var t = S.tab[S.aktif];
    lembar('Lainnya',
      (t ? '<div class="grup">Kode' + (kode.kata() ? ' · <span class="mono">' + esc(kode.kata()) + '</span>' : '') + '</div>' +
        '<div class="kisi">' +
          '<button class="tombol" id="o-def">Ke definisi</button><button class="tombol" id="o-ruj">Rujukan</button>' +
          '<button class="tombol" id="o-gn">Ganti nama</button><button class="tombol" id="o-dok">Dokumentasi</button>' +
          '<button class="tombol" id="o-rapi">Rapikan</button><button class="tombol" id="o-riwayat">Riwayat</button>' +
          '<button class="tombol" id="o-naik">Baris naik</button><button class="tombol" id="o-turun">Baris turun</button>' +
          '<button class="tombol" id="o-hapus-baris">Hapus baris</button>' + (ekst(t.nama) === 'jrg' ? '<button class="tombol" id="o-profil">Profil</button>' : '') +
          '<button class="tombol" id="o-di-term">Jalankan di terminal</button>' +
          '<button class="tombol" id="o-pilihan">Jalankan pilihan</button><button class="tombol" id="o-banding">Bandingkan</button>' +
        '</div>' : '') +
      (t ? '<div class="grup">' + esc(t.nama) + '</div>' +
        '<button class="baris" id="o-simpan">' + ikon('simpan') + '<span class="nm"><b>Simpan</b></span></button>' +
        '<button class="baris" id="o-ganti">' + ikon('ubah') + '<span class="nm"><b>Ganti nama atau pindahkan</b></span></button>' +
        '<button class="baris" id="o-hapus">' + ikon('hapus') + '<span class="nm"><b>Hapus berkas ini</b></span></button>' : '') +
      (t && t.nama.indexOf('/') > 0 ? '<button class="baris" id="o-sendiri">' + ikon('jalan') + '<span class="nm"><b>Jalankan berkas ini saja</b><span>Bukan berkas utama proyek</span></span></button>' +
        '<button class="baris" id="o-utama">' + ikon('folder') + '<span class="nm"><b>Jadikan berkas utama proyek</b><span id="o-utama-ket">Folder ' + esc(t.nama.split('/')[0]) + '</span></span></button>' : '') +
      (t && ekst(t.nama) === 'jrg' ? '<button class="baris" id="o-uji">' + ikon('uji') + '<span class="nm"><b>Jalankan uji</b><span>test_* dan uji_*</span></span></button>' : '') +
      (t ? '<button class="baris" id="o-argumen">' + ikon('perintah') + '<span class="nm"><b>Argumen program</b><span>' + esc(S.atur.argumen[t.nama] || 'Belum ada') + '</span></span></button>' : '') +
      (t ? '<button class="baris" id="o-ekspor">' + ikon('unduh') + '<span class="nm"><b>' + (IDE.berkas.android() ? 'Simpan ke penyimpanan HP' : 'Unduh berkas ini') + '</b></span></button>' : '') +
      (t ? '<button class="baris" id="o-cari">' + ikon('cari') + '<span class="nm"><b>Cari dan ganti</b><span>Di berkas ini</span></span></button>' : '') +
      '<button class="baris" id="o-cari-semua">' + ikon('cari') + '<span class="nm"><b>Cari di semua berkas</b></span></button>' +

      '<button class="baris" id="o-todo">' + ikon('masalah') + '<span class="nm"><b>Daftar TODO</b><span>TODO, FIXME, XXX di semua berkas</span></span></button>' +
      '<button class="baris" id="o-terhapus">' + ikon('urung') + '<span class="nm"><b>Berkas terhapus</b><span>Pulihkan dari riwayatnya</span></span></button>' +
      '<button class="baris" id="o-unggah">' + ikon('unggah') + '<span class="nm"><b>Ambil berkas dari perangkat</b></span></button>' +
      '<button class="baris" id="o-palet">' + ikon('perintah') + '<span class="nm"><b>Perintah</b><span>Ctrl+Shift+P</span></span></button>' +
      '<div class="grup">Aplikasi</div>' +
      '<button class="baris" id="o-konsol">' + ikon('konsol') + '<span class="nm"><b>Terminal</b><span>Shell perangkat ini, banyak sesi</span></span></button>' +
      (S.info.linux && S.info.linux.bisa ? '<button class="baris" id="o-linux">' + ikon('konsol') + '<span class="nm"><b>Linux</b><span>' +
        (S.info.linux.terpasang ? 'Ubuntu ' + esc(S.info.linux.versi) + ': apt, python, gcc, node, git' : 'Pasang Ubuntu: apt, python, gcc, node, git') + '</span></span></button>' : '') +
      '<button class="baris" id="o-bahasa">' + ikon('bahasa') + '<span class="nm"><b>Bahasa</b><span>Pasang dan lihat bahasa</span></span></button>' +
      '<button class="baris" id="o-perangkat">' + ikon('perangkat') + '<span class="nm"><b>Perangkat</b><span>Izin, getar, suara</span></span></button>' +
      '<button class="baris" id="o-tugas">' + ikon('perintah') + '<span class="nm"><b>Perintah proyek</b><span>Bangun, uji, jalankan dengan satu ketukan</span></span></button>' +
      '<button class="baris" id="o-logo">' + ikon('gambar') + '<span class="nm"><b>Ikon dan logo</b><span>Ganti logo IDE, buat ikon aplikasi</span></span></button>' +
      '<button class="baris" id="o-atur">' + ikon('atur') + '<span class="nm"><b>Pengaturan</b><span>Huruf, tema, mesin JRG</span></span></button>' +
      '<p class="catatan versi"><img class="logo" src="' + IDE.logo.url() + '" alt="">JRG IDE ' + esc(S.info.versi) + (S.info.varian === 'terminal' ? ' Terminal' : '') + ' · ' + esc(S.info.jrgc) + (S.info.android ? ' · Android' : '') + '</p>');
    if (t) {
      $('#o-simpan').onclick = function () { tutupLembar(); simpan(false); };
      $('#o-ganti').onclick = lembarGanti;
      $('#o-hapus').onclick = lembarHapus;
      $('#o-cari').onclick = lembarCari;
      $('#o-argumen').onclick = lembarArgumen;
      $('#o-ekspor').onclick = function () { ekspor(t.nama, false); };
      if ($('#o-uji')) $('#o-uji').onclick = function () { tutupLembar(); jalankan(true, 'uji'); };
    }
    $('#o-unggah').onclick = function () { lembarUnggah(t && t.nama.indexOf('/') > 0 ? induk(t.nama) : ''); };
    $('#o-palet').onclick = function () { palet('>'); };
    if (t && t.nama.indexOf('/') > 0) {
      var akar = t.nama.split('/')[0];
      $('#o-sendiri').onclick = function () { tutupLembar(); jalankan(true); };
      api('/api/proyek?nama=' + encodeURIComponent(t.nama)).then(function (p) {
        if (p.akar) akar = p.akar;
        if ($('#o-utama-ket')) $('#o-utama-ket').textContent = 'Folder ' + akar + (p.utama ? ' · sekarang ' + p.utama.slice(akar.length + 1) : ' · belum ada');
      }).catch(gagal);
      $('#o-utama').onclick = function () {
        simpanSemua().then(function () { return api('/api/proyek', { akar: akar, utama: t.nama.slice(akar.length + 1) }); }).then(function () {
          tutupLembar(); IDE.toast('Jalankan sekarang menjalankan ' + t.nama);
        }).catch(gagal);
      };
    }
    $('#o-cari-semua').onclick = lembarCariSemua;
    $('#o-todo').onclick = kode.todo;
    $('#o-terhapus').onclick = kode.terhapus;
    if (t) {
      var tutupLalu = function (f) { return function () { tutupLembar(); f(); }; };
      var diSunting = function (f) { return function () { keSunting(); f(); }; };
      $('#o-def').onclick = diSunting(kode.definisi); $('#o-ruj').onclick = diSunting(kode.rujukan); $('#o-gn').onclick = diSunting(kode.gantiNama);
      $('#o-dok').onclick = kode.dokumentasi; $('#o-rapi').onclick = tutupLalu(diSunting(kode.rapikan)); $('#o-riwayat').onclick = kode.riwayat;
      $('#o-naik').onclick = tutupLalu(diSunting(function () { ed.pindahBaris(-1); })); $('#o-turun').onclick = tutupLalu(diSunting(function () { ed.pindahBaris(1); }));
      $('#o-hapus-baris').onclick = tutupLalu(diSunting(function () { ed.hapusBaris(); }));
      if ($('#o-profil')) $('#o-profil').onclick = tutupLalu(function () { jalankan(false, 'profil'); });
      $('#o-di-term').onclick = tutupLalu(function () { jalankan(false, '', true); });
      $('#o-pilihan').onclick = tutupLalu(jalankanPilihan); $('#o-banding').onclick = kode.bandingkan;
    }
    $('#o-konsol').onclick = function () { tutupLembar(); alat('konsol'); fokusKonsol(); };
    if ($('#o-linux')) $('#o-linux').onclick = function () { lembarLinux(); };
    $('#o-bahasa').onclick = function () { lembarBahasa(); };
    $('#o-perangkat').onclick = function () { lembarPerangkat(); };
    $('#o-logo').onclick = function () { lembarLogo(); };
    $('#o-tugas').onclick = function () { lembarTugas(); };
    $('#o-atur').onclick = lembarAtur;
  }

  function lembarGanti() {
    var t = S.tab[S.aktif];
    lembar('Ganti nama', '<input class="isian" id="g-nama" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Nama baru">' +
      '<div class="deret"><button class="tombol utama" id="g-ok">Ganti</button></div>');
    $('#g-nama').value = t.nama;
    $('#g-ok').onclick = function () {
      var ke = $('#g-nama').value.trim();
      if (!ke || ke === t.nama) { tutupLembar(); return; }
      simpanTab(t).then(function () { return api('/api/ganti', { dari: t.nama, ke: ke }); }).then(function () {
        t.nama = ke;
        tutupLembar(); pilih(S.aktif);
        return muatBerkas();
      }).catch(gagal);
    };
  }
  function lembarHapus() {
    var t = S.tab[S.aktif];
    lembar('Hapus berkas', '<p class="catatan">Berkas <b>' + esc(t.nama) + '</b> dihapus dari perangkat ini dan tidak bisa dikembalikan.</p>' +
      '<div class="deret"><button class="tombol bahaya" id="h-ok">Hapus</button><button class="tombol" id="h-batal">Batal</button></div>');
    $('#h-batal').onclick = tutupLembar;
    $('#h-ok').onclick = function () {
      api('/api/hapus', { nama: t.nama }).then(function () {
        t.kotor = false;
        tutupLembar(); tutupTab(S.aktif);
        return muatBerkas();
      }).catch(gagal);
    };
  }

  /* ---------- cari ---------- */
  var CARI = { kata: '', ganti: '', persis: false, pola: false, utuh: false };
  /* Temuan sebagai [{i, n, ganti}]: letak, panjang, dan teks penggantinya (`$1` dst. untuk pola). */
  function temuan(v, kata) {
    var h = [], re, m;
    if (!kata) return h;
    try {
      var src = CARI.pola ? kata : kata.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      if (CARI.utuh) src = '(?<![\\w$])(?:' + src + ')(?![\\w$])';
      re = new RegExp(src, 'g' + (CARI.persis ? '' : 'i') + 'm');
    } catch (e) { CARI.salah = e.message; return h; }
    CARI.salah = '';
    while ((m = re.exec(v)) && h.length < 100000) {
      if (!m[0].length) { re.lastIndex++; continue; }
      var g = m;
      h.push({ i: m.index, n: m[0].length, ganti: !CARI.pola ? CARI.ganti : CARI.ganti.replace(/\$(\d+|&|\$)/g, function (_, k) {
        return k === '$' ? '$' : k === '&' ? g[0] : g[+k] == null ? '' : g[+k];
      }) });
    }
    return h;
  }
  function lembarCari() {
    lembar('Cari dan ganti',
      '<input class="isian" id="c-kata" placeholder="Cari" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Teks yang dicari">' +
      '<input class="isian" id="c-ganti" placeholder="Ganti dengan (boleh kosong)" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Teks pengganti">' +
      '<label class="centang"><input type="checkbox" id="c-persis"> Bedakan huruf besar dan kecil</label>' +
      '<label class="centang"><input type="checkbox" id="c-utuh"> Kata utuh</label>' +
      '<label class="centang"><input type="checkbox" id="c-pola"> Pola (regex; pengganti boleh memakai $1)</label>' +
      '<div class="deret"><button class="tombol utama" id="c-lanjut">Berikutnya</button><button class="tombol" id="c-satu">Ganti</button><button class="tombol" id="c-semua">Ganti semua</button></div>' +
      '<p class="catatan" id="c-ket"></p>');
    var kata = $('#c-kata'), ganti = $('#c-ganti');
    var pilihan = ed.terpilih();
    kata.value = pilihan && pilihan.indexOf('\n') < 0 ? pilihan : CARI.kata;
    ganti.value = CARI.ganti;
    $('#c-persis').checked = CARI.persis; $('#c-utuh').checked = CARI.utuh; $('#c-pola').checked = CARI.pola;
    function baca() { CARI.kata = kata.value; CARI.ganti = ganti.value; CARI.persis = $('#c-persis').checked; CARI.utuh = $('#c-utuh').checked; CARI.pola = $('#c-pola').checked; }
    function hitung() {
      baca();
      var n = temuan(ed.getValue(), CARI.kata).length;
      $('#c-ket').textContent = CARI.salah ? 'Pola tidak sah: ' + CARI.salah : CARI.kata ? (n ? n + ' temuan di berkas ini' : 'Tidak ditemukan') : '';
      return n;
    }
    /* Temuan pertama SESUDAH kursor; di ujung berkas berputar ke awal. */
    function berikut(dari) {
      var t = temuan(ed.getValue(), CARI.kata);
      for (var i = 0; i < t.length; i++) if (t[i].i >= dari) return t[i];
      return t.length ? t[0] : null;
    }
    kata.oninput = hitung; ganti.oninput = baca;
    ['#c-persis', '#c-utuh', '#c-pola'].forEach(function (k) { $(k).onchange = hitung; });
    $('#c-lanjut').onclick = function () {
      if (!hitung()) return;
      var x = berikut(ed.sel()[1]);
      tutupLembar(); ed.selectRange(x.i, x.i + x.n);
    };
    $('#c-satu').onclick = function () {
      if (!hitung()) return;
      var s = ed.sel(), t = temuan(ed.getValue(), CARI.kata);
      var x = t.filter(function (y) { return y.i === s[0] && y.i + y.n === s[1]; })[0] || berikut(s[1]);
      ed.gantiMutlak(x.i, x.i + x.n, x.ganti, x.i + x.ganti.length, x.i + x.ganti.length);
      var q = hitung() ? berikut(x.i + x.ganti.length) : null;
      if (q) ed.selectRange(q.i, q.i + q.n);
    };
    $('#c-semua').onclick = function () {
      var n = hitung();
      if (!n) return;
      var v = ed.getValue(), t = temuan(v, CARI.kata), keluar = '', dari = 0;
      t.forEach(function (x) { keluar += v.slice(dari, x.i) + x.ganti; dari = x.i + x.n; });
      var p = ed.sel()[0];
      ed.gantiSemua(keluar + v.slice(dari), [Math.min(p, keluar.length), Math.min(p, keluar.length)]);
      tutupLembar();
      IDE.toast(n + ' diganti');
    };
    hitung();
    setTimeout(function () { kata.focus(); kata.select(); }, 250);
  }
  function lembarCariSemua() {
    lembar('Cari di semua berkas',
      '<input class="isian" id="s-kata" placeholder="Cari di semua berkas" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Teks yang dicari">' +
      '<div id="s-hasil"><p class="catatan">Paling sedikit dua huruf. Huruf besar dan kecil dianggap sama.</p></div>');
    var inp = $('#s-kata'), timer = 0;
    inp.value = CARI.kata;
    function cari() {
      var q = inp.value.trim();
      if (q.length < 2) return;
      simpan(true).then(function () { return api('/api/cari?kata=' + encodeURIComponent(q)); }).then(function (d) {
        if (inp.value.trim() !== q) return;
        $('#s-hasil').innerHTML = d.length ? d.map(function (x, i) {
          return '<button class="baris" data-i="' + i + '">' + ikon('berkas') + '<span class="nm"><b>' + esc(x.nama) + ':' + x.baris + '</b><span class="mono">' + esc(x.teks) + '</span></span></button>';
        }).join('') + (d.length >= 200 ? '<p class="catatan">Ditampilkan 200 temuan pertama.</p>' : '') : '<p class="catatan">Tidak ditemukan.</p>';
        U.$$('#s-hasil [data-i]').forEach(function (b) {
          b.onclick = function () {
            var x = d[+b.getAttribute('data-i')];
            CARI.kata = q;
            tutupLembar();
            buka(x.nama).then(function () { ed.goto(x.baris, 1); });
          };
        });
      }).catch(gagal);
    }
    inp.oninput = function () { clearTimeout(timer); timer = setTimeout(cari, 300); };
    if (inp.value.trim().length >= 2) cari();
    setTimeout(function () { inp.focus(); }, 250);
  }

  /* Satu daftar yang bisa dicari: yang di perangkat, yang bisa dipasang, dan yang BELUM bisa
     (dengan sebabnya) — mengetik "golang" menemukan Go alih-alih kosong. */
  function lembarBahasa(kabar, kata) {
    var P = IDE.paket;
    lembar('Bahasa', '<input class="isian" id="bh-cari" placeholder="Cari bahasa, mis. python atau golang" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Cari bahasa">' +
      '<div id="bh-isi"></div><p class="catatan" id="p-kabar">' + esc(kabar || '') + '</p>' +
      '<p class="catatan">Paket di sini diunduh sekali, dicek sidiknya, lalu jalan offline.' +
        (S.info.linux && S.info.linux.bisa ? ' C, C++, Rust, Go, Java dan lainnya: pasang manual di terminal Ubuntu (<code>pkg install gcc</code>, <code>rust</code>, <code>golang</code>, ...); tombol Jalankan langsung memakainya.' : '') + '</p>');
    var inp = $('#bh-cari');
    inp.value = kata || '';
    function cocok(q, teks) {
      if (!q) return true;
      teks = teks.toLowerCase();
      return teks.split(/[\s,()]+/).some(function (k) { return k && (k.indexOf(q) === 0 || q.indexOf(k) === 0 && k.length > 1); }) || teks.indexOf(q) >= 0;
    }
    function gambar() {
      var q = inp.value.trim().toLowerCase();
      var ada = S.info.bahasa.filter(function (b) { return cocok(q, b.label + ' ' + b.id + ' ' + b.ekst.join(' ')); }).map(function (b) {
        var pk = P.untuk(b.ekst[0]), lewatPaket = !b.program && pk && P.terpasang(pk);
        return '<div class="baris">' + ikon('bahasa') + '<span class="nm"><b>' + esc(b.label) + '</b><span>.' + esc(b.ekst.join(', .')) +
          (b.program ? '' : lewatPaket ? ' · lewat paket terpasang' : ' · ' + esc(b.sebab)) + '</span></span><span class="cip" data-s="' + (b.program || lewatPaket ? 'ada' : 'tak') + '">' + (b.program || lewatPaket ? 'siap' : 'belum') + '</span></div>';
      }).join('') +
      (cocok(q, 'HTML SVG CSS web') ? '<div class="baris">' + ikon('bahasa') + '<span class="nm"><b>HTML, SVG</b><span>Dipratinjau di panel keluaran, bersama CSS dan skrip di sebelahnya</span></span><span class="cip" data-s="ada">siap</span></div>' : '') +
      (cocok(q, 'Markdown md') ? '<div class="baris">' + ikon('bahasa') + '<span class="nm"><b>Markdown</b><span>Dipratinjau di panel keluaran</span></span><span class="cip" data-s="ada">siap</span></div>' : '') +
      (cocok(q, 'JSON') ? '<div class="baris">' + ikon('bahasa') + '<span class="nm"><b>JSON</b><span>Diperiksa sah atau tidaknya</span></span><span class="cip" data-s="ada">siap</span></div>' : '');
      var pasang = Object.keys(P.daftar).filter(function (id) { var p = P.daftar[id]; return cocok(q, p.label + ' ' + id + ' ' + p.ekst.join(' ') + ' ' + p.alias); }).map(function (id) {
        var p = P.daftar[id], terpasang = P.terpasang(id);
        return '<div class="baris">' + ikon('bahasa') + '<span class="nm"><b>' + esc(p.label) + '</b><span class="panjang">.' + esc(p.ekst.join(', .')) + ' · ' + esc(p.ket) + '</span></span>' +
          '<button class="tombol' + (terpasang ? '' : ' utama') + '" data-paket="' + id + '" data-aksi="' + (terpasang ? 'copot' : 'pasang') + '">' + (terpasang ? 'Copot' : 'Pasang · ' + p.ukuran) + '</button></div>';
      }).join('');
      var nyata = S.info.bahasa.map(function (b) { return b.label; });
      var belum = P.belum.filter(function (b) { return nyata.indexOf(b.label) < 0 && cocok(q, b.label + ' ' + b.alias); }).map(function (b) {
        return '<div class="baris">' + ikon('bahasa') + '<span class="nm"><b>' + esc(b.label) + '</b><span class="panjang">' + esc(b.sebab) + '</span></span><span class="cip" data-s="tak">belum bisa</span></div>';
      }).join('');
      $('#bh-isi').innerHTML = (ada ? '<div class="grup">Di perangkat ini</div>' + ada : '') + (pasang ? '<div class="grup">Bisa dipasang</div>' + pasang : '') +
        (belum ? '<div class="grup">Belum bisa dijalankan di sini</div>' + belum : '') +
        (ada || pasang || belum ? '' : '<p class="catatan">Tidak ada bahasa bernama itu di daftar. Berkasnya tetap bisa ditulis; yang belum ada penjalannya.</p>');
      U.$$('#bh-isi [data-paket]').forEach(function (b) {
        b.onclick = function () {
          var id = b.getAttribute('data-paket');
          U.$$('#lembar [data-paket]').forEach(function (x) { x.disabled = true; });
          var kerja = b.getAttribute('data-aksi') === 'copot' ? P.copot(id) :
            P.pasang(id, function (t) { var k = $('#p-kabar'); if (k) k.textContent = t; }, S.info.cermin_paket);
          kerja.then(function () { lembarBahasa(P.daftar[id].label + (P.terpasang(id) ? ' terpasang.' : ' dicopot.'), inp.value); gambarTab(); },
            function (x) { lembarBahasa('Gagal: ' + (x.message || x), inp.value); });
        };
      });
    }
    inp.oninput = gambar;
    gambar();
  }

  /* Izin Android diminta lewat pintu `JRGAndroid` milik aktivitasnya; jawabannya peristiwa `jrg-izin`. */
  var IZIN = [['kamera', 'Kamera'], ['mikrofon', 'Mikrofon'], ['lokasi', 'Lokasi'], ['penyimpanan', 'Penyimpanan']];
  var KET_IZIN = { diberi: 'diizinkan', belum: 'belum', tidak_di_manifes: 'tidak ada di versi ini', tidak_dikenal: 'tidak dikenal' };
  /* ---------- ikon dan logo (js/logo.js) ---------- */
  var LG = { gambar: null, nama: '', bentuk: 'bulat', zoom: 100, geserX: 0, geserY: 0, ruang: 0, tepi: 0, warnaTepi: '#FFFFFF',
    latar: 'transparan', warna: '#0B1230', ke: 'semua', jalur: '' };
  /* Folder terdekat (dari `nama` ke atas) yang punya jrgm.toml; null kalau tidak ada. */
  function akarToml(nama) {
    var b = nama.split('/');
    while (b.length > 1) { b.pop(); var a = b.join('/'); if (S.berkas.some(function (f) { return f.nama === a + '/jrgm.toml'; })) return a; }
    return S.berkas.some(function (f) { return f.nama === 'jrgm.toml'; }) ? '' : null;
  }
  function keBase64(blob) {
    return new Promise(function (ok, gagal) {
      var r = new FileReader();
      r.onload = function () { ok(String(r.result).replace(/^[^,]*,/, '')); };
      r.onerror = function () { gagal(r.error || new Error('gambar tidak terbaca')); };
      r.readAsDataURL(blob);
    });
  }
  function catatanLogo() {
    var app = S.info.android ? 'Ikon aplikasinya ikut APK yang terpasang. "Pasang di layar utama" membuat pintasan dengan ikon ini; untuk ikon APK-nya sendiri, Terapkan menyimpan PNG-nya, lalu jrgm apk.'
      : 'Ikon aplikasi ditulis ke aset/ikon.png milik IDE ini; dipakai saat jrgm apk berikutnya.';
    return LG.ke === 'ide' ? 'Logo di kepala IDE, layar kosong, dan ikon tab. Langsung berganti, tersimpan di perangkat ini.'
      : LG.ke === 'aplikasi' ? app : 'Logo di dalam IDE langsung berganti. ' + app;
  }
  function lembarLogo() {
    if (!LG.jalur) { var t = S.tab[S.aktif], a = t ? akarToml(t.nama) : null; LG.jalur = a === null ? 'ikon.png' : (a ? a + '/' : '') + 'aset/ikon.png'; }
    function geser(k, min, maks, label, satuan) {
      return '<div class="geser"><span>' + label + '</span><input type="range" data-lg="' + k + '" min="' + min + '" max="' + maks + '" value="' + LG[k] + '" aria-label="' + label + '">' +
        '<span class="nilai" id="lgn-' + k + '">' + LG[k] + satuan + '</span></div>';
    }
    function pilih(k, daftar) {
      return '<div class="pilih">' + daftar.map(function (d) {
        return '<button data-lgk="' + k + '" data-v="' + d[0] + '" aria-pressed="' + (LG[k] === d[0]) + '">' + esc(d[1]) + '</button>';
      }).join('') + '</div>';
    }
    lembar('Ikon dan logo',
      '<div class="lg"><canvas id="lg-pratinjau" width="512" height="512" aria-label="Pratinjau ikon"></canvas>' +
      '<div class="lg-bentuk">' + IDE.logo.BENTUK.map(function (b) {
        return '<button data-bentuk="' + b[0] + '" aria-pressed="' + (LG.bentuk === b[0]) + '"><canvas width="96" height="96"></canvas><span>' + esc(b[1]) + '</span></button>';
      }).join('') + '</div></div>' +
      '<button class="baris" id="lg-pilih">' + ikon('gambar') + '<span class="nm"><b>Pilih gambar</b><span id="lg-ket">' + esc(LG.nama || 'Logo JRG++ bawaan') + '</span></span></button>' +
      '<input type="file" id="lg-berkas" accept="image/*" hidden>' +
      geser('zoom', 40, 250, 'Perbesar', '%') + geser('geserX', -50, 50, 'Geser X', '%') + geser('geserY', -50, 50, 'Geser Y', '%') +
      geser('ruang', 0, 30, 'Ruang tepi', '%') + geser('tepi', 0, 24, 'Garis tepi', 'px') +
      '<div class="geser"><span>Warna garis</span><input type="color" id="lg-warnaTepi" value="' + LG.warnaTepi + '" aria-label="Warna garis tepi"></div>' +
      '<div class="geser"><span>Latar</span>' + pilih('latar', [['transparan', 'Tembus'], ['warna', 'Warna'], ['kabur', 'Kabur']]) + '</div>' +
      '<div class="geser" id="lg-baris-warna"' + (LG.latar === 'warna' ? '' : ' hidden') + '><span>Warna latar</span><input type="color" id="lg-warna" value="' + LG.warna + '" aria-label="Warna latar"></div>' +
      '<div class="geser"><span>Logo</span><div class="pilih"><button id="lg-tampil" aria-pressed="' + (S.atur.logo !== false) + '">Tampil</button>' +
        '<button id="lg-polos" aria-pressed="' + (S.atur.logo === false) + '">Polos</button></div></div>' +
      '<p class="subjudul">Terapkan ke</p>' +
      '<div class="geser">' + pilih('ke', [['ide', 'Logo di IDE'], ['aplikasi', 'Ikon aplikasi'], ['semua', 'Semuanya']]) + '</div>' +
      '<p class="catatan" id="lg-cat">' + esc(catatanLogo()) + '</p>' +
      '<div class="deret"><button class="tombol utama" id="lg-terap">Terapkan</button>' +
      '<button class="tombol" id="lg-pintasan"' + (window.JRGAndroid && JRGAndroid.pintasan ? '' : ' hidden') + '>Pasang di layar utama</button>' +
      '<button class="tombol" id="lg-bawaan"' + (IDE.logo.khusus() ? '' : ' hidden') + '>Kembalikan logo bawaan</button></div>' +
      '<p class="subjudul">Ikon untuk proyek</p>' +
      '<input class="isian kecil mono" id="lg-jalur" value="' + esc(LG.jalur) + '" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Jalur PNG di proyek">' +
      '<div class="deret"><button class="tombol" id="lg-proyek">Simpan ke proyek</button><button class="tombol" id="lg-perangkat">Simpan ke perangkat</button></div>' +
      '<p class="catatan" id="lg-kabar">PNG 512 px. Kalau proyeknya punya jrgm.toml dengan [android], baris ikon-nya ikut diisi.</p>');
    var kabar = function (t) { $('#lg-kabar').textContent = t; };
    function gambarUlang() {
      IDE.logo.gambar($('#lg-pratinjau'), LG);
      U.$$('#lembar [data-bentuk]').forEach(function (b) {
        IDE.logo.gambar(b.querySelector('canvas'), Object.assign({}, LG, { bentuk: b.getAttribute('data-bentuk') }));
      });
    }
    if (LG.gambar) gambarUlang();
    else IDE.logo.muatGambar(IDE.logo.SUMBER_BAWAAN).then(function (g) { LG.gambar = g; gambarUlang(); }, function (e) { kabar('Tidak bisa: ' + e.message); });
    U.$$('#lembar [data-bentuk]').forEach(function (b) {
      b.onclick = function () {
        LG.bentuk = b.getAttribute('data-bentuk');
        U.$$('#lembar [data-bentuk]').forEach(function (x) { x.setAttribute('aria-pressed', String(x === b)); });
        gambarUlang();
      };
    });
    U.$$('#lembar [data-lg]').forEach(function (g) {
      var k = g.getAttribute('data-lg');
      g.oninput = function () { LG[k] = +g.value; $('#lgn-' + k).textContent = g.value + (k === 'tepi' ? 'px' : '%'); gambarUlang(); };
    });
    U.$$('#lembar [data-lgk]').forEach(function (b) {
      b.onclick = function () {
        var k = b.getAttribute('data-lgk');
        LG[k] = b.getAttribute('data-v');
        U.$$('#lembar [data-lgk="' + k + '"]').forEach(function (x) { x.setAttribute('aria-pressed', String(x === b)); });
        $('#lg-baris-warna').hidden = LG.latar !== 'warna';
        $('#lg-cat').textContent = catatanLogo();
        gambarUlang();
      };
    });
    [['#lg-tampil', true], ['#lg-polos', false]].forEach(function (p) {
      $(p[0]).onclick = function () {
        S.atur.logo = p[1]; simpanAtur(); terapkanAtur();
        $('#lg-tampil').setAttribute('aria-pressed', String(p[1])); $('#lg-polos').setAttribute('aria-pressed', String(!p[1]));
      };
    });
    $('#lg-warna').oninput = function () { LG.warna = this.value; gambarUlang(); };
    $('#lg-warnaTepi').oninput = function () { LG.warnaTepi = this.value; gambarUlang(); };
    $('#lg-jalur').oninput = function () { LG.jalur = this.value.trim(); };
    $('#lg-pilih').onclick = function () {
      if (IDE.berkas.apkLama(S.info)) { $('#lg-ket').textContent = IDE.berkas.BELUM_APK; return; }
      $('#lg-berkas').click();
    };
    $('#lg-berkas').onchange = function () {
      var f = this.files && this.files[0];
      if (!f) return;
      var u = URL.createObjectURL(f);
      IDE.logo.muatGambar(u).then(function (g) {
        LG.gambar = g; LG.nama = f.name; LG.zoom = 100; LG.geserX = LG.geserY = 0;
        lembarLogo();
      }, function () { $('#lg-ket').textContent = '`' + f.name + '` bukan gambar yang bisa dibaca'; URL.revokeObjectURL(u); });
    };
    $('#lg-terap').onclick = function () {
      var b = this; b.disabled = true;
      var kerja = [], pesan = [];
      if (LG.ke !== 'aplikasi') kerja.push(IDE.logo.png(LG).then(IDE.logo.simpan).then(function () { pesan.push('logo IDE diganti'); }));
      if (LG.ke !== 'ide') {
        if (S.info.android) kerja.push(IDE.logo.png(LG).then(function (blob) {
          return IDE.berkas.unggah(api, [new File([blob], 'ikon-aplikasi.png', { type: 'image/png' })], '');
        }).then(function () { return muatBerkas(); }).then(function () { return IDE.berkas.ekspor(api, S.info, 'ikon-aplikasi.png', false); })
          .then(function (j) {
            if (j && j !== 'ditanyakan' && j !== 'diunduh') throw new Error(j);
            pesan.push('PNG ikon aplikasi ada di ikon-aplikasi.png dan sedang disimpan ke perangkat; salin ke ide/aset/ikon.png lalu jrgm apk');
          }));
        else {
          var tulis = function (jalur, kanvas) {
            return new Promise(function (ok) { IDE.logo.gambar(kanvas, LG).toBlob(ok, 'image/png'); }).then(keBase64)
              .then(function (isi) { return api('/api/ikon_ide', { jalur: jalur, isi: isi }); });
          };
          kerja.push(tulis('aset/ikon.png', IDE.logo.kanvas(512)).then(function (d) { pesan.push('ikon aplikasi ditulis ke ' + d.jalur); }));
          if (LG.ke === 'semua') kerja.push(tulis('web/aset/logo.png', IDE.logo.kanvas(128)));
        }
      }
      Promise.all(kerja).then(function () {
        b.disabled = false;
        $('#lg-bawaan').hidden = !IDE.logo.khusus();
        var t = pesan.join('; ');
        kabar(t.charAt(0).toUpperCase() + t.slice(1) + '.');
        IDE.toast(LG.ke !== 'ide' && S.info.android ? 'Ikon layar utama berganti lewat APK baru' : 'Diterapkan');
      }).catch(function (e) { b.disabled = false; kabar('Tidak bisa: ' + (e.message || e)); });
    };
    /* Pintasan bersemat dengan ikon ini (APK sejak JRGM-3); sistem yang menanyakan konfirmasinya. */
    $('#lg-pintasan').onclick = function () {
      var b64 = $('#lg-pratinjau').toDataURL('image/png').split(',')[1];
      var h = JRGAndroid.pintasan('JRG IDE', b64);
      kabar(h === 'ditanyakan' ? 'Konfirmasi di dialog sistem; pintasannya muncul di layar utama.'
        : h === 'tidak_didukung' ? 'Peluncur HP ini tidak mendukung pintasan bersemat.' : 'Tidak bisa: ' + h);
    };
    $('#lg-bawaan').onclick = function () { IDE.logo.hapus().then(function () { lembarLogo(); IDE.toast('Logo bawaan dipakai lagi'); }); };
    function keProyek() {
      var jalur = LG.jalur.replace(/^\/+/, '');
      if (!/\.png$/i.test(jalur)) return Promise.reject(new Error('jalurnya harus berakhiran .png'));
      var i = jalur.lastIndexOf('/'), dir = i < 0 ? '' : jalur.slice(0, i);
      return IDE.logo.png(LG).then(function (blob) {
        var f = new File([blob], jalur.slice(i + 1), { type: 'image/png' });
        return IDE.berkas.unggah(api, [f], dir);
      }).then(function (h) {
        if (h.gagal.length) throw new Error(h.gagal[0]);
        return muatBerkas();
      }).then(function () { return jalur; });
    }
    $('#lg-proyek').onclick = function () {
      keProyek().then(function (jalur) {
        var akar = akarToml(jalur);
        if (akar === null) { kabar('Tersimpan di ' + jalur + '.'); return; }
        var toml = (akar ? akar + '/' : '') + 'jrgm.toml', rel = akar ? jalur.slice(akar.length + 1) : jalur;
        return api('/api/baca?nama=' + encodeURIComponent(toml)).then(function (d) {
          var isi = d.isi, m = /^\[android\][^\n]*\n/m.exec(isi);
          if (!m) { kabar('Tersimpan di ' + jalur + '. ' + toml + ' belum punya [android]; tambahkan ikon = "' + rel + '" di sana.'); return; }
          var awal = m.index + m[0].length, ujung = isi.slice(awal).search(/^\[/m), bagian = ujung < 0 ? isi.slice(awal) : isi.slice(awal, awal + ujung);
          var baris = 'ikon     = "' + rel + '"';
          bagian = /^ikon\s*=.*$/m.test(bagian) ? bagian.replace(/^ikon\s*=.*$/m, baris) : baris + '\n' + bagian;
          var baru = isi.slice(0, awal) + bagian + (ujung < 0 ? '' : isi.slice(awal + ujung));
          var ti = S.tab.map(function (t) { return t.nama; }).indexOf(toml);
          if (ti >= 0 && S.tab[ti].kotor) { kabar('Tersimpan di ' + jalur + '. ' + toml + ' sedang disunting; isi ikon = "' + rel + '" sendiri.'); return; }
          return api('/api/simpan', { nama: toml, isi: baru }).then(function () {
            if (ti >= 0) { S.tab[ti].isi = baru; S.tab[ti].keadaan = null; if (ti === S.aktif) { ed.milik = null; pilih(ti); } }
            kabar('Tersimpan di ' + jalur + ', dan ' + toml + ' memakainya sebagai ikon APK.');
          });
        });
      }).catch(function (e) { kabar('Tidak bisa: ' + (e.message || e)); });
    };
    $('#lg-perangkat').onclick = function () {
      if (!IDE.berkas.android()) {
        IDE.logo.png(LG).then(function (blob) { IDE.berkas.simpanBlob(blob, LG.jalur.slice(LG.jalur.lastIndexOf('/') + 1) || 'ikon.png'); kabar('Diunduh.'); });
        return;
      }
      keProyek().then(function (jalur) { return IDE.berkas.ekspor(api, S.info, jalur, false); })
        .then(function (j) { if (j && j !== 'ditanyakan' && j !== 'diunduh') kabar('Tidak bisa: ' + j); })
        .catch(function (e) { kabar('Tidak bisa: ' + (e.message || e)); });
    };
  }

  /* Linux (Ubuntu di dalam proot, versi Terminal; src/linux.jrg). Pemasangan maju tiap kali `/api/linux`
     ditanya, jadi lembar ini bertanya selama ia terbuka dan pemasangannya belum selesai. */
  var tanyaLinux = 0;
  function lembarLinux() {
    lembar('Linux', '<div id="lx-isi"><p class="catatan">Memeriksa…</p></div>');
    var hapusYakin = false, nomor = ++tanyaLinux;
    function berjalan(d) { return d.tahap && d.tahap !== 'siap' && d.tahap !== 'gagal'; }
    function mb(n) { return (n / 1048576).toFixed(1) + ' MB'; }
    function gambar(d) {
      var el = $('#lx-isi');
      if (!el) return;
      var h = '';
      if (berjalan(d)) {
        var persen = d.total > 0 ? Math.min(100, Math.round(d.dapat * 100 / d.total)) : 0;
        h = '<div class="grup">' + esc(d.pesan) + '</div>' +
          (d.tahap === 'unduh' || d.tahap === 'tarik' ? '<div class="meter"><i style="width:' + persen + '%"></i></div><p class="catatan">' +
            (d.total > 0 ? mb(d.dapat) + ' / ' + mb(d.total) : 'Menyambung…') + '</p>' : '<p class="catatan">Sebentar…</p>') +
          '<div class="deret"><button class="tombol" id="lx-batal">Batal</button></div>';
      } else if (d.terpasang) {
        h = '<div class="grup">Ubuntu ' + esc(d.versi) + (d.seccomp === false ? ' · mode kompatibel' : '') + '</div>' +
          '<div class="kisi"><button class="tombol utama" id="lx-buka">Terminal Ubuntu</button><button class="tombol" id="lx-hapus">Hapus</button></div>' +
          '<p class="catatan">Paket: <code>pkg install python gcc nodejs git</code> (atau <code>apt</code>, <code>sudo</code> juga jalan). ' +
          'Berkas IDE ada di <code>~/berkas</code>, <code>buka &lt;berkas&gt;</code> membuka penyunting, <code>jrgc</code> ikut tersedia. ' +
          'Memori HP: <code>setup-storage</code>, lalu <code>~/storage</code>.</p>' +
          '<p class="catatan">Bahasa yang tidak ada di HP (Python, C, C++, Rust, Java, Node) dijalankan tombol Jalankan lewat Ubuntu begitu dipasang di sana. Dari shell Android: <code>linux</code>.</p>' +
          (d.sumber ? '<div class="grup">Sumber JRG</div><p class="catatan">Kode sumber bahasa JRG (jrgc, jrgm, pustaka, contoh) ikut di aplikasi ini. ' +
            'Dibongkar ke <code>berkas/jrg-lang</code>; bangun di Ubuntu: <code>pkg install rust</code>, lalu <code>cargo install --path .</code> di foldernya.</p>' +
            '<div class="deret"><button class="tombol" id="lx-sumber">Bongkar ke berkas/jrg-lang</button></div>' : '');
      } else {
        h = (d.tahap === 'gagal' && d.pesan ? '<p class="catatan galat">' + esc(d.pesan) + '</p>' : '') +
          '<div class="grup">Ubuntu 24.04 LTS</div>' +
          '<p class="catatan">Linux sungguhan di dalam IDE, tanpa root: apt dan seluruh paket Ubuntu. Unduhan ±30 MB, terpasang ±150 MB, ' +
          'sesudah itu paket dasar diunduh sekali saat terminal Ubuntu pertama dibuka.</p>' +
          '<div class="deret"><button class="tombol utama" id="lx-pasang">' + (d.tahap === 'gagal' ? 'Coba lagi' : 'Pasang') + '</button></div>' +
          (d.arsip ? '<p class="catatan">Tanpa internet: taruh <code>' + esc(d.arsip) + '</code> dari cdimage.ubuntu.com di folder berkas, lalu Pasang.</p>' : '');
      }
      el.innerHTML = h;
      if ($('#lx-batal')) $('#lx-batal').onclick = function () { api('/api/linux/batal', {}).then(tanya, gagal); };
      if ($('#lx-pasang')) $('#lx-pasang').onclick = function () { api('/api/linux/pasang', {}).then(tanya, gagal); };
      if ($('#lx-buka')) $('#lx-buka').onclick = function () {
        tutupLembar(); sesiTerminal(undefined, true);
      };
      if ($('#lx-sumber')) $('#lx-sumber').onclick = function () {
        var t = this; t.disabled = true; t.textContent = 'Membongkar…';
        api('/api/linux/sumber', {}).then(function () { t.textContent = 'Sudah di berkas/jrg-lang'; muatBerkas(); IDE.toast('Sumber JRG di berkas/jrg-lang'); },
          function (e) { t.disabled = false; t.textContent = 'Bongkar ke berkas/jrg-lang'; gagal(e); });
      };
      if ($('#lx-hapus')) $('#lx-hapus').onclick = function () {
        /* Konfirmasi di halaman (confirm() tidak muncul di WebView): ketukan kedua yang menghapus. */
        if (!hapusYakin) { hapusYakin = true; this.textContent = 'Hapus semua isinya?'; this.classList.add('bahaya'); return; }
        this.disabled = true; this.textContent = 'Menghapus…';
        api('/api/linux/hapus', {}).then(function () { IDE.toast('Linux dihapus'); segarkanInfo(); tanya(); }, gagal);
      };
    }
    function tanya() {
      if (nomor !== tanyaLinux || !$('#lx-isi')) return;
      api('/api/linux').then(function (d) {
        var dulu = S.info.linux || {};
        S.info.linux = d;
        gambar(d);
        if (dulu.terpasang !== d.terpasang) segarkanInfo();
        if (berjalan(d)) setTimeout(tanya, 700);
      }, function (e) { gagal(e); });
    }
    tanya();
  }
  /* Info pelayan dibaca ulang: daftar bahasa berubah sesudah Linux dipasang atau dihapus. */
  function segarkanInfo() { return api('/api/info').then(function (d) { S.info = d; }).catch(function () {}); }

  function perintahTermux() {
    var u = 'http://127.0.0.1:' + (location.port || '80');
    return 'curl -s ' + u + '/termux.sh > $PREFIX/bin/jrg-ide && chmod +x $PREFIX/bin/jrg-ide';
  }
  function lembarPerangkat(kabar) {
    var A = window.JRGAndroid;
    lembar('Perangkat', '<div class="grup">Izin</div>' + (A ? IZIN.map(function (z) {
      var s = A.izin(z[0]);
      if (s === 'tidak_di_manifes' || s === 'tidak_dikenal') return '';
      return '<div class="baris">' + ikon('perangkat') + '<span class="nm"><b>' + z[1] + '</b><span>' + (KET_IZIN[s] || s) + '</span></span>' +
        '<button class="tombol' + (s === 'belum' ? ' utama' : '') + '" data-izin="' + z[0] + '" data-s="' + s + '">' + (s === 'diberi' ? 'Coba' : 'Minta') + '</button></div>';
    }).join('') : '<p class="catatan">Izin cuma ada di aplikasi Android.</p>') +
    '<div class="grup">Coba</div>' +
    '<div class="kisi"><button class="tombol" id="pr-getar">Getar</button><button class="tombol" id="pr-suara">Suara</button></div>' +
    '<div id="pr-coba"></div><p class="catatan" id="pr-kabar">' + esc(kabar || '') + '</p>' +
    /* Termux bertukar berkas dengan IDE lewat pelayan ini (linux/termux.sh). */
    '<div class="grup">Termux</div><p class="catatan">Di Termux, pasang perintah <code>jrg-ide</code> sekali:</p>' +
    '<pre class="mono catatan" id="pr-termux" style="white-space:pre-wrap;user-select:all">' + esc(perintahTermux()) + '</pre>' +
    '<div class="deret"><button class="tombol" id="pr-termux-salin">Salin</button></div>' +
    '<p class="catatan">Lalu <code>jrg-ide ls</code>, <code>jrg-ide ambil &lt;nama&gt;</code>, <code>jrg-ide kirim &lt;berkas&gt;</code>. ' +
    'Folder bersama lewat memori HP: <code>~/storage/shared/JRG-bersama</code> di Termux, <code>~/bersama</code> di Ubuntu.</p>');
    $('#pr-termux-salin').onclick = function () { term.salin(perintahTermux()); };
    $('#pr-getar').onclick = function () {
      /* Di APK lewat penggetar Android langsung: `navigator.vibrate` milik WebView diam saat HP senyap. */
      var A = window.JRGAndroid, h = A && A.getar ? A.getar('120,80,120,80,120') : (navigator.vibrate && navigator.vibrate([120, 80, 120, 80, 120]) ? 'peramban' : 'tidak_bisa');
      $('#pr-kabar').textContent = h === 'ok' || h === 'peramban' ? 'Getar dikirim. Tidak terasa? Periksa kekuatan getar di Setelan.'
        : h === 'tidak_ada' ? 'Tidak ada penggetar.' : h === 'tidak_di_manifes' ? 'Versi ini tanpa izin getar.'
        : h === 'tidak_bisa' ? 'Tidak bisa bergetar di sini.' : 'Gagal: ' + h;
    };
    $('#pr-suara').onclick = function () {
      tutupLembar();
      buka('suara.jrg').then(function () { var t = S.tab[S.aktif]; if (t && t.nama === 'suara.jrg') jalankan(); });
    };
    U.$$('#lembar [data-izin]').forEach(function (b) {
      b.onclick = function () {
        var nama = b.getAttribute('data-izin');
        if (b.getAttribute('data-s') === 'diberi') return cobaIzin(nama);
        var j = A.mintaIzin(nama);
        if (j !== 'ditanyakan') lembarPerangkat(KET_IZIN[j] || j);
      };
    });
  }
  function cobaIzin(nama) {
    var kabar = function (t) { if ($('#pr-kabar')) $('#pr-kabar').textContent = t; };
    if (nama === 'penyimpanan') return kabar('Penyimpanan HP ada di /sdcard; buka dari Terminal: cd /sdcard');
    if (nama === 'lokasi') {
      kabar('Mencari lokasi…');
      return navigator.geolocation.getCurrentPosition(function (p) {
        kabar('Lokasi: ' + p.coords.latitude.toFixed(5) + ', ' + p.coords.longitude.toFixed(5) + ' (±' + Math.round(p.coords.accuracy) + ' m)');
      }, function (x) { kabar('Lokasi tidak didapat: ' + x.message); }, { timeout: 15000 });
    }
    var mau = nama === 'kamera' ? { video: true } : { audio: true };
    if (!navigator.mediaDevices) return kabar('Peramban ini tidak punya getUserMedia.');
    navigator.mediaDevices.getUserMedia(mau).then(function (aliran) {
      var henti = function () { aliran.getTracks().forEach(function (t) { t.stop(); }); };
      if (nama === 'kamera' && $('#pr-coba')) {
        $('#pr-coba').innerHTML = '<video autoplay playsinline muted style="width:100%;border-radius:8px"></video>';
        $('#pr-coba video').srcObject = aliran;
        kabar('Kamera terbuka 8 detik.');
      } else ukurMikrofon(aliran, kabar);
      setTimeout(function () { henti(); if ($('#pr-coba')) $('#pr-coba').innerHTML = ''; }, nama === 'kamera' ? 8000 : 5000);
    }, function (x) { kabar(nama + ' tidak bisa dibuka: ' + (x.message || x.name)); });
  }

  /* Lima detik kerasnya suara, supaya terbukti mikrofonnya MENDENGAR, bukan cuma terbuka. */
  function ukurMikrofon(aliran, kabar) {
    var AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return kabar('Mikrofon terbuka (' + aliran.getAudioTracks().length + ' jalur).');
    var ac = new AC(), an = ac.createAnalyser(), data = new Uint8Array(1024), puncak = 0, n = 0;
    ac.createMediaStreamSource(aliran).connect(an);
    an.fftSize = 2048;
    if ($('#pr-coba')) $('#pr-coba').innerHTML = '<div class="meter"><i></i></div>';
    var detak = setInterval(function () {
      an.getByteTimeDomainData(data);
      var maks = 0;
      for (var i = 0; i < data.length; i++) maks = Math.max(maks, Math.abs(data[i] - 128));
      puncak = Math.max(puncak, maks);
      var batang = $('#pr-coba .meter i');
      if (batang) batang.style.width = Math.min(100, Math.round(maks / 1.28)) + '%';
      kabar('Mikrofon mendengarkan — bicaralah. Paling keras sejauh ini ' + Math.round(puncak / 1.28) + '%');
      if (++n >= 48) {
        clearInterval(detak); ac.close();
        kabar(puncak > 2 ? 'Mikrofon bekerja: suara paling keras ' + Math.round(puncak / 1.28) + '%.' : 'Mikrofon terbuka tapi tidak menangkap suara apa pun.');
      }
    }, 100);
  }

  function lembarKamus() {
    lembar('Kamus fungsi JRG', '<input class="isian" id="k-cari" placeholder="cari, mis. sort atau str::" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Cari fungsi">' +
      '<div id="k-hasil"><p class="catatan">Daftarnya diambil langsung dari mesin JRG di perangkat ini, jadi selalu cocok dengan yang menjalankan kodemu.</p></div>');
    var inp = $('#k-cari'), timer = 0;
    var kata = ed && S.tab[S.aktif] ? (ed.wordAt ? ed.wordAt() : '') : '';
    function cari() {
      var q = inp.value.trim();
      if (q.length < 2) return;
      api('/api/kamus?kata=' + encodeURIComponent(q)).then(function (d) {
        var baris = d.teks.split('\n').filter(function (l) { return /^[A-Za-z_][\w:]*$/.test(l.trim()); }).slice(0, 80);
        $('#k-hasil').innerHTML = baris.length ? baris.map(function (l) {
          return '<button class="baris" data-f="' + esc(l.trim()) + '"><span class="nm"><b>' + esc(l.trim()) + '</b></span></button>';
        }).join('') : '<div class="dok">' + esc(d.teks || 'Tidak ada yang cocok.') + '</div>';
        U.$$('#k-hasil [data-f]').forEach(function (b) {
          b.onclick = function () {
            api('/api/dok?nama=' + encodeURIComponent(b.getAttribute('data-f'))).then(function (x) {
              $('#k-hasil').innerHTML = '<div class="dok">' + esc(x.teks) + '</div><div class="deret"><button class="tombol" id="k-sisip">Sisipkan namanya</button></div>';
              $('#k-sisip').onclick = function () { tutupLembar(); if (S.tab[S.aktif]) ed.insert(b.getAttribute('data-f')); };
            }).catch(gagal);
          };
        });
      }).catch(gagal);
    }
    inp.oninput = function () { clearTimeout(timer); timer = setTimeout(cari, 300); };
    if (typeof kata === 'string' && kata.length > 1) { inp.value = kata; cari(); }
    setTimeout(function () { inp.focus(); }, 250);
  }

  function lembarAtur() {
    function pilihan(kunci, daftar) {
      return '<div class="pilih">' + daftar.map(function (d) {
        return '<button data-k="' + kunci + '" data-v="' + esc(String(d[0])) + '" aria-pressed="' + (String(S.atur[kunci]) === String(d[0])) + '">' + esc(d[1]) + '</button>';
      }).join('') + '</div>';
    }
    lembar('Pengaturan',
      '<div class="geser"><span>Ukuran huruf</span>' + pilihan('huruf', [[12, '12'], [14, '14'], [16, '16'], [18, '18'], [20, '20']]) + '</div>' +
      '<div class="geser"><span>Baris panjang</span>' + pilihan('bungkus', [[false, 'Gulir'], [true, 'Bungkus']]) + '</div>' +
      '<div class="geser"><span>Tampilan</span>' + pilihan('mode', [['dark', 'Malam'], ['light', 'Siang']]) + '</div>' +
      '<div class="geser"><span>Tema</span>' + pilihan('tema', [['mint', 'Mint'], ['air', 'Air'], ['laut', 'Laut'], ['senja', 'Senja'], ['mawar', 'Mawar']]) + '</div>' +
      '<div class="geser"><span>Logo</span>' + pilihan('logo', [[true, 'Tampil'], [false, 'Polos']]) + '</div>' +
      '<div class="geser"><span>Rapikan saat simpan</span>' + pilihan('rapiSimpan', [[true, 'Nyala'], [false, 'Mati']]) + '</div>' +
      '<div class="geser"><span>Saran ketik</span>' + pilihan('saran', [[true, 'Nyala'], [false, 'Mati']]) + '</div>' +
      '<div class="geser"><span>Periksa galat</span>' + pilihan('periksa', [[true, 'Nyala'], [false, 'Mati']]) + '</div>' +
      '<div class="geser"><span>Tombol Jalankan</span>' + pilihan('jalanDi', [['keluar', 'Panel Keluaran'], ['terminal', 'Terminal']]) + '</div>' +
      '<div class="geser"><span>Mesin JRG</span>' + pilihan('mesin', [['interp', 'Interpreter'], ['vm', 'VM']].concat(S.info.llvm ? [['llvm', 'Native']] : [])) + '</div>' +
      '<p class="catatan">Interpreter mendukung seluruh bahasa JRG; VM lebih cepat. Keduanya memberi jawaban yang sama.' + (S.info.llvm ? ' Native mengompilasi lewat LLVM (butuh clang) dan belum menerima seluruh bahasa' + (S.info.android ? '; di HP ia jalan di Ubuntu, tanpa dok::, crypto::, re:: dan ptr::.' : '.') :
        S.info.varian === 'terminal' ? ' Native (LLVM) muncul sesudah clang dipasang di Ubuntu: Lainnya > Linux, lalu pkg install clang.' :
        S.info.android ? ' Native (LLVM) cuma ada di JRG IDE Terminal.' : '') + '</p>' +
      bagianLatar() +
      '<p class="subjudul">Cadangan</p>' +
      '<button class="baris" id="at-cadang">' + ikon('unduh') + '<span class="nm"><b>Cadangkan pengaturan</b><span>Ke berkas pengaturan-ide.json di daftar berkas</span></span></button>' +
      '<button class="baris" id="at-pulih">' + ikon('unggah') + '<span class="nm"><b>Pulihkan pengaturan</b><span id="at-pulih-ket">Dari pengaturan-ide.json</span></span></button>');
    pasangLatar();
    $('#at-cadang').onclick = function () {
      var salin = JSON.parse(JSON.stringify(S.atur));
      delete salin.kursor; delete salin.sesi; delete salin.terakhir;
      api('/api/simpan', { nama: 'pengaturan-ide.json', isi: JSON.stringify(salin, null, 2) + '\n' }).then(muatBerkas)
        .then(function () { IDE.toast('Tersimpan di pengaturan-ide.json'); }, gagal);
    };
    $('#at-pulih').onclick = function () {
      api('/api/baca?nama=pengaturan-ide.json').then(function (d) {
        var baru = JSON.parse(d.isi);
        if (!baru || typeof baru !== 'object' || Array.isArray(baru)) throw new Error('isinya bukan objek pengaturan');
        Object.keys(baru).forEach(function (k) { if (k in S.atur && k !== 'kursor' && k !== 'sesi') S.atur[k] = baru[k]; });
        simpanAtur(); terapkanAtur(); lembarAtur(); IDE.toast('Pengaturan dipulihkan');
      }).catch(function (e) { $('#at-pulih-ket').textContent = 'Tidak bisa: ' + (e.message || e); });
    };
    U.$$('#lembar [data-k]').forEach(function (b) {
      b.onclick = function () {
        var k = b.getAttribute('data-k'), v = b.getAttribute('data-v');
        S.atur[k] = k === 'huruf' ? +v : k === 'bungkus' || k === 'saran' || k === 'periksa' || k === 'latarGerak' || k === 'logo' || k === 'rapiSimpan' ? v === 'true' : v;
        simpanAtur(); terapkanAtur(); lembarAtur();
        if (k === 'periksa' && !S.atur.periksa) { S.masalah.daftar = []; S.masalah.umum = {}; segarkanMasalah(); }
        if (k === 'periksa' && S.atur.periksa && S.tab[S.aktif]) periksaNanti(S.tab[S.aktif].nama, 0);
      };
    });
  }
  /* ---------- latar (js/latar.js) ---------- */
  function ukuranTeks(n) { return n >= 1048576 ? (n / 1048576).toFixed(1) + ' MB' : Math.max(1, Math.round(n / 1024)) + ' KB'; }
  function bagianLatar() {
    var L = IDE.latar.info();
    function geser(k, min, maks, label, satuan) {
      return '<div class="geser"><span>' + label + '</span><input type="range" data-g="' + k + '" min="' + min + '" max="' + maks + '" value="' + (+S.atur[k] || 0) + '" aria-label="' + label + '">' +
        '<span class="nilai" id="gn-' + k + '">' + (+S.atur[k] || 0) + satuan + '</span></div>';
    }
    return '<p class="subjudul">Latar</p>' +
      '<button class="baris" id="lt-pilih">' + ikon('unggah') + '<span class="nm"><b>' + (L ? 'Ganti latar' : 'Pilih foto atau video') + '</b><span id="lt-ket">' +
        (L ? esc(L.nama) + ' · ' + (L.jenis === 'video' ? 'video' : 'gambar') + ' · ' + ukuranTeks(L.ukuran) : 'Dari galeri atau penyimpanan; ukuran dan resolusinya tidak diubah') + '</span></span></button>' +
      (L ? '<button class="baris" id="lt-hapus">' + ikon('hapus') + '<span class="nm"><b>Hapus latar</b></span></button>' +
        geser('latarRedup', 0, 90, 'Redupkan', '%') +
        geser('latarTembus', 0, 90, 'Panel tembus', '%') +
        (L.jenis === 'gambar' ? geser('latarKabur', 0, 24, 'Kaburkan', 'px') : '') +
        '<div class="geser"><span>Isi layar</span>' + pilihanAtur('latarIsi', [['penuh', 'Penuh'], ['utuh', 'Utuh']]) + '</div>' +
        (L.jenis === 'video' ? '<div class="geser"><span>Video</span>' + pilihanAtur('latarGerak', [[true, 'Putar'], [false, 'Diam']]) + '</div>' +
          '<p class="catatan">Video diputar tanpa suara dan berhenti saat aplikasi tidak terlihat. Kabur tidak tersedia untuk video, supaya tetap ringan.</p>' : '') : '') +
      '<input type="file" id="lt-berkas" accept="image/*,video/*" hidden>';
  }
  function pilihanAtur(kunci, daftar) {
    return '<div class="pilih">' + daftar.map(function (d) {
      return '<button data-k="' + kunci + '" data-v="' + esc(String(d[0])) + '" aria-pressed="' + (String(S.atur[kunci]) === String(d[0])) + '">' + esc(d[1]) + '</button>';
    }).join('') + '</div>';
  }
  function pasangLatar() {
    $('#lt-pilih').onclick = function () {
      if (IDE.berkas.apkLama(S.info)) { $('#lt-ket').textContent = IDE.berkas.BELUM_APK; return; }
      $('#lt-berkas').click();
    };
    $('#lt-berkas').onchange = function () {
      var f = this.files && this.files[0];
      if (!f) return;
      $('#lt-ket').textContent = 'Menyimpan ' + f.name + ' (' + ukuranTeks(f.size) + ')…';
      IDE.latar.simpan(f, S.atur).then(function () { lembarAtur(); }, function (e) { $('#lt-ket').textContent = 'Tidak bisa: ' + (e.message || e); });
    };
    if ($('#lt-hapus')) $('#lt-hapus').onclick = function () { IDE.latar.hapus(S.atur).then(lembarAtur); };
    U.$$('#lembar [data-g]').forEach(function (g) {
      var k = g.getAttribute('data-g');
      g.oninput = function () {
        S.atur[k] = +g.value;
        $('#gn-' + k).textContent = g.value + (k === 'latarKabur' ? 'px' : '%');
        IDE.latar.terapkan(S.atur);
      };
      g.onchange = simpanAtur;
    });
  }
  function terapkanAtur() {
    document.documentElement.setAttribute('data-mode', S.atur.mode);
    document.documentElement.setAttribute('data-tema', S.atur.tema);
    document.documentElement.classList.toggle('tanpa-logo', S.atur.logo === false);
    var warna = getComputedStyle(document.documentElement).getPropertyValue('--bg-2').trim();
    if (warna) $('meta[name=theme-color]').setAttribute('content', warna);
    if (ed) { ed.setFont(S.atur.huruf); ed.setWrap(S.atur.bungkus); }
    IDE.latar.terapkan(S.atur);
  }

  /* ---------- mulai ---------- */
  bangun();
  terapkanAtur();
  gambarTab();
  IDE.latar.muat(S.atur);
  IDE.logo.muat();
  /* Info, paket, daftar berkas dan isi tab sesi sebelumnya diminta serentak (dulu berurutan: empat
     perjalanan ke pelayan sebelum penyunting terisi). Tab yang berkasnya sudah tidak ada dibuang. */
  var baca = function (n) { return api('/api/baca?nama=' + encodeURIComponent(n)).catch(function () { return null; }); };
  var sesiAwal = (S.atur.sesi || []).slice(0, 20), isiAwal = sesiAwal.map(baca);
  Promise.all([api('/api/info'), IDE.paket.segarkan(), muatBerkas()]).then(function (d) {
    S.info = d[0];
    /* Android membuat ulang aplikasinya: yang tadi di terminal kembali ke terminal, sesinya pulih dari pelayan. */
    if (S.atur.diKonsol) { alat('konsol'); if (S.atur.konsol === 'baris') konsol.fokus(); else term.tampil().catch(function () {}); }
    var ada = function (n) { return S.berkas.some(function (f) { return f.nama === n; }); };
    var awal = ada(S.atur.terakhir) ? S.atur.terakhir : ada('halo.jrg') ? 'halo.jrg' : (S.berkas[0] && S.berkas[0].nama);
    var nama = sesiAwal.filter(ada), janji = isiAwal.filter(function (_, i) { return ada(sesiAwal[i]); });
    if (awal && nama.indexOf(awal) < 0) { nama.push(awal); janji.push(baca(awal)); }
    if (!nama.length) { gambarTab(); return; }
    return Promise.all(janji).then(function (d) {
      d.forEach(function (x) { if (x) S.tab.push(tabBaru(x)); });
      var i = S.tab.map(function (t) { return t.nama; }).indexOf(awal);
      pilih(i < 0 ? 0 : i);
      if (S.tab[S.aktif]) periksaNanti(S.tab[S.aktif].nama, 0);
    });
  }).catch(function (e) {
    $('#kosong').hidden = false;
    $('#kosong').innerHTML = '<div><b>Pelayan JRG tidak menjawab</b>' + esc(e.message || e) + '</div>';
  });
})(window.IDE);
