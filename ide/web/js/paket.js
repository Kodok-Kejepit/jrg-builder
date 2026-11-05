/* JRG IDE — js/paket.js: bahasa yang DIPASANG dari dalam IDE. Berkasnya diunduh sekali, sidiknya
   diperiksa, lalu disimpan pelayan di perangkat (`/paket/...`). Kenapa di halaman dan bukan
   sebagai proses: ide/README.md, "Bahasa yang dipasang". */
(function (IDE) {
  'use strict';
  var PY = 'https://cdn.jsdelivr.net/pyodide/v0.28.3/full/';
  var PAKET = {
    python: {
      label: 'Python 3.13', ukuran: '12 MB', ekst: ['py'], cara: 'python', alias: 'py python3 pyodide numpy pandas',
      ket: 'CPython asli (Pyodide). Pustaka seperti numpy dipasang sendiri saat diimpor. Masukan diisi SEBELUM dijalankan.',
      berkas: [
        { nama: 'pyodide.js', url: PY + 'pyodide.js', sha256: '24a458425dcb4ea9836eb5ce26701d18cb769374e2b79247602ba605bf093278' },
        { nama: 'pyodide.asm.js', url: PY + 'pyodide.asm.js', sha256: 'b22e5831eade9ff10e6fe2c811c68688cd91f10154377b4f80debcf5bafa1e56' },
        { nama: 'pyodide.asm.wasm', url: PY + 'pyodide.asm.wasm', sha256: '5effb6a1a6cc4a1a85bec4622701aa797c031e1de923cbbaf2ad47abdc4ab325' },
        { nama: 'python_stdlib.zip', url: PY + 'python_stdlib.zip', sha256: '71fee17f88a6260ec8c9c7c063533ee59c021fdc88a1ce76247378d3c4a35f4c' },
        { nama: 'pyodide-lock.json', url: PY + 'pyodide-lock.json', sha256: 'f6e6f42f451f42affbbcddb00e8c9a3278dcbf399f57aab9f3f568839a7ff4a6' }
      ]
    },
    ts: {
      label: 'TypeScript 5.7', ukuran: '3,5 MB', ekst: ['ts'], cara: 'kompilasi', alias: 'ts tsc',
      ket: 'Kompilator resmi: kode diubah jadi JavaScript lalu dijalankan penjalan JavaScript. Sintaks diperiksa, kecocokan tipe tidak.',
      berkas: [
        { nama: 'typescript.min.js', url: 'https://cdn.jsdelivr.net/npm/typescript@5.7.2/lib/typescript.min.js', sha256: '1ec660c2b865656e6a5f5e27152f2b4d32798512c6e0782d5aafb73d7e61275b' }
      ]
    },
    coffee: {
      label: 'CoffeeScript 2.7', ukuran: '0,5 MB', ekst: ['coffee'], cara: 'kompilasi', alias: 'coffee',
      ket: 'Kompilator resmi: kode diubah jadi JavaScript lalu dijalankan penjalan JavaScript.',
      berkas: [
        { nama: 'coffeescript.js', url: 'https://cdn.jsdelivr.net/npm/coffeescript@2.7.0/lib/coffeescript-browser-compiler-legacy/coffeescript.js', sha256: '1d497742c4391dbaae63557757e4c5b7e5aabfdb5efc0316c1b9b9362fb48b35' }
      ]
    },
    ruby: {
      label: 'Ruby (Opal 1.8)', ukuran: '2,1 MB', ekst: ['rb'], cara: 'pekerja', alias: 'rb opal',
      ket: 'Ruby yang diterjemahkan ke JavaScript oleh Opal: bahasa intinya lengkap, sebagian pustaka standar tidak ada. Masukan diisi SEBELUM dijalankan.',
      berkas: [
        { nama: 'opal.min.js', url: 'https://cdn.opalrb.com/opal/1.8.2/opal.min.js', sha256: '285ffc304c21ec10e3dad4566878a5fedb0568266024f1dd196d95c870c341b7' },
        { nama: 'opal-parser.min.js', url: 'https://cdn.opalrb.com/opal/1.8.2/opal-parser.min.js', sha256: '152d395dba90d4caeff85296063364d152109a9aac142e3d97c4653e703a84ae' }
      ]
    },
    scheme: {
      label: 'Scheme (BiwaScheme 0.8)', ukuran: '0,4 MB', ekst: ['scm'], cara: 'pekerja', alias: 'lisp scm r6rs r7rs',
      ket: 'Penafsir Scheme (R6RS/R7RS kecil). Tanpa masukan.',
      berkas: [
        { nama: 'biwascheme-min.js', url: 'https://cdn.jsdelivr.net/npm/biwascheme@0.8.0/release/biwascheme-min.js', sha256: '123b156aa9fafb808ecfdba34ca175c7a98a64c84800e591e85ebc64a53cf73f' }
      ]
    },
    prolog: {
      label: 'Prolog (Tau 0.3)', ukuran: '0,4 MB', ekst: ['pl'], cara: 'pekerja', alias: 'pl tau logika',
      ket: 'Penafsir Prolog ISO. Yang dijalankan tujuan di `:- initialization(main).`. Tanpa masukan.',
      berkas: [
        { nama: 'core.js', url: 'https://cdn.jsdelivr.net/npm/tau-prolog@0.3.4/modules/core.js', sha256: '696c4117abffb1d11ac56f17727ee5790785ece21e8da66573449b016d8016a5' },
        { nama: 'lists.js', url: 'https://cdn.jsdelivr.net/npm/tau-prolog@0.3.4/modules/lists.js', sha256: '6268ac8c67b2f5c20134a8b6717e63db80aef17ba32e851107cbc8e5eb8adf42' }
      ]
    }
  };
  /* Bahasa yang DICARI orang tapi belum bisa dijalankan di sini, dengan sebabnya — supaya
     pencarian menjawab jujur alih-alih kosong. */
  /* `ubuntu`: nama paketnya di Ubuntu (versi Terminal: `pkg install <ubuntu>`). */
  var KOMPILATOR = 'Butuh kompilator native, yang di versi Play tidak boleh jalan. Pakai JRG IDE Terminal.';
  var BELUM = [
    { label: 'Go', alias: 'golang go', sebab: KOMPILATOR, ubuntu: 'golang' },
    { label: 'Rust', alias: 'rust rs cargo', sebab: KOMPILATOR, ubuntu: 'rust' },
    { label: 'C', alias: 'c gcc clang', sebab: KOMPILATOR, ubuntu: 'gcc' },
    { label: 'C++', alias: 'cpp c++ cxx g++', sebab: KOMPILATOR, ubuntu: 'g++' },
    { label: 'Java', alias: 'java jdk jvm', sebab: 'Butuh JDK (ratusan MB), belum dikemas.', ubuntu: 'java' },
    { label: 'Kotlin', alias: 'kotlin kt', sebab: 'Butuh JDK dan kotlinc, belum dikemas.', ubuntu: 'kotlin' },
    { label: 'C#', alias: 'csharp c# dotnet .net', sebab: 'Butuh runtime .NET, belum dikemas.', ubuntu: 'dotnet8' },
    { label: 'Swift', alias: 'swift ios', sebab: KOMPILATOR },
    { label: 'Dart', alias: 'dart flutter', sebab: KOMPILATOR },
    { label: 'PHP', alias: 'php laravel', sebab: 'Belum dikemas, masih antre.', ubuntu: 'php' },
    { label: 'Perl', alias: 'perl', sebab: 'Belum dikemas, masih antre.', ubuntu: 'perl' },
    { label: 'R', alias: 'r rstats statistik', sebab: 'webR puluhan MB, belum dikemas.', ubuntu: 'r-base' },
    { label: 'Haskell', alias: 'haskell hs ghc', sebab: KOMPILATOR, ubuntu: 'ghc' },
    { label: 'Zig', alias: 'zig', sebab: KOMPILATOR },
    { label: 'Assembly', alias: 'asm assembly nasm', sebab: KOMPILATOR, ubuntu: 'gcc' }
  ];
  var terpasang = {}, pekerja = {};

  function api(jalur, badan) {
    var o = badan === undefined ? {} : { method: 'POST', body: JSON.stringify(badan) };
    return fetch(jalur, o).then(function (r) {
      return r.json().then(function (d) { if (!r.ok) throw new Error(d.galat || ('pelayan menjawab ' + r.status)); return d; });
    });
  }
  function heksa(buf) {
    return Array.prototype.map.call(new Uint8Array(buf), function (b) { return (b < 16 ? '0' : '') + b.toString(16); }).join('');
  }
  function base64(u8) {
    var s = '';
    for (var i = 0; i < u8.length; i += 32768) s += String.fromCharCode.apply(null, u8.subarray(i, i + 32768));
    return btoa(s);
  }
  /* Potongan 384 KB (kelipatan 3, jadi base64-nya bisa disambung apa adanya). */
  function unggah(id, nama, u8, kabar) {
    var POTONG = 393216, rantai = Promise.resolve();
    for (var i = 0; i < u8.length || i === 0; i += POTONG) (function (dari) {
      rantai = rantai.then(function () {
        kabar(nama + ' ' + Math.round(100 * Math.min(dari + POTONG, u8.length) / (u8.length || 1)) + '%');
        return api('/api/paket/tulis', { paket: id, berkas: nama, awal: dari === 0, isi: base64(u8.subarray(dari, dari + POTONG)) });
      });
    })(i);
    return rantai;
  }

  var P = IDE.paket = {
    daftar: PAKET, belum: BELUM,
    /* Yang belum bisa, tanpa bahasa yang ternyata ada di perangkat ini (mis. C kalau clang terpasang). */
    /* Di versi Terminal sebabnya diganti cara memasangnya di Ubuntu. */
    belumDi: function (info) {
      var nyata = ((info && info.bahasa) || []).map(function (b) { return b.label; });
      var lx = info && info.linux && info.linux.bisa ? info.linux : null;
      return BELUM.filter(function (b) { return nyata.indexOf(b.label) < 0; }).map(function (b) {
        if (!lx || !b.ubuntu) return b;
        return { label: b.label, alias: b.alias, ubuntu: b.ubuntu,
          sebab: (lx.terpasang ? 'Pasang di Ubuntu: ' : 'Pasang Linux dulu (Lainnya > Linux), lalu ') + 'pkg install ' + b.ubuntu };
      });
    },
    untuk: function (ekst) { for (var k in PAKET) if (PAKET[k].ekst.indexOf(ekst) >= 0) return k; return null; },
    terpasang: function (id) { return !!terpasang[id]; },
    segarkan: function () { return api('/api/paket').then(function (d) { terpasang = d; return d; }); },
    /* `cermin`: alamat dasar pengganti CDN (uji, atau jaringan tertutup); sidik tetap diperiksa. */
    pasang: function (id, kabar, cermin) {
      var p = PAKET[id], rantai = Promise.resolve();
      p.berkas.forEach(function (f) {
        rantai = rantai.then(function () {
          kabar('Mengunduh ' + f.nama + '…');
          return fetch(cermin ? cermin + id + '/' + f.nama : f.url);
        }).then(function (r) {
          if (!r.ok) throw new Error(f.nama + ': server menjawab ' + r.status);
          return r.arrayBuffer();
        }, function () {
          throw new Error(f.nama + ' tidak bisa diunduh — memasang bahasa butuh internet (sekali saja)');
        }).then(function (buf) {
          return crypto.subtle.digest('SHA-256', buf).then(function (h) {
            if (heksa(h) !== f.sha256) throw new Error(f.nama + ': sidiknya tidak cocok, jadi tidak dipasang');
            return unggah(id, f.nama, new Uint8Array(buf), kabar);
          });
        });
      });
      return rantai.then(function () { return api('/api/paket/selesai', { paket: id }); }).then(P.segarkan);
    },
    copot: function (id) {
      if (pekerja[id]) { pekerja[id].terminate(); pekerja[id] = null; }
      return api('/api/paket/copot', { paket: id }).then(P.segarkan);
    },
    /* Python: jalan di pekerja yang DIPAKAI ULANG (memuat Pyodide makan beberapa detik);
       `hentikan` mematikan pekerjanya, jadi run berikutnya memuat lagi. */
    python: function (o) {
      var w = pekerja.python || (pekerja.python = new Worker('/web/js/pekerja_py.js'));
      var henti = null;
      var janji = new Promise(function (ok) {
        henti = function () { w.terminate(); pekerja.python = null; ok({ sukses: false, kode: -1, dihentikan: true }); };
        w.onmessage = function (e) {
          var d = e.data;
          if (d.jenis === 'selesai') ok({ sukses: d.kode === 0, kode: d.kode });
          else if (d.jenis === 'siap') o.kabar('');
          else if (d.jenis === 'kabar') o.kabar(d.teks);
          else o.tambah(d.jenis === 'galat' ? { galat: d.teks } : { keluaran: d.teks });
        };
        w.onerror = function (e) {
          o.tambah({ galat: 'Python tidak bisa dimuat: ' + (e.message || 'berkas paketnya rusak — copot lalu pasang lagi') + '\n' });
          henti();
        };
        o.kabar('Menyiapkan Python…');
        w.postMessage({ akar: location.origin + '/paket/python/', kode: o.kode, nama: o.nama, masukan: o.masukan, berkas: o.berkas, cermin: o.cermin ? location.origin + o.cermin : '' });
      });
      return { janji: janji, hentikan: henti };
    },
    /* Bahasa yang penjalannya pustaka JavaScript: pekerja BARU tiap run, jadi tidak ada sisa run lalu. */
    pekerja: function (id, o) {
      var w = new Worker('/web/js/pekerja_bahasa.js'), henti = null;
      var janji = new Promise(function (ok) {
        henti = function () { w.terminate(); ok({ sukses: false, kode: -1, dihentikan: true }); };
        w.onmessage = function (e) {
          var d = e.data;
          if (d.jenis === 'selesai') { w.terminate(); ok({ sukses: d.kode === 0, kode: d.kode }); }
          else o.tambah(d.jenis === 'galat' ? { galat: d.teks } : { keluaran: d.teks });
        };
        w.onerror = function (e) {
          o.tambah({ galat: PAKET[id].label + ' tidak bisa dimuat: ' + (e.message || 'berkas paketnya rusak — copot lalu pasang lagi') + '\n' });
          w.terminate(); ok({ sukses: false, kode: 1 });
        };
        w.postMessage({ bahasa: id, akar: location.origin + '/paket/' + id + '/', kode: o.kode, nama: o.nama, masukan: o.masukan });
      });
      return { janji: janji, hentikan: henti };
    },
    /* TypeScript, CoffeeScript: hasil kompilasinya (JavaScript) dikembalikan; yang menjalankan penjalan JS. */
    kompilasi: function (id, o) {
      return new Promise(function (ok, no) {
        var w = pekerja[id] || (pekerja[id] = new Worker('/web/js/pekerja_ts.js'));
        w.onmessage = function (e) { ok(e.data); };
        w.onerror = function (e) { pekerja[id] = null; no(new Error('kompilator ' + PAKET[id].label + ' tidak bisa dimuat: ' + (e.message || ''))); };
        w.postMessage({ bahasa: id, akar: location.origin + '/paket/' + id + '/', kode: o.kode, nama: o.nama });
      });
    }
  };
})(window.IDE);
