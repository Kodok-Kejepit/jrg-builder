/* JRG IDE — js/saran.js
   Pelengkapan otomatis di penyunting, dan simbol berkas (fn, struct, kelas…) untuk panel Simbol.
   Sumber saran: nama bawaan JRG dari `jrgc --daftar-builtin` di perangkat itu sendiri, kata kunci
   bahasanya (dari penyorot), dan nama yang sudah ditulis di berkas yang sama. */
(function (IDE) {
  'use strict';
  var U = IDE.util, esc = U.esc;

  /* ---------- simbol ---------- */
  /* Satu pola per bahasa; tiap temuan {nama, jenis, baris, dalam}. `dalam` = indentasi (method di dalam impl/kelas). */
  var POLA = {
    jrg: /^(\s*)(?:pub\s+)?(fn|struct|enum|trait|impl|const|type|extern\s+fn)\s+([A-Za-z_][\w]*(?:\s+for\s+[A-Za-z_][\w:]*)?)/,
    rust: /^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(fn|struct|enum|trait|impl|const|type|mod)\s+([A-Za-z_][\w]*(?:\s+for\s+[A-Za-z_][\w:]*)?)/,
    python: /^(\s*)(?:async\s+)?(def|class)\s+([A-Za-z_]\w*)/,
    js: /^(\s*)(?:export\s+)?(?:default\s+)?(?:async\s+)?(function\*?|class|const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?=[=(<{]|extends|$)/,
    lua: /^(\s*)(?:local\s+)?(function)\s+([A-Za-z_][\w.:]*)/,
    ruby: /^(\s*)(def|class|module)\s+([A-Za-z_][\w.?!]*)/,
    sh: /^(\s*)(?:(function)\s+)?([A-Za-z_][\w-]*)\s*\(\)\s*\{?/,
    go: /^(\s*)(func|type)\s+(?:\([^)]*\)\s*)?([A-Za-z_]\w*)/,
    c: /^(\s*)(?:static\s+|inline\s+|extern\s+)*(struct|enum|class|[A-Za-z_][\w<>:*\s]*?)\s+\**([A-Za-z_]\w*)\s*\([^;]*$/,
    java: /^(\s*)(?:(?:public|private|protected|static|final|abstract|open|override|data|sealed|internal|suspend)\s+)*(class|interface|enum|fun|object|record)\s+([A-Za-z_]\w*)/,
    sql: /^(\s*)(create\s+(?:table|view|index))\s+(?:if\s+not\s+exists\s+)?([A-Za-z_][\w.]*)/i,
    md: /^()(#{1,6})\s+(.+)$/
  };
  POLA.ts = POLA.js; POLA.kotlin = POLA.java; POLA.cs = POLA.java;
  /* `const x = 5` di JS bukan simbol yang layak didaftar; yang dicari fungsi panah dan kelas. */
  function layakJs(baris, jenis) {
    return jenis !== 'const' && jenis !== 'let' && jenis !== 'var' || /=\s*(?:async\s*)?(?:function|\([^)]*\)\s*=>|[A-Za-z_$][\w$]*\s*=>|class)/.test(baris);
  }
  function simbol(teks, lang) {
    var pola = POLA[lang], hasil = [];
    if (!pola) return hasil;
    var baris = teks.split('\n'), dalamKomentar = false;
    for (var i = 0; i < baris.length && hasil.length < 2000; i++) {
      var l = baris[i];
      if (lang !== 'md' && lang !== 'python' && lang !== 'sh' && lang !== 'ruby') {
        if (dalamKomentar) { if (l.indexOf('*/') >= 0) dalamKomentar = false; continue; }
        if (/^\s*\/\*/.test(l) && l.indexOf('*/') < 0) { dalamKomentar = true; continue; }
      }
      var m = pola.exec(l);
      if (!m) continue;
      var jenis = m[2].replace(/\s+/g, ' ');
      if ((lang === 'js' || lang === 'ts') && !layakJs(l, jenis)) continue;
      if (lang === 'c' && /^(return|else|if|while|for|switch)$/.test(m[3])) continue;
      if (lang === 'sh' && !jenis && !/\(\)/.test(l)) continue;
      hasil.push({ nama: m[3].trim(), jenis: lang === 'md' ? 'h' + jenis.length : (jenis || 'function'), baris: i + 1, dalam: lang === 'md' ? jenis.length - 1 : Math.floor(m[1].replace(/\t/g, '    ').length / 2) });
    }
    return hasil;
  }

  /* ---------- cuplikan ---------- */
  /* Kerangka yang sering ditulis; `$0` tempat kursor sesudahnya. Baris kedua dan seterusnya ikut
     indentasi baris tempat cuplikannya ditulis. */
  var CUPLIKAN = {
    jrg: {
      main: ['fungsi main', 'pub fn main() {\n    $0\n}'],
      fn: ['fungsi', 'fn $0() {\n}'],
      pfn: ['fungsi publik', 'pub fn $0() {\n}'],
      struct: ['struct', 'struct $0 {\n}'],
      enum: ['enum', 'enum $0 {\n}'],
      impl: ['impl', 'impl $0 {\n}'],
      trait: ['trait', 'trait $0 {\n}'],
      for: ['for indeks dan nilai', 'for i, x in $0 {\n}'],
      fori: ['for rentang', 'for i in 0..$0 {\n}'],
      while: ['while', 'while $0 {\n}'],
      if: ['if', 'if $0 {\n}'],
      ife: ['if-else', 'if $0 {\n} else {\n}'],
      match: ['match', 'match $0 {\n    _ => {}\n}'],
      matcho: ['match Option', 'match $0 {\n    Some(x) => {}\n    None => {}\n}'],
      matchr: ['match Result', 'match $0 {\n    Ok(v) => {}\n    Err(e) => {}\n}'],
      test: ['fungsi uji', 'fn test_$0() {\n    assert_eq(1, 1)\n}'],
      spawn: ['goroutine', 'spawn {\n    $0\n}'],
      wokf: ['cetak teks berformat', 'wok(str::format("{}", $0))']
    },
    python: {
      def: ['fungsi', 'def $0():\n    pass'],
      class: ['kelas', 'class $0:\n    def __init__(self):\n        pass'],
      for: ['for', 'for x in $0:\n    pass'],
      main: ['blok utama', 'if __name__ == "__main__":\n    $0']
    },
    js: {
      fn: ['fungsi', 'function $0() {\n}'],
      for: ['for of', 'for (const x of $0) {\n}'],
      log: ['console.log', 'console.log($0);']
    }
  };
  CUPLIKAN.ts = CUPLIKAN.js;

  /* ---------- pelengkapan ---------- */
  var bawaan = null;          /* ['str::split', ...] dari mesin di perangkat */
  function muatBawaan(api) {
    if (bawaan) return Promise.resolve(bawaan);
    return api('/api/kamus?kata=').then(function (d) {
      bawaan = d.teks.split('\n').map(function (l) { return l.trim(); }).filter(function (l) { return /^[A-Za-z_][\w:]*$/.test(l); });
      return bawaan;
    }, function () { bawaan = []; return bawaan; });
  }

  /* Nilai kecocokan: awalan menang, lalu awalan sesudah `::`, lalu huruf-hurufnya berurutan. */
  function nilai(calon, q) {
    var c = calon.toLowerCase(), k = q.toLowerCase();
    if (c === k) return -1;
    if (c.indexOf(k) === 0) return 1000 - c.length;
    var ekor = c.lastIndexOf('::');
    if (ekor >= 0 && c.slice(ekor + 2).indexOf(k) === 0) return 600 - c.length;
    if (k.length >= 3 && c.indexOf(k) > 0) return 400 - c.length;
    var j = 0;
    for (var i = 0; i < c.length && j < k.length; i++) if (c[i] === k[j]) j++;
    return j === k.length && k.length >= 2 && c[0] === k[0] ? 200 - c.length : -1;
  }

  function Saran(ed, opsi) {
    var self = this;
    this.ed = ed; this.opsi = opsi;
    this.el = document.createElement('div');
    this.el.className = 'saran';
    this.el.setAttribute('role', 'listbox');
    this.el.hidden = true;
    ed.root.appendChild(this.el);
    this.daftar = []; this.pilih = 0; this.awal = null;
    var ta = ed.ta;
    /* Fase tangkap: tombol yang dipakai daftar saran tidak boleh sampai ke penyunting. */
    ta.addEventListener('keydown', function (e) {
      if (self.el.hidden) {
        if ((e.ctrlKey || e.metaKey) && e.key === ' ') { e.preventDefault(); self.buka(true); }
        return;
      }
      var k = e.key;
      if (k === 'ArrowDown' || k === 'ArrowUp') { e.preventDefault(); e.stopImmediatePropagation(); self.geser(k === 'ArrowDown' ? 1 : -1); }
      else if (k === 'Enter' || k === 'Tab') { e.preventDefault(); e.stopImmediatePropagation(); self.terima(self.pilih); }
      else if (k === 'Escape') { e.preventDefault(); e.stopImmediatePropagation(); self.tutup(); }
    }, true);
    ta.addEventListener('input', function (e) {
      if (self.menerima) return;
      if (!opsi.aktif()) return self.tutup();
      var t = e.inputType || '';
      if (t === 'insertText' && e.data && /[\w:.$]/.test(e.data.slice(-1))) self.buka(false);
      else if (t.indexOf('delete') === 0 && !self.el.hidden) self.buka(false);
      else self.tutup();
    });
    ta.addEventListener('blur', function () { setTimeout(function () { if (document.activeElement !== ta) self.tutup(); }, 150); });
    ta.addEventListener('click', function () { self.tutup(); });
    ed.scroll.addEventListener('scroll', function () { if (!self.el.hidden) self.letak(); }, { passive: true });
    /* pointerdown + preventDefault: fokus (dan papan ketik HP) tetap di penyunting. */
    this.el.addEventListener('pointerdown', function (e) { e.preventDefault(); });
    this.el.addEventListener('click', function (e) {
      var b = e.target.closest('[data-i]');
      if (b) self.terima(+b.getAttribute('data-i'));
    });
  }

  Saran.prototype.calon = function (q) {
    var lang = this.ed.lang, g = IDE.hl.langs[lang] || {}, lihat = {}, semua = [];
    function tambah(nama, jenis) { if (!lihat[nama]) { lihat[nama] = 1; semua.push({ nama: nama, jenis: jenis }); } }
    if (lang === 'jrg' && bawaan) bawaan.forEach(function (n) { tambah(n, 'bawaan'); });
    [g.kw, g.val, g.ty].forEach(function (s, i) { if (s) Object.keys(s).forEach(function (n) { tambah(n, ['kata', 'nilai', 'tipe'][i]); }); });
    /* Nama di berkas ini, dikumpulkan paling sering sekali per 2 detik (berkas besar: belasan ms per
       kumpulan). Kata yang sedang diketik sendiri tidak ditawarkan. */
    var kini = Date.now();
    if (!this.nama || kini - this.nama.t > 2000 || this.nama.lang !== lang || this.nama.isi !== this.ed.nomorIsi) {
      var teks = this.ed.getValue(), p = this.ed.sel()[0], re = /[A-Za-z_$][\w$]*(?:::[A-Za-z_]\w*)*/g, m, n = 0, ada = {}, daftar = [];
      while ((m = re.exec(teks)) && n++ < 40000) {
        if (m.index <= p && m.index + m[0].length >= p) continue;
        if (m[0].length > 2 && !ada[m[0]]) { ada[m[0]] = 1; daftar.push(m[0]); }
      }
      this.nama = { t: kini, lang: lang, isi: this.ed.nomorIsi, daftar: daftar };
    }
    this.nama.daftar.forEach(function (nm) { if (nm !== q) tambah(nm, 'nama'); });
    var hasil = semua.map(function (c) { c.n = nilai(c.nama, q); return c; }).filter(function (c) { return c.n >= 0; })
      .sort(function (a, b) { return b.n - a.n || (a.jenis === 'nama' ? -1 : 0) - (b.jenis === 'nama' ? -1 : 0) || (a.nama < b.nama ? -1 : 1); }).slice(0, 40);
    /* Cuplikan yang namanya diawali kata itu (persis pun boleh) naik ke atas. */
    var cup = CUPLIKAN[lang] || {}, ql = q.toLowerCase();
    var depan = q.indexOf('::') < 0 ? Object.keys(cup).filter(function (k) { return k.indexOf(ql) === 0; }).map(function (k) {
      return { nama: k, jenis: 'cuplikan', ket: cup[k][0], isi: cup[k][1] };
    }) : [];
    return depan.concat(hasil).slice(0, 40);
  };

  Saran.prototype.buka = function (paksa) {
    var self = this, pre = this.ed.prefix();
    if (!pre || (!paksa && pre.teks.replace(/:/g, '').length < 2 && !/::$/.test(pre.teks))) return this.tutup();
    if (this.ed.lang === 'jrg' && !bawaan) { muatBawaan(this.opsi.api).then(function () { if (self.ed.prefix()) self.buka(paksa); }); }
    var d = this.calon(pre.teks);
    if (!d.length) return this.tutup();
    this.daftar = d; this.awal = pre; this.pilih = 0;
    this.el.innerHTML = d.map(function (c, i) {
      return '<div class="sr' + (c.isi ? ' cup' : '') + '" role="option" data-i="' + i + '" aria-selected="' + (i === 0) + '"><b>' + esc(c.nama) + '</b><span>' + esc(c.ket || c.jenis) + '</span></div>';
    }).join('');
    this.el.hidden = false;
    this.letak();
  };

  /* Di bawah kursor; naik ke atasnya kalau ruang bawahnya tidak cukup. */
  Saran.prototype.letak = function () {
    var ed = this.ed, p = ed.pos(), sc = ed.scroll;
    var baris = ed.ta.value.slice(0, p.lokal).split('\n').pop();
    var gw = ed.gutter.offsetWidth;
    var x = ed.wrap ? gw + 12 : gw + 12 + baris.length * ed.charW() - sc.scrollLeft;
    var y = ed.barisTop(p.baris) - sc.scrollTop;
    var tinggi = ed.root.clientHeight, lebar = ed.root.clientWidth;
    var t = Math.min(this.el.scrollHeight, 220);
    this.el.style.left = Math.max(4, Math.min(x - 8, lebar - this.el.offsetWidth - 6)) + 'px';
    this.el.style.top = (y + ed.lh + t + 4 > tinggi && y > t ? y - t - 2 : y + ed.lh + 2) + 'px';
  };

  Saran.prototype.geser = function (d) {
    var n = this.daftar.length;
    this.pilih = (this.pilih + d + n) % n;
    var self = this;
    U.$$('.sr', this.el).forEach(function (b, i) { b.setAttribute('aria-selected', String(i === self.pilih)); });
    var b = this.el.children[this.pilih];
    if (b) b.scrollIntoView({ block: 'nearest' });
  };

  Saran.prototype.terima = function (i) {
    var c = this.daftar[i], pre = this.awal, ed = this.ed;
    this.tutup();
    if (!c || !pre) return;
    var p = ed.ta.selectionStart;
    if (c.isi) return this.cuplikan(c, pre, p);
    var sesudah = ed.ta.value.slice(p, p + 1);
    /* Nama bawaan adalah fungsi: kurungnya ikut, kursor di dalamnya, dan tanda tangannya ditampilkan. */
    var fungsi = c.jenis === 'bawaan' && sesudah !== '(' && !/^[A-Z]\w*$/.test(c.nama);
    this.menerima = true;
    try { ed.replace(pre.dari, p, c.nama + (fungsi ? '()' : ''), pre.dari + c.nama.length + (fungsi ? 1 : 0)); }
    finally { this.menerima = false; }
    ed.revealCaret();
    if (c.jenis === 'bawaan' && this.opsi.tanda) this.opsi.tanda(c.nama);
  };

  Saran.prototype.cuplikan = function (c, pre, p) {
    var ed = this.ed, v = ed.ta.value, awal = v.lastIndexOf('\n', pre.dari - 1) + 1;
    var ind = /^[ \t]*/.exec(v.slice(awal, pre.dari))[0];
    var teks = c.isi.replace(/\n/g, '\n' + ind), di = teks.indexOf('$0');
    teks = teks.replace('$0', '');
    this.menerima = true;
    try { ed.replace(pre.dari, p, teks, pre.dari + (di < 0 ? teks.length : di)); }
    finally { this.menerima = false; }
    ed.revealCaret();
  };

  Saran.prototype.tutup = function () {
    if (this.el.hidden) return;
    this.el.hidden = true;
    this.daftar = []; this.awal = null;
  };

  IDE.saran = { Saran: Saran, simbol: simbol, muatBawaan: muatBawaan, CUPLIKAN: CUPLIKAN };
})(window.IDE);
