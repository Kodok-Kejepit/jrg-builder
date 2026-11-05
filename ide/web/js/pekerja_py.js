/* JRG IDE — js/pekerja_py.js: Python (Pyodide) di Web Worker. Di pekerja supaya program yang
   berputar selamanya bisa dihentikan dengan mematikan pekerjanya, dan halaman tidak membeku. */
var py = null, kunci = null;
var CDN = 'https://cdn.jsdelivr.net/pyodide/v0.28.3/full/';
function kirim(jenis, teks) { postMessage({ jenis: jenis, teks: teks }); }
function siap(akar) {
  if (py) return Promise.resolve();
  importScripts(akar + 'pyodide.js');
  return loadPyodide({ indexURL: akar }).then(function (p) { py = p; });
}
/* Bingkai milik Pyodide dibuang dari jejak galat: yang berguna cuma baris di berkas pemakai. */
function bersih(x, nama) {
  return String(x && x.message || x).replace(/^PythonError:\s*/, '').split('\n').filter(function (l) {
    return !/\/lib\/python\d+\.zip\//.test(l) && !/^\s*(await CodeRunner|\.\.\.<|\.run_async|coroutine = eval)/.test(l);
  }).join('\n').replace(/File "<exec>"/g, 'File "' + nama + '"');
}
function api(jalur, badan) {
  var o = badan === undefined ? {} : { method: 'POST', body: JSON.stringify(badan) };
  return fetch(jalur, o).then(function (r) { return r.json(); });
}
function heksa(buf) {
  return Array.prototype.map.call(new Uint8Array(buf), function (b) { return (b < 16 ? '0' : '') + b.toString(16); }).join('');
}
function base64(u8) {
  var s = '';
  for (var i = 0; i < u8.length; i += 32768) s += String.fromCharCode.apply(null, u8.subarray(i, i + 32768));
  return btoa(s);
}
function simpanPaket(nama, u8) {
  var POTONG = 393216, rantai = Promise.resolve();
  for (var i = 0; i < u8.length || i === 0; i += POTONG) (function (dari) {
    rantai = rantai.then(function () {
      return api('/api/paket/tulis', { paket: 'python', berkas: nama, awal: dari === 0, isi: base64(u8.subarray(dari, dari + POTONG)) });
    });
  })(i);
  return rantai;
}
/* Satu pustaka dari daftar resmi Pyodide: kalau belum ada di perangkat, diunduh, sidiknya
   dicocokkan dengan daftar itu (yang sidiknya sendiri diperiksa saat Python dipasang), lalu disimpan. */
function ambilPustaka(p, d) {
  return api('/api/paket/ada?paket=python&berkas=' + encodeURIComponent(p.file_name)).then(function (j) {
    if (j.ada) return;
    kirim('kabar', 'Mengunduh pustaka ' + p.name + '…');
    return fetch((d.cermin ? d.cermin + 'python/' : CDN) + p.file_name).then(function (r) {
      if (!r.ok) throw new Error('server menjawab ' + r.status);
      return r.arrayBuffer();
    }).then(function (buf) {
      return crypto.subtle.digest('SHA-256', buf).then(function (h) {
        if (heksa(h) !== p.sha256) throw new Error('sidiknya tidak cocok, jadi tidak dipasang');
        return simpanPaket(p.file_name, new Uint8Array(buf));
      });
    }).catch(function (x) {
      throw new Error('pustaka `' + p.name + '` belum ada di perangkat dan tidak bisa diunduh (' + (x.message || x) + ') — butuh internet sekali');
    });
  });
}
/* Modul yang diimpor kode pemakai tapi belum ada: dari daftar Pyodide dipasang ke perangkat
   (lalu jalan tanpa internet); sisanya dicoba lewat PyPI, yang cuma bertahan selama sesi ini. */
function pasangImpor(d) {
  var sumber = [d.kode].concat((d.berkas || []).filter(function (f) { return /\.py$/.test(f.nama); }).map(function (f) { return f.isi; }));
  var cari = py.runPython('import importlib.util as _u\nfrom pyodide.code import find_imports as _f\n' +
    'def _hilang(sumber):\n    nama = set()\n    for s in sumber:\n        try: nama.update(m.split(".")[0] for m in _f(s))\n        except SyntaxError: pass\n' +
    '    return [n for n in sorted(nama) if _u.find_spec(n) is None]\n_hilang');
  var hilang = cari(py.toPy(sumber)).toJs();
  if (!hilang.length) return Promise.resolve();
  var janjiKunci = kunci ? Promise.resolve(kunci) : fetch(d.akar + 'pyodide-lock.json').then(function (r) { return r.json(); }).then(function (k) { kunci = k.packages; return kunci; });
  return janjiKunci.then(function (paket) {
    var dariImpor = {}, perlu = {}, asing = [];
    Object.keys(paket).forEach(function (k) { (paket[k].imports || []).forEach(function (m) { dariImpor[m] = k; }); });
    function tambah(k) { if (perlu[k] || !paket[k]) return; perlu[k] = paket[k]; (paket[k].depends || []).forEach(tambah); }
    hilang.forEach(function (m) { if (dariImpor[m]) tambah(dariImpor[m]); else asing.push(m); });
    if (asing.length) tambah('micropip');
    var rantai = Promise.resolve();
    Object.keys(perlu).forEach(function (k) { rantai = rantai.then(function () { return ambilPustaka(perlu[k], d); }); });
    return rantai.then(function () {
      var nama = Object.keys(perlu);
      if (!nama.length) return;
      kirim('kabar', 'Memuat ' + nama.filter(function (n) { return n !== 'micropip'; }).join(', ') + '…');
      return py.loadPackage(nama, { messageCallback: function () {}, errorCallback: function () {} });
    }).then(function () {
      if (!asing.length) return;
      kirim('kabar', 'Mencari ' + asing.join(', ') + ' di PyPI…');
      var mp = py.pyimport('micropip'), r = Promise.resolve();
      /* Yang gagal dibiarkan: `import`-nya sendiri yang akan melapor, dengan baris yang benar. */
      asing.forEach(function (m) { r = r.then(function () { return mp.install(m).catch(function () {}); }); });
      return r;
    });
  });
}
onmessage = function (e) {
  var d = e.data;
  siap(d.akar).then(function () {
    var baris = d.masukan ? d.masukan.replace(/\n$/, '').split('\n') : [], i = 0;
    py.setStdin({ stdin: function () { return i < baris.length ? baris[i++] : null; } });
    py.setStdout({ batched: function (t) { kirim('keluaran', t + '\n'); } });
    py.setStderr({ batched: function (t) { kirim('galat', t + '\n'); } });
    /* Berkas proyek ditaruh di sistem berkas Python, jadi `import bantu` dan `open("data.txt")` jalan. */
    py.runPython('import sys, os, shutil\nos.chdir("/")\nshutil.rmtree("/proyek", ignore_errors=True)\nos.makedirs("/proyek", exist_ok=True)\nos.chdir("/proyek")\n' +
      'sys.path[:] = ["/proyek"] + [p for p in sys.path if p != "/proyek"]\n' +
      'for _m in [k for k, v in sys.modules.items() if getattr(v, "__file__", None) and str(v.__file__).startswith("/proyek/")]: del sys.modules[_m]');
    (d.berkas || []).forEach(function (f) {
      var potong = f.nama.split('/'); potong.pop();
      if (potong.length) py.FS.mkdirTree('/proyek/' + potong.join('/'));
      py.FS.writeFile('/proyek/' + f.nama, f.b64 != null ? Uint8Array.from(atob(f.b64), function (c) { return c.charCodeAt(0); }) : f.isi);
    });
    return pasangImpor(d);
  }).then(function () {
    kirim('siap', '');
    return py.runPythonAsync(d.kode, { globals: py.toPy({ __name__: '__main__' }) });
  }).then(function () { postMessage({ jenis: 'selesai', kode: 0 }); }, function (x) {
    kirim('galat', bersih(x, d.nama) + '\n');
    postMessage({ jenis: 'selesai', kode: 1 });
  });
};
