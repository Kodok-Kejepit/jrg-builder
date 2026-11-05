/* JRG IDE — js/vt.js: emulator terminal (xterm/VT220 secukupnya) yang menggambar ke DOM.
   Model layar (sel per baris), pengurai urutan escape, dan penggambar baris yang kotor saja per
   bingkai. Penjelasannya: ide/README.md, "Terminal". */
(function (IDE) {
  'use strict';

  /* Lebar tampil satu kode Unicode: 0 (penggabung), 2 (lebar/emoji), 1 sisanya. */
  var LEBAR2 = [0x1100, 0x115F, 0x231A, 0x231B, 0x2329, 0x232A, 0x23E9, 0x23EC, 0x23F0, 0x23F0, 0x23F3, 0x23F3,
    0x25FD, 0x25FE, 0x2614, 0x2615, 0x2648, 0x2653, 0x267F, 0x267F, 0x2693, 0x2693, 0x26A1, 0x26A1, 0x26AA, 0x26AB,
    0x26BD, 0x26BE, 0x26C4, 0x26C5, 0x26CE, 0x26CE, 0x26D4, 0x26D4, 0x26EA, 0x26EA, 0x26F2, 0x26F3, 0x26F5, 0x26F5,
    0x26FA, 0x26FA, 0x26FD, 0x26FD, 0x2705, 0x2705, 0x270A, 0x270B, 0x2728, 0x2728, 0x274C, 0x274C, 0x274E, 0x274E,
    0x2753, 0x2755, 0x2757, 0x2757, 0x2795, 0x2797, 0x27B0, 0x27B0, 0x27BF, 0x27BF, 0x2B1B, 0x2B1C, 0x2B50, 0x2B50,
    0x2B55, 0x2B55, 0x2E80, 0x303E, 0x3041, 0x33FF, 0x3400, 0x4DBF, 0x4E00, 0x9FFF, 0xA000, 0xA4CF, 0xA960, 0xA97F,
    0xAC00, 0xD7A3, 0xF900, 0xFAFF, 0xFE10, 0xFE19, 0xFE30, 0xFE6F, 0xFF00, 0xFF60, 0xFFE0, 0xFFE6,
    0x16FE0, 0x16FE4, 0x17000, 0x18CFF, 0x1B000, 0x1B2FF, 0x1F004, 0x1F004, 0x1F0CF, 0x1F0CF, 0x1F18E, 0x1F18E,
    0x1F191, 0x1F19A, 0x1F200, 0x1F251, 0x1F300, 0x1F320, 0x1F32D, 0x1F335, 0x1F337, 0x1F37C, 0x1F37E, 0x1F393,
    0x1F3A0, 0x1F3CA, 0x1F3CF, 0x1F3D3, 0x1F3E0, 0x1F3F0, 0x1F3F4, 0x1F3F4, 0x1F3F8, 0x1F43E, 0x1F440, 0x1F440,
    0x1F442, 0x1F4FC, 0x1F4FF, 0x1F53D, 0x1F54B, 0x1F54E, 0x1F550, 0x1F567, 0x1F57A, 0x1F57A, 0x1F595, 0x1F596,
    0x1F5A4, 0x1F5A4, 0x1F5FB, 0x1F64F, 0x1F680, 0x1F6C5, 0x1F6CC, 0x1F6CC, 0x1F6D0, 0x1F6D2, 0x1F6D5, 0x1F6D7,
    0x1F6EB, 0x1F6EC, 0x1F6F4, 0x1F6FC, 0x1F7E0, 0x1F7EB, 0x1F90C, 0x1F93A, 0x1F93C, 0x1F945, 0x1F947, 0x1F9FF,
    0x1FA70, 0x1FAFF, 0x20000, 0x2FFFD, 0x30000, 0x3FFFD];
  var NOL = [0x0300, 0x036F, 0x0483, 0x0489, 0x0591, 0x05BD, 0x0610, 0x061A, 0x064B, 0x065F, 0x0670, 0x0670,
    0x06D6, 0x06DC, 0x06DF, 0x06E4, 0x0900, 0x0902, 0x093A, 0x093A, 0x093C, 0x093C, 0x0941, 0x0948, 0x094D, 0x094D,
    0x0E31, 0x0E31, 0x0E34, 0x0E3A, 0x0E47, 0x0E4E, 0x1AB0, 0x1AFF, 0x1DC0, 0x1DFF, 0x200B, 0x200F, 0x2028, 0x202E,
    0x2060, 0x2064, 0x20D0, 0x20FF, 0xFE00, 0xFE0F, 0xFE20, 0xFE2F, 0xFEFF, 0xFEFF, 0x1F3FB, 0x1F3FF, 0xE0100, 0xE01EF];
  function dalam(t, c) {
    var lo = 0, hi = t.length / 2 - 1;
    while (lo <= hi) {
      var m = (lo + hi) >> 1;
      if (c < t[m * 2]) hi = m - 1; else if (c > t[m * 2 + 1]) lo = m + 1; else return true;
    }
    return false;
  }
  function lebar(c) {
    if (c < 0x300) return 1;
    if (dalam(NOL, c)) return 0;
    return dalam(LEBAR2, c) ? 2 : 1;
  }

  /* Grafik garis DEC (ESC ( 0): dipakai mc, dialog, tmux untuk bingkai. */
  var GARIS = { '`': '◆', a: '▒', f: '°', g: '±', j: '┘', k: '┐', l: '┌', m: '└', n: '┼', o: '⎺', p: '⎻', q: '─',
    r: '⎼', s: '⎽', t: '├', u: '┤', v: '┴', w: '┬', x: '│', y: '≤', z: '≥', '{': 'π', '|': '≠', '}': '£', '~': '·' };

  var B_TEBAL = 1, B_REDUP = 2, B_MIRING = 4, B_GARIS = 8, B_KEDIP = 16, B_BALIK = 32, B_SEMBUNYI = 64, B_CORET = 128;
  var BAWAAN = -1;

  /* Atribut disimpan sebagai nomor kecil: (fg, bg, bendera) yang sama memakai nomor yang sama. */
  function Atribut() { this.daftar = [[BAWAAN, BAWAAN, 0]]; this.peta = { '-1,-1,0': 0 }; this.tag = ['']; }
  Atribut.prototype.nomor = function (fg, bg, fl) {
    var k = fg + ',' + bg + ',' + fl, n = this.peta[k];
    if (n !== undefined) return n;
    if (this.daftar.length > 4000) { this.daftar.length = 1; this.tag.length = 1; this.peta = { '-1,-1,0': 0 }; }
    n = this.daftar.length;
    this.daftar.push([fg, bg, fl]);
    this.peta[k] = n;
    this.tag[n] = null;
    return n;
  };
  function rgb256(n) {
    if (n < 232) {
      n -= 16;
      var v = [0, 95, 135, 175, 215, 255];
      return 'rgb(' + v[Math.floor(n / 36)] + ',' + v[Math.floor(n / 6) % 6] + ',' + v[n % 6] + ')';
    }
    var g = 8 + (n - 232) * 10;
    return 'rgb(' + g + ',' + g + ',' + g + ')';
  }
  function warna(v, latar) {
    if (v === BAWAAN) return null;
    if (v < 16) return { k: (latar ? 'vb' : 'vf') + v };
    if (v < 256) return { s: rgb256(v) };
    var r = v - 0x1000000;
    return { s: 'rgb(' + ((r >> 16) & 255) + ',' + ((r >> 8) & 255) + ',' + (r & 255) + ')' };
  }
  /* Kelas dan gaya CSS satu nomor atribut, dibuat sekali lalu disimpan. */
  Atribut.prototype.css = function (n) {
    var t = this.tag[n];
    if (t != null) return t;
    var a = this.daftar[n], fl = a[2], fg = warna(a[0], false), bg = warna(a[1], true), kelas = [], gaya = '';
    if (fl & B_BALIK) {
      var x = fg; fg = bg; bg = x;
      if (fg && fg.k) fg = { k: 'vf' + fg.k.slice(2) };
      if (bg && bg.k) bg = { k: 'vb' + bg.k.slice(2) };
      if (!fg) kelas.push('vfb');
      if (!bg) kelas.push('vbf');
    }
    if (fg) { if (fg.k) kelas.push(fg.k); else gaya += 'color:' + fg.s + ';'; }
    if (bg) { if (bg.k) kelas.push(bg.k); else gaya += 'background:' + bg.s + ';'; }
    if (fl & B_TEBAL) kelas.push('vt');
    if (fl & B_REDUP) kelas.push('vr');
    if (fl & B_MIRING) kelas.push('vm');
    if (fl & B_GARIS) kelas.push('vg');
    if (fl & B_CORET) kelas.push('vc');
    if (fl & B_KEDIP) kelas.push('vk');
    if (fl & B_SEMBUNYI) kelas.push('vs');
    t = { k: kelas.join(' '), g: gaya };
    t.buka = t.k || t.g ? '<span' + (t.k ? ' class="' + t.k + '"' : '') + (t.g ? ' style="' + t.g + '"' : '') + '>' : '';
    this.tag[n] = t;
    return t;
  };

  function Baris(kolom, a) {
    this.c = new Array(kolom).fill(' ');
    this.a = new Array(kolom).fill(a);
    this.w = false;
  }
  Baris.prototype.teks = function () {
    var s = '';
    for (var i = 0; i < this.c.length; i++) s += this.c[i];
    return s.replace(/\s+$/, '');
  };

  function esc1(ch) { return ch === '<' ? '&lt;' : ch === '&' ? '&amp;' : ch === '>' ? '&gt;' : ch; }

  /* Satu layar terminal. `o`: { kirim(teks), judul(t), osc(kode, isi), bel(), ubahUkuran() }. */
  function Vt(akar, o) {
    this.o = o;
    this.akar = akar;
    this.attr = new Atribut();
    this.maksSb = o.scrollback || 3000;
    this.kolom = 80; this.baris = 24;
    akar.innerHTML = '<div class="t-sb"></div><div class="t-scr"></div><span class="t-ukur">MMMMMMMMMM</span>';
    this.elSb = akar.firstChild; this.elScr = this.elSb.nextSibling; this.elUkur = this.elScr.nextSibling;
    this.sb = []; this.sbBaru = 0; this.sbHapus = false;
    this.reset(true);
  }

  Vt.prototype.reset = function (awal) {
    this.utama = this.layarBaru(); this.alt = null; this.buf = this.utama;
    this.x = 0; this.y = 0; this.tunda = false;
    this.fg = BAWAAN; this.bg = BAWAAN; this.fl = 0; this.aKini = 0;
    this.atas = 0; this.bawah = this.baris - 1;
    this.mode = { awBungkus: true, sisip: false, asal: false, kursor: true, kursorApl: false, keypadApl: false, tempel: false,
      tetikus: 0, sgr: false, fokus: false, lnm: false, balik: false };
    this.g = ['B', 'B']; this.gl = 0;
    this.simpan = null; this.simpanAlt = null;
    this.tab = {}; for (var i = 8; i < 500; i += 8) this.tab[i] = true;
    this.keadaan = 0; this.param = ''; this.antara = ''; this.oscIsi = '';
    this.gayaKursor = 0;
    if (!awal) { this.sb = []; this.sbHapus = true; }
    this.kotorSemua();
  };
  Vt.prototype.layarBaru = function () {
    var l = [];
    for (var i = 0; i < this.baris; i++) l.push(new Baris(this.kolom, 0));
    return l;
  };
  Vt.prototype.kotorSemua = function () { this.kotor = {}; this.semua = true; this.minta(); };
  Vt.prototype.tandai = function (y) { if (!this.semua) this.kotor[y] = true; if (!this.jadwal) this.minta(); };
  Vt.prototype.minta = function () {
    if (this.jadwal) return;
    var self = this;
    this.jadwal = (window.requestAnimationFrame || setTimeout)(function () { self.jadwal = 0; self.gambar(); });
  };

  /* Ukuran sel dari huruf yang sedang dipakai; jawabannya {kolom, baris}. */
  Vt.prototype.ukurSel = function () {
    var r = this.elUkur.getBoundingClientRect();
    this.lebarSel = r.width / 10 || 8;
    this.tinggiSel = parseFloat(getComputedStyle(this.akar).lineHeight) || 17;
    var w = this.akar.clientWidth - 8, h = this.akar.clientHeight - 4;
    return { kolom: Math.max(10, Math.floor(w / this.lebarSel)), baris: Math.max(4, Math.floor(h / this.tinggiSel)) };
  };

  Vt.prototype.ubahUkuran = function (kolom, baris) {
    if (kolom === this.kolom && baris === this.baris) return false;
    var self = this;
    [this.utama, this.alt].forEach(function (buf) {
      if (!buf) return;
      for (var i = 0; i < buf.length; i++) {
        var b = buf[i];
        while (b.c.length < kolom) { b.c.push(' '); b.a.push(0); }
        if (b.c.length > kolom) { b.c.length = kolom; b.a.length = kolom; if (b.c[kolom - 1] === '' ) b.c[kolom - 1] = ' '; }
      }
      /* Layar memendek: baris teratas (di atas kursor) pindah ke scrollback, seperti xterm. */
      while (buf.length > baris) {
        if (buf === self.utama && self.y > 0 && self.buf === self.utama) { self.dorongSb(buf.shift()); self.y--; }
        else buf.pop();
      }
      while (buf.length < baris) buf.push(new Baris(kolom, 0));
    });
    this.kolom = kolom; this.baris = baris;
    this.atas = 0; this.bawah = baris - 1;
    this.x = Math.min(this.x, kolom - 1); this.y = Math.min(this.y, baris - 1); this.tunda = false;
    this.kotorSemua();
    return true;
  };

  Vt.prototype.dorongSb = function (b) {
    this.sb.push(b);
    this.sbBaru++;
    /* Dipotong per 512 baris, bukan per baris: splice tiap baris membuat banjir keluaran kuadratik. */
    if (this.sb.length > this.maksSb + 512) this.sb.splice(0, this.sb.length - this.maksSb);
  };

  /* ---------- operasi layar ---------- */
  Vt.prototype.kosong = function () { return this.bg === BAWAAN ? 0 : this.attr.nomor(BAWAAN, this.bg, 0); };
  Vt.prototype.barisKosong = function () { return new Baris(this.kolom, this.kosong()); };
  Vt.prototype.gulirAtas = function (n, atas, bawah) {
    atas = atas == null ? this.atas : atas; bawah = bawah == null ? this.bawah : bawah;
    n = Math.min(n, bawah - atas + 1);
    for (var i = 0; i < n; i++) {
      var b = this.buf.splice(atas, 1)[0];
      if (this.buf === this.utama && atas === 0) this.dorongSb(b);
      this.buf.splice(bawah, 0, this.barisKosong());
    }
    this.kotorRentang(atas, bawah);
  };
  Vt.prototype.gulirBawah = function (n, atas, bawah) {
    atas = atas == null ? this.atas : atas; bawah = bawah == null ? this.bawah : bawah;
    n = Math.min(n, bawah - atas + 1);
    for (var i = 0; i < n; i++) {
      this.buf.splice(bawah, 1);
      this.buf.splice(atas, 0, this.barisKosong());
    }
    this.kotorRentang(atas, bawah);
  };
  Vt.prototype.kotorRentang = function (a, b) {
    if (a === 0 && b === this.baris - 1) this.semua = true;
    else if (!this.semua) for (var y = a; y <= b; y++) this.kotor[y] = true;
    this.minta();
  };
  Vt.prototype.lf = function () {
    if (this.y === this.bawah) this.gulirAtas(1);
    else if (this.y < this.baris - 1) { this.y++; }
  };
  Vt.prototype.hapusSel = function (y, dari, sampai) {
    var b = this.buf[y], k = this.kosong();
    if (!b) return;
    if (dari > 0 && b.c[dari] === '') b.c[dari - 1] = ' ';
    if (sampai < this.kolom && b.c[sampai] === '') b.c[sampai] = ' ';
    for (var x = dari; x < sampai && x < this.kolom; x++) { b.c[x] = ' '; b.a[x] = k; }
    if (sampai >= this.kolom) b.w = false;
    this.tandai(y);
  };
  Vt.prototype.ke = function (x, y) {
    var atas = this.mode.asal ? this.atas : 0, bawah = this.mode.asal ? this.bawah : this.baris - 1;
    this.x = Math.max(0, Math.min(this.kolom - 1, x));
    this.y = Math.max(atas, Math.min(bawah, y + atas));
    this.tunda = false;
  };

  Vt.prototype.cetak = function (ch, w) {
    if (w === 0) {
      var px = this.tunda ? this.x : this.x - 1, b0 = this.buf[this.y];
      if (px >= 0 && b0) {
        if (b0.c[px] === '' && px > 0) px--;
        b0.c[px] += ch; this.tandai(this.y);
      }
      return;
    }
    if (this.tunda) {
      if (this.mode.awBungkus) { this.buf[this.y].w = true; this.x = 0; this.lf(); }
      this.tunda = false;
    }
    if (w === 2 && this.x === this.kolom - 1) {
      if (!this.mode.awBungkus) return;
      this.hapusSel(this.y, this.x, this.kolom);
      this.buf[this.y].w = true; this.x = 0; this.lf();
    }
    var b = this.buf[this.y];
    if (this.mode.sisip) {
      for (var i = 0; i < w; i++) { b.c.splice(this.x, 0, ' '); b.a.splice(this.x, 0, this.aKini); }
      b.c.length = this.kolom; b.a.length = this.kolom;
    }
    if (b.c[this.x] === '' && this.x > 0) b.c[this.x - 1] = ' ';
    if (w === 1 && b.c[this.x + 1] === '' ) b.c[this.x + 1] = ' ';
    b.c[this.x] = ch; b.a[this.x] = this.aKini;
    this.terakhir = ch;
    if (w === 2) {
      if (this.x + 2 < this.kolom && b.c[this.x + 2] === '') b.c[this.x + 2] = ' ';
      b.c[this.x + 1] = ''; b.a[this.x + 1] = this.aKini;
    }
    this.tandai(this.y);
    this.x += w;
    if (this.x >= this.kolom) { this.x = this.kolom - 1; this.tunda = true; }
  };

  /* Jalur cepat: deretan ASCII tercetak ditulis langsung ke sel barisnya. Jawabannya indeks sesudahnya. */
  Vt.prototype.cetakAscii = function (s, i, n) {
    var k = this.kolom;
    while (i < n) {
      var c = s.charCodeAt(i);
      if (c < 32 || c >= 127) break;
      if (this.tunda) {
        if (this.mode.awBungkus) { this.buf[this.y].w = true; this.x = 0; this.lf(); }
        this.tunda = false;
      }
      var b = this.buf[this.y], x = this.x, a = this.aKini, mulai = i;
      if (b.c[x] === '' && x > 0) b.c[x - 1] = ' ';
      while (i < n && x < k) {
        c = s.charCodeAt(i);
        if (c < 32 || c >= 127) break;
        b.c[x] = s.charAt(i); b.a[x] = a; x++; i++;
      }
      if (x < k && b.c[x] === '') b.c[x] = ' ';
      if (i > mulai) this.terakhir = s.charAt(i - 1);
      this.tandai(this.y);
      if (x >= k) { this.x = k - 1; this.tunda = true; } else this.x = x;
    }
    return i;
  };

  /* ---------- pengurai ---------- */
  var DASAR = 0, ESC = 1, CSI = 2, OSC = 3, ABAI = 4, ESCANTARA = 5, OSCESC = 6, ABAIESC = 7;

  Vt.prototype.tulis = function (s) {
    var n = s.length;
    for (var i = 0; i < n; i++) {
      var c = s.charCodeAt(i);
      switch (this.keadaan) {
        case DASAR:
          if (c >= 32 && c < 127 && !this.mode.sisip && this.g[this.gl] !== '0') { i = this.cetakAscii(s, i, n) - 1; break; }
          if (c >= 32 && c !== 127) {
            /* Rangkaian huruf biasa diproses langsung di sini (jalur tercepat). */
            if (c >= 0xD800 && c <= 0xDBFF && i + 1 < n) {
              var d = s.charCodeAt(i + 1);
              if (d >= 0xDC00 && d <= 0xDFFF) {
                var cp = ((c - 0xD800) << 10) + (d - 0xDC00) + 0x10000;
                this.cetak(s.substr(i, 2), lebar(cp)); i++; continue;
              }
            }
            var ch = s.charAt(i);
            if (this.g[this.gl] === '0' && GARIS[ch]) ch = GARIS[ch];
            this.cetak(ch, c < 0x300 ? 1 : lebar(c));
          } else this.kendali(c);
          break;
        case ESC:
          this.escape(c);
          break;
        case ESCANTARA:
          if (c >= 0x20 && c <= 0x2F) { this.antara += s.charAt(i); break; }
          this.escAntara(this.antara, s.charAt(i));
          this.keadaan = DASAR;
          break;
        case CSI:
          if ((c >= 0x30 && c <= 0x3F)) this.param += s.charAt(i);
          else if (c >= 0x20 && c <= 0x2F) this.antara += s.charAt(i);
          else if (c >= 0x40 && c <= 0x7E) { this.keadaan = DASAR; this.csi(s.charAt(i)); }
          else if (c === 0x1B) { this.keadaan = ESC; }
          else if (c === 0x18 || c === 0x1A) this.keadaan = DASAR;
          else if (c < 0x20) this.kendali(c);
          break;
        case OSC:
          if (c === 7) { this.keadaan = DASAR; this.osc(this.oscIsi); }
          else if (c === 0x1B) this.keadaan = OSCESC;
          else if (c === 0x18 || c === 0x1A) this.keadaan = DASAR;
          else if (this.oscIsi.length < 100000) this.oscIsi += s.charAt(i);
          break;
        case OSCESC:
          this.keadaan = DASAR;
          this.osc(this.oscIsi);
          if (c !== 0x5C) { this.keadaan = ESC; this.escape(c); }
          break;
        case ABAI:
          if (c === 0x1B) this.keadaan = ABAIESC;
          else if (c === 7 || c === 0x18 || c === 0x1A) this.keadaan = DASAR;
          break;
        case ABAIESC:
          this.keadaan = c === 0x5C ? DASAR : ABAI;
          break;
      }
    }
  };

  Vt.prototype.kendali = function (c) {
    switch (c) {
      case 7: if (this.o.bel) this.o.bel(); break;
      case 8: if (this.tunda) this.tunda = false; else if (this.x > 0) this.x--; this.tandai(this.y); break;
      case 9: this.tabMaju(1); break;
      case 10: case 11: case 12: this.lf(); if (this.mode.lnm) this.x = 0; this.tunda = false; break;
      case 13: this.x = 0; this.tunda = false; break;
      case 14: this.gl = 1; break;
      case 15: this.gl = 0; break;
      case 27: this.keadaan = ESC; this.param = ''; this.antara = ''; break;
    }
  };
  Vt.prototype.tabMaju = function (n) {
    while (n-- > 0) {
      var x = this.x + 1;
      while (x < this.kolom - 1 && !this.tab[x]) x++;
      this.x = Math.min(x, this.kolom - 1);
    }
    this.tunda = false;
  };

  Vt.prototype.escape = function (c) {
    var ch = String.fromCharCode(c);
    this.keadaan = DASAR;
    switch (ch) {
      case '[': this.keadaan = CSI; this.param = ''; this.antara = ''; return;
      case ']': this.keadaan = OSC; this.oscIsi = ''; return;
      case 'P': case 'X': case '^': case '_': this.keadaan = ABAI; return;
      case '(': case ')': case '*': case '+': case '#': case ' ': case '%':
        this.keadaan = ESCANTARA; this.antara = ch; return;
      case '7': this.simpanKursor(); return;
      case '8': this.pulihKursor(); return;
      case 'D': this.lf(); this.tunda = false; return;
      case 'E': this.x = 0; this.lf(); this.tunda = false; return;
      case 'H': this.tab[this.x] = true; return;
      case 'M':
        if (this.y === this.atas) this.gulirBawah(1); else if (this.y > 0) this.y--;
        this.tunda = false; return;
      case 'c': this.reset(false); return;
      case '=': this.mode.keypadApl = true; return;
      case '>': this.mode.keypadApl = false; return;
      case '\\': return;
      case 'n': this.gl = 0; return;
    }
  };
  Vt.prototype.escAntara = function (antara, akhir) {
    var k = antara.charAt(0);
    if (k === '(' || k === ')') this.g[k === '(' ? 0 : 1] = akhir;
    else if (antara === '#' && akhir === '8') {
      for (var y = 0; y < this.baris; y++) for (var x = 0; x < this.kolom; x++) { this.buf[y].c[x] = 'E'; this.buf[y].a[x] = 0; }
      this.kotorSemua();
    }
  };

  Vt.prototype.simpanKursor = function () {
    this.simpan = { x: this.x, y: this.y, fg: this.fg, bg: this.bg, fl: this.fl, asal: this.mode.asal, g: this.g.slice(), gl: this.gl, tunda: this.tunda };
  };
  Vt.prototype.pulihKursor = function () {
    var s = this.simpan;
    if (!s) { this.x = 0; this.y = 0; this.tunda = false; return; }
    this.fg = s.fg; this.bg = s.bg; this.fl = s.fl; this.aKini = this.attr.nomor(this.fg, this.bg, this.fl);
    this.mode.asal = s.asal; this.g = s.g.slice(); this.gl = s.gl;
    this.x = Math.min(s.x, this.kolom - 1); this.y = Math.min(s.y, this.baris - 1); this.tunda = s.tunda;
  };

  function angka(p, i, bawaan) {
    var v = p[i];
    return v === undefined || v === '' || isNaN(v) ? bawaan : +v;
  }

  Vt.prototype.csi = function (akhir) {
    var mentah = this.param, pribadi = '';
    if (mentah && /[<=>?]/.test(mentah.charAt(0))) { pribadi = mentah.charAt(0); mentah = mentah.slice(1); }
    var p = mentah.split(';'), n1 = Math.max(1, angka(p, 0, 1)), y, i;
    var antara = this.antara;
    if (antara === ' ' && akhir === 'q') { this.gayaKursor = angka(p, 0, 0); this.tandai(this.y); return; }
    if (antara === '!' && akhir === 'p') { this.reset(true); return; }
    if (antara) return;
    switch (akhir) {
      case '@': {
        var b = this.buf[this.y], k = this.kosong();
        for (i = 0; i < n1; i++) { b.c.splice(this.x, 0, ' '); b.a.splice(this.x, 0, k); }
        b.c.length = this.kolom; b.a.length = this.kolom; this.tandai(this.y); break;
      }
      case 'A': this.y = Math.max(this.y < this.atas ? 0 : this.atas, this.y - n1); this.tunda = false; break;
      case 'B': case 'e': this.y = Math.min(this.y > this.bawah ? this.baris - 1 : this.bawah, this.y + n1); this.tunda = false; break;
      case 'C': case 'a': this.x = Math.min(this.kolom - 1, this.x + n1); this.tunda = false; break;
      case 'D': this.x = Math.max(0, this.x - n1); this.tunda = false; break;
      case 'E': this.x = 0; this.y = Math.min(this.bawah, this.y + n1); this.tunda = false; break;
      case 'F': this.x = 0; this.y = Math.max(this.atas, this.y - n1); this.tunda = false; break;
      case 'G': case '`': this.x = Math.min(this.kolom - 1, n1 - 1); this.tunda = false; break;
      case 'H': case 'f': this.ke(angka(p, 1, 1) - 1, n1 - 1); break;
      case 'I': this.tabMaju(n1); break;
      case 'Z':
        for (i = 0; i < n1; i++) { var x = this.x - 1; while (x > 0 && !this.tab[x]) x--; this.x = Math.max(0, x); }
        this.tunda = false; break;
      case 'J': {
        var m = angka(p, 0, 0);
        if (m === 0) { this.hapusSel(this.y, this.x, this.kolom); for (y = this.y + 1; y < this.baris; y++) this.hapusSel(y, 0, this.kolom); }
        else if (m === 1) { this.hapusSel(this.y, 0, this.x + 1); for (y = 0; y < this.y; y++) this.hapusSel(y, 0, this.kolom); }
        else if (m === 2) { for (y = 0; y < this.baris; y++) this.hapusSel(y, 0, this.kolom); }
        else if (m === 3) { this.sb = []; this.sbHapus = true; this.minta(); }
        break;
      }
      case 'K': {
        var mk = angka(p, 0, 0);
        if (mk === 0) this.hapusSel(this.y, this.x, this.kolom);
        else if (mk === 1) this.hapusSel(this.y, 0, this.x + 1);
        else this.hapusSel(this.y, 0, this.kolom);
        break;
      }
      case 'L': if (this.y >= this.atas && this.y <= this.bawah) { this.gulirBawah(n1, this.y, this.bawah); this.x = 0; } break;
      case 'M': if (this.y >= this.atas && this.y <= this.bawah) { this.gulirAtas2(n1); this.x = 0; } break;
      case 'P': {
        var bp = this.buf[this.y], kp = this.kosong();
        bp.c.splice(this.x, n1); bp.a.splice(this.x, n1);
        while (bp.c.length < this.kolom) { bp.c.push(' '); bp.a.push(kp); }
        this.tandai(this.y); break;
      }
      case 'S': this.gulirAtas(n1); break;
      case 'T': if (!pribadi) this.gulirBawah(n1); break;
      case 'X': this.hapusSel(this.y, this.x, this.x + n1); break;
      case 'b': if (this.terakhir) for (i = 0; i < Math.min(n1, 2000); i++) this.cetak(this.terakhir, lebar(this.terakhir.codePointAt(0))); break;
      case 'c':
        if (pribadi === '>') this.o.kirim('\x1b[>0;10;1c');
        else if (!pribadi) this.o.kirim('\x1b[?62;22c');
        break;
      case 'd': this.ke(this.x, n1 - 1); break;
      case 'g': { var mg = angka(p, 0, 0); if (mg === 0) delete this.tab[this.x]; else if (mg === 3) this.tab = {}; break; }
      case 'h': case 'l': this.setelMode(pribadi, p, akhir === 'h'); break;
      case 'm': if (!pribadi) this.sgr(mentah); break;
      case 'n':
        if (angka(p, 0, 0) === 5) this.o.kirim('\x1b[0n');
        else if (angka(p, 0, 0) === 6) this.o.kirim('\x1b[' + (pribadi === '?' ? '?' : '') + (this.y + 1 - (this.mode.asal ? this.atas : 0)) + ';' + (this.x + 1) + 'R');
        break;
      case 'r':
        if (!pribadi) {
          var a = angka(p, 0, 1) - 1, bw = angka(p, 1, this.baris) - 1;
          if (bw > this.baris - 1) bw = this.baris - 1;
          if (a < bw) { this.atas = Math.max(0, a); this.bawah = bw; this.ke(0, 0); }
        }
        break;
      case 's': if (!pribadi) this.simpanKursor(); break;
      case 'u': if (!pribadi) this.pulihKursor(); break;
      case 't':
        if (angka(p, 0, 0) === 18) this.o.kirim('\x1b[8;' + this.baris + ';' + this.kolom + 't');
        break;
    }
  };
  /* DL: baris di bawah kursor naik; yang hilang TIDAK masuk scrollback. */
  Vt.prototype.gulirAtas2 = function (n) {
    n = Math.min(n, this.bawah - this.y + 1);
    for (var i = 0; i < n; i++) { this.buf.splice(this.y, 1); this.buf.splice(this.bawah, 0, this.barisKosong()); }
    this.kotorRentang(this.y, this.bawah);
  };

  Vt.prototype.setelMode = function (pribadi, p, nyala) {
    for (var i = 0; i < p.length; i++) {
      var m = +p[i];
      if (pribadi !== '?') {
        if (m === 4) this.mode.sisip = nyala;
        else if (m === 20) this.mode.lnm = nyala;
        continue;
      }
      switch (m) {
        case 1: this.mode.kursorApl = nyala; break;
        case 5: this.mode.balik = nyala; this.akar.classList.toggle('balik', nyala); break;
        case 6: this.mode.asal = nyala; this.ke(0, 0); break;
        case 7: this.mode.awBungkus = nyala; break;
        case 25: this.mode.kursor = nyala; this.tandai(this.y); break;
        case 9: case 1000: case 1002: case 1003: this.mode.tetikus = nyala ? m : 0; break;
        case 1006: this.mode.sgr = nyala; break;
        case 1004: this.mode.fokus = nyala; break;
        case 2004: this.mode.tempel = nyala; break;
        case 47: case 1047: this.layarAlt(nyala, false); break;
        case 1048: if (nyala) this.simpanKursor(); else this.pulihKursor(); break;
        case 1049: this.layarAlt(nyala, true); break;
      }
    }
  };
  Vt.prototype.layarAlt = function (nyala, kursor) {
    if (nyala === (this.buf === this.alt && !!this.alt)) return;
    if (nyala) {
      if (kursor) this.simpanKursor();
      this.alt = this.layarBaru();
      this.buf = this.alt;
      this.akar.classList.add('alt');
    } else {
      this.buf = this.utama;
      this.alt = null;
      if (kursor) this.pulihKursor();
      this.akar.classList.remove('alt');
    }
    this.atas = 0; this.bawah = this.baris - 1;
    this.kotorSemua();
    if (this.o.ubahLayar) this.o.ubahLayar(nyala);
  };

  Vt.prototype.sgr = function (mentah) {
    var p = mentah === '' ? ['0'] : mentah.split(';'), i = 0;
    while (i < p.length) {
      var bagian = p[i].split(':'), v = bagian[0] === '' ? 0 : +bagian[0];
      if (v === 0) { this.fg = BAWAAN; this.bg = BAWAAN; this.fl = 0; }
      else if (v === 1) this.fl |= B_TEBAL;
      else if (v === 2) this.fl |= B_REDUP;
      else if (v === 3) this.fl |= B_MIRING;
      else if (v === 4) { if (bagian.length > 1 && bagian[1] === '0') this.fl &= ~B_GARIS; else this.fl |= B_GARIS; }
      else if (v === 5 || v === 6) this.fl |= B_KEDIP;
      else if (v === 7) this.fl |= B_BALIK;
      else if (v === 8) this.fl |= B_SEMBUNYI;
      else if (v === 9) this.fl |= B_CORET;
      else if (v === 21) this.fl |= B_GARIS;
      else if (v === 22) this.fl &= ~(B_TEBAL | B_REDUP);
      else if (v === 23) this.fl &= ~B_MIRING;
      else if (v === 24) this.fl &= ~B_GARIS;
      else if (v === 25) this.fl &= ~B_KEDIP;
      else if (v === 27) this.fl &= ~B_BALIK;
      else if (v === 28) this.fl &= ~B_SEMBUNYI;
      else if (v === 29) this.fl &= ~B_CORET;
      else if (v >= 30 && v <= 37) this.fg = v - 30;
      else if (v === 39) this.fg = BAWAAN;
      else if (v >= 40 && v <= 47) this.bg = v - 40;
      else if (v === 49) this.bg = BAWAAN;
      else if (v >= 90 && v <= 97) this.fg = v - 90 + 8;
      else if (v >= 100 && v <= 107) this.bg = v - 100 + 8;
      else if (v === 38 || v === 48 || v === 58) {
        var w = null;
        if (bagian.length > 1) {
          if (bagian[1] === '5') w = +bagian[2] & 255;
          else if (bagian[1] === '2') { var o = bagian.length >= 6 ? 3 : 2; w = 0x1000000 + (((+bagian[o] & 255) << 16) | ((+bagian[o + 1] & 255) << 8) | (+bagian[o + 2] & 255)); }
        } else if (p[i + 1] === '5') { w = +p[i + 2] & 255; i += 2; }
        else if (p[i + 1] === '2') { w = 0x1000000 + (((+p[i + 2] & 255) << 16) | ((+p[i + 3] & 255) << 8) | (+p[i + 4] & 255)); i += 4; }
        if (w !== null && !isNaN(w)) { if (v === 38) this.fg = w; else if (v === 48) this.bg = w; }
      }
      i++;
    }
    this.aKini = this.attr.nomor(this.fg, this.bg, this.fl);
  };

  Vt.prototype.osc = function (isi) {
    var i = isi.indexOf(';'), kode = i < 0 ? isi : isi.slice(0, i), sisa = i < 0 ? '' : isi.slice(i + 1);
    if (kode === '0' || kode === '2') { if (this.o.judul) this.o.judul(sisa); }
    else if (this.o.osc) this.o.osc(kode, sisa);
  };

  /* ---------- penggambar ---------- */
  Vt.prototype.html = function (b, cx) {
    var out = '', run = '', aKini = -1, tag = '', at = this.attr, n = b.c.length, akhir = n;
    if (cx < 0) { while (akhir > 0 && b.c[akhir - 1] === ' ' && b.a[akhir - 1] === 0) akhir--; }
    for (var x = 0; x < akhir; x++) {
      var ch = b.c[x];
      if (ch === '') continue;
      var a = b.a[x], w2 = x + 1 < n && b.c[x + 1] === '';
      if (x === cx || w2) {
        if (run) { out += tag ? tag + run + '</span>' : run; run = ''; }
        aKini = -1;
        var t = at.css(a), k = (t.k + (x === cx ? ' vkur' : '') + (w2 ? ' vw2' : '')).trim();
        out += '<span class="' + k + '"' + (t.g ? ' style="' + t.g + '"' : '') + '>' + (ch === ' ' ? ' ' : ch.replace(/[<&>]/g, esc1)) + '</span>';
        continue;
      }
      if (a !== aKini) {
        if (run) { out += tag ? tag + run + '</span>' : run; run = ''; }
        aKini = a; tag = at.css(a).buka;
      }
      run += ch.length === 1 ? esc1(ch) : ch.replace(/[<&>]/g, esc1);
    }
    if (run) out += tag ? tag + run + '</span>' : run;
    return out || ' ';
  };

  Vt.prototype.gambar = function () {
    var self = this;
    if (this.sbHapus) { this.elSb.textContent = ''; this.sbHapus = false; this.sbBaru = 0; this.sbJumlah = 0; }
    /* Selagi keluaran membanjir, scrollback ditambahkan paling sering tiap 250 md; layarnya sendiri
       tetap digambar tiap bingkai. Menata ribuan baris per bingkai yang membuat banjir lambat. */
    var kini = Date.now(), self2 = this;
    if (this.sbBaru && kini - (this.sbKapan || 0) < 250) {
      if (!this.sbTimer) this.sbTimer = setTimeout(function () { self2.sbTimer = 0; self2.minta(); }, 260);
    } else if (this.sbBaru) {
      this.sbKapan = kini;
      var n = Math.min(this.sbBaru, this.sb.length, this.maksSb), h = '';
      for (var i = this.sb.length - n; i < this.sb.length; i++) h += '<div class="tr">' + this.html(this.sb[i], -1) + '</div>';
      /* Satu blok per tambahan, bertinggi pasti: blok di luar layar tidak ditata sama sekali. */
      var blok = '<div class="sbk" data-n="' + n + '" style="contain-intrinsic-size:auto ' + (n * (this.tinggiSel || 16)) + 'px">' + h + '</div>';
      if (n >= this.maksSb) { this.elSb.innerHTML = blok; this.sbJumlah = n; }
      else {
        this.elSb.insertAdjacentHTML('beforeend', blok);
        this.sbJumlah = (this.sbJumlah || 0) + n;
        while (this.sbJumlah - (+this.elSb.firstChild.getAttribute('data-n')) >= this.maksSb) {
          this.sbJumlah -= +this.elSb.firstChild.getAttribute('data-n');
          this.elSb.removeChild(this.elSb.firstChild);
        }
      }
      this.sbBaru = 0;
    }
    var el = this.elScr;
    while (el.childElementCount < this.baris) { var r = document.createElement('div'); r.className = 'tr'; el.appendChild(r); }
    while (el.childElementCount > this.baris) el.removeChild(el.lastChild);
    var kursor = this.mode.kursor && !this.sembunyiKursor;
    if (this.kursorLama !== undefined && this.kursorLama !== this.y) this.kotor[this.kursorLama] = true;
    this.kotor[this.y] = true;
    var anak = el.children;
    for (var y = 0; y < this.baris; y++) {
      if (!this.semua && !this.kotor[y]) continue;
      var bx = this.buf[y];
      if (bx) anak[y].innerHTML = this.html(bx, kursor && y === this.y ? this.x : -1);
    }
    this.kursorLama = this.y;
    this.kotor = {}; this.semua = false;
    this.akar.setAttribute('data-kursor', ['blok', 'blok', 'blok', 'bawah', 'bawah', 'garis', 'garis'][this.gayaKursor] || 'blok');
    if (this.o.digambar) this.o.digambar();
  };

  /* Teks semua baris (scrollback + layar), untuk salin dan cari. */
  Vt.prototype.semuaTeks = function () {
    var t = [];
    function tambah(b) {
      var s = b.teks();
      if (t.length && t.sambung) t[t.length - 1] += s; else t.push(s);
      t.sambung = b.w;
    }
    if (this.buf === this.utama) this.sb.forEach(tambah);
    this.buf.forEach(tambah);
    while (t.length && t[t.length - 1] === '') t.pop();
    return t.join('\n');
  };

  /* Kunci dari papan ketik menjadi urutan yang dikirim ke PTY. `m`: {ctrl, alt, shift}. */
  Vt.prototype.urutanTombol = function (key, m) {
    var mod = 1 + (m.shift ? 1 : 0) + (m.alt ? 2 : 0) + (m.ctrl ? 4 : 0), pakaiMod = mod > 1;
    var panah = { ArrowUp: 'A', ArrowDown: 'B', ArrowRight: 'C', ArrowLeft: 'D', Home: 'H', End: 'F' };
    if (panah[key]) {
      if (pakaiMod) return '\x1b[1;' + mod + panah[key];
      return (this.mode.kursorApl ? '\x1bO' : '\x1b[') + panah[key];
    }
    var tilde = { Insert: 2, Delete: 3, PageUp: 5, PageDown: 6, F5: 15, F6: 17, F7: 18, F8: 19, F9: 20, F10: 21, F11: 23, F12: 24 };
    if (tilde[key]) return '\x1b[' + tilde[key] + (pakaiMod ? ';' + mod : '') + '~';
    var f14 = { F1: 'P', F2: 'Q', F3: 'R', F4: 'S' };
    if (f14[key]) return pakaiMod ? '\x1b[1;' + mod + f14[key] : '\x1bO' + f14[key];
    var a = m.alt ? '\x1b' : '';
    if (key === 'Enter') return a + '\r';
    if (key === 'Backspace') return a + (m.ctrl ? '\x08' : '\x7f');
    if (key === 'Tab') return m.shift ? '\x1b[Z' : a + '\t';
    if (key === 'Escape') return a + '\x1b';
    if (key.length === 1 || (key.length === 2 && key.charCodeAt(0) >= 0xD800)) {
      var ch = key;
      if (m.ctrl) {
        var c = ch.toUpperCase().charCodeAt(0);
        if (c >= 64 && c <= 95) ch = String.fromCharCode(c - 64);
        else if (ch === ' ' || ch === '2') ch = '\x00';
        else if (ch === '/' || ch === '7' || ch === '-') ch = '\x1f';
        else if (ch === '3') ch = '\x1b';
        else if (ch === '4') ch = '\x1c';
        else if (ch === '5') ch = '\x1d';
        else if (ch === '6' || ch === '~') ch = '\x1e';
        else if (ch === '8' || ch === '?') ch = '\x7f';
      }
      return a + ch;
    }
    return '';
  };

  /* Laporan tetikus (mode 1000/1002/1003, SGR 1006). `jenis`: 'tekan' | 'lepas' | 'gerak' | 'roda'. */
  Vt.prototype.urutanTetikus = function (jenis, tombol, x, y, m) {
    if (!this.mode.tetikus) return '';
    if (jenis === 'gerak' && this.mode.tetikus < 1002) return '';
    var b = jenis === 'roda' ? 64 + tombol : jenis === 'gerak' ? 32 + tombol : tombol;
    if (m) b += (m.shift ? 4 : 0) + (m.alt ? 8 : 0) + (m.ctrl ? 16 : 0);
    x = Math.max(0, Math.min(this.kolom - 1, x)) + 1; y = Math.max(0, Math.min(this.baris - 1, y)) + 1;
    if (this.mode.sgr) return '\x1b[<' + b + ';' + x + ';' + y + (jenis === 'lepas' ? 'm' : 'M');
    if (jenis === 'lepas') b = 3;
    return '\x1b[M' + String.fromCharCode(32 + b, 32 + Math.min(x, 223), 32 + Math.min(y, 223));
  };

  IDE.Vt = Vt;
  IDE.vtLebar = lebar;
})(window.IDE);
