/* JRG IDE — js/penyunting.js (diturunkan dari penyunting aplikasi Mawar): <pre> berwarna di bawah,
   <textarea> transparan di atas, dalam SATU wadah gulir. Kenapa begitu, dan syarat presisinya:
   ide/README.md, "Penyunting". */
(function (M) {
  'use strict';

  var U = M.util, esc = U.esc;
  var PASANG = { '(': ')', '[': ']', '{': '}', '"': '"', '`': '`' };
  var PENUTUP = { ')': 1, ']': 1, '}': 1, '"': 1, '`': 1 };
  var TAB = '    ';

  function Editor(host, opts) {
    if (!(this instanceof Editor)) return new Editor(host, opts);
    opts = opts || {};
    this.opts = opts;
    this.lang = opts.lang || 'plain';
    this.wrap = !!opts.wrap;
    this.fontSize = opts.fontSize || 14;
    this.readOnly = !!opts.readOnly;
    host.innerHTML =
      '<div class="ed' + (this.wrap ? ' ed--wrap' : '') + '">' +
        '<div class="ed-scroll">' +
          '<div class="ed-body">' +
            '<div class="ed-gutter" aria-hidden="true"></div>' +
            '<div class="ed-code">' +
              '<div class="ed-line-hi" aria-hidden="true"></div>' +
              '<div class="ed-kurung" aria-hidden="true" hidden></div><div class="ed-kurung" aria-hidden="true" hidden></div>' +
              '<pre class="ed-hl" aria-hidden="true"><div class="ed-sizer"><div class="ed-win"></div></div></pre>' +
              '<textarea class="ed-ta" spellcheck="false" autocomplete="off" autocorrect="off" autocapitalize="off" ' +
                'data-gramm="false" wrap="' + (this.wrap ? 'soft' : 'off') + '" aria-label="' + esc(opts.label || 'Kode') + '"' + (this.readOnly ? ' readonly' : '') + '></textarea>' +
            '</div>' +
          '</div>' +
        '</div>' +
        '<span class="ed-probe" aria-hidden="true">0000000000</span>' +
      '</div>';
    this.root = host.firstChild;
    this.scroll = this.root.querySelector('.ed-scroll');
    this.gutter = this.root.querySelector('.ed-gutter');
    this.hl = this.root.querySelector('.ed-hl');
    this.sizer = this.root.querySelector('.ed-sizer');
    this.win = this.root.querySelector('.ed-win');
    this.barisTeks = [];          /* baris mentah */
    this.barisHtml = [];          /* sorotan per baris (cache) */
    this.maksKolom = 0;
    this.sorotTimer = 0;
    this.jendela = [-1, -1];
    this.ta = this.root.querySelector('.ed-ta');
    this.lineHi = this.root.querySelector('.ed-line-hi');
    this.kurung = this.root.querySelectorAll('.ed-kurung');
    this.probe = this.root.querySelector('.ed-probe');
    this.nLines = 0;
    this.raf = 0;
    this.tanda = {};              /* baris (1-based) -> pesan galat, dari pemeriksa */
    this.lemah = {};              /* baris yang cuma berisi peringatan (lint), bukan galat */
    /* Jendela ketik: textarea cuma memegang baris w0.. (lihat pasangJendela); sisanya di pre/post. */
    this.pre = ''; this.post = ''; this.w0 = 0; this.preLen = 0; this.lamaMid = '';
    this.riwayat = []; this.maju = []; this.selLama = [0, 0]; this.menyusun = false;
    this.setFont(this.fontSize, true);
    this.ta.value = opts.value || '';
    this.lamaMid = this.ta.value;
    this.periksaBesar();
    this.bind();
    this.render();
  }

  /* Biaya satu ketikan di <textarea> Chromium tumbuh KUADRAT terhadap letak kursor (diukur: 300 baris
     13 ms, 1000 baris 66 ms, 4000 baris 774 ms per huruf), jadi berkas besar dipegang bertiga:
     `pre` + isi textarea (paling banyak JENDELA_MAKS baris di sekitar kursor) + `post`. */
  var JENDELA = 300, TEPI = 60, JENDELA_MAKS = 600;

  /* Offset (mutlak) awal baris ke-i (0-based) menurut barisTeks. */
  Editor.prototype.awalBaris = function (i) {
    var L = this.barisTeks, o = 0;
    for (var k = 0; k < i && k < L.length; k++) o += L[k].length + 1;
    return o;
  };
  Editor.prototype.barisDari = function (off) {
    var L = this.barisTeks, o = 0;
    for (var k = 0; k < L.length; k++) { o += L[k].length + 1; if (o > off) return k; }
    return Math.max(0, L.length - 1);
  };
  /* Isi textarea jadi baris di sekitar baris `tengah` (0-based), pilihan mutlak a..b dipertahankan.
     `penuh`: seluruh berkas (Pilih semua, mode bungkus). barisTeks harus sudah sesuai isi. */
  Editor.prototype.pasangJendela = function (tengah, a, b, penuh) {
    var n = this.barisTeks.length, w0 = 0, w1 = n;
    if (!penuh && !this.wrap && n > JENDELA_MAKS) {
      w0 = Math.max(0, Math.min(n - JENDELA, tengah - (JENDELA >> 1)));
      w1 = Math.min(n, w0 + JENDELA);
    }
    var v = this.getValue(), awal = this.awalBaris(w0), akhir = w1 >= n ? v.length : this.awalBaris(w1) - 1;
    var arah = this.ta.selectionDirection;
    this.penuhPaksa = !!penuh;
    this.pre = v.slice(0, awal); this.post = v.slice(akhir);
    this.w0 = w0; this.preLen = awal;
    var mid = v.slice(awal, akhir);
    if (this.ta.value !== mid) this.ta.value = mid;
    this.lamaMid = mid;
    var pad = w0 ? (this.padAtas() + w0 * this.lh) + 'px' : '';
    if (this.ta.style.paddingTop !== pad) this.ta.style.paddingTop = pad;
    var jepit = function (x) { return Math.max(0, Math.min(mid.length, x - awal)); };
    try { this.ta.setSelectionRange(jepit(a), jepit(b), arah); } catch (e) { /* abaikan */ }
  };
  /* Dipanggil sesudah ketikan atau gerak kursor: kursor yang mendekati tepi jendela memindahkannya. */
  Editor.prototype.jagaJendela = function () {
    if (this.menyusun || this.wrap) return;
    var n = this.barisTeks.length, isi = this.ta.value;
    if (n <= JENDELA_MAKS && !this.pre && !this.post) return;
    var s = this.ta.selectionStart, e = this.ta.selectionEnd;
    if (this.penuhPaksa && s !== e) return;
    this.penuhPaksa = false;
    var nMid = 1, i = -1;
    while ((i = isi.indexOf('\n', i + 1)) >= 0) nMid++;
    var lokal = 0;
    i = -1;
    while ((i = isi.indexOf('\n', i + 1)) >= 0 && i < s) lokal++;
    var perlu = n <= JENDELA_MAKS || nMid > JENDELA_MAKS ||
      (this.w0 > 0 && lokal < TEPI) || (this.w0 + nMid < n && nMid - 1 - lokal < TEPI);
    if (perlu) this.pasangJendela(this.w0 + lokal, this.preLen + s, this.preLen + e);
  };
  /* Pilihan mutlak [awal, akhir] dan teks terpilih. */
  Editor.prototype.sel = function () { return [this.preLen + this.ta.selectionStart, this.preLen + this.ta.selectionEnd]; };
  Editor.prototype.terpilih = function () { return this.ta.value.slice(this.ta.selectionStart, this.ta.selectionEnd); };


  Editor.prototype.setFont = function (px, diam) {
    this.fontSize = U.clamp(px, 10, 26);
    var lh = Math.round(this.fontSize * 1.6);       /* bilangan bulat: tidak ada pembulatan per baris */
    this.root.style.setProperty('--ed-fs', this.fontSize + 'px');
    this.root.style.setProperty('--ed-lh', lh + 'px');
    this.lh = lh;
    this.cw = 0;
    this.__pad = null;
    this.jendela = [-1, -1];
    if (!diam) this.render();
  };
  Editor.prototype.setWrap = function (on) {
    /* bungkus baris membuat tinggi tiap baris berbeda, jadi jendela tidak bisa
       dihitung — untuk berkas raksasa bungkus ditolak, bukan dipaksakan */
    if (on && this.getValue().length > 400000) {
      M.toast('Berkas ini terlalu besar untuk mode bungkus baris', { icon: 'info' });
      return;
    }
    this.wrap = !!on;
    this.jendela = [-1, -1];
    this.gutter.style.paddingTop = '';
    this.root.classList.toggle('ed--wrap', this.wrap);
    this.ta.setAttribute('wrap', this.wrap ? 'soft' : 'off');
    var s = this.sel();
    this.render();
    this.pasangJendela(this.barisDari(s[0]), s[0], s[1]);
  };
  Editor.prototype.setLang = function (lang) {
    this.lang = lang || 'plain';
    this.barisHtml = [];
    this.jendela = [-1, -1];
    this.render();
  };
  Editor.prototype.getValue = function () { return this.pre + this.ta.value + this.post; };
  /* setValue mengganti isi (riwayat undo ikut kosong — dipakai saat ganti berkas) */
  Editor.prototype.setValue = function (v) {
    this.pre = String(v == null ? '' : v); this.post = ''; this.ta.value = ''; this.preLen = 0; this.w0 = 0;
    this.barisTeks = []; this.barisHtml = []; this.jendela = [-1, -1];
    this.riwayat = []; this.maju = []; this.selLama = [0, 0];
    this.nomorIsi = (this.nomorIsi || 0) + 1;
    this.periksaBesar();
    this.scroll.scrollTop = 0; this.scroll.scrollLeft = 0;
    this.render();
    this.pasangJendela(0, 0, 0);
  };
  /* Ganti seluruh isi sebagai SATU langkah urung (Ganti semua, Rapikan, Pulihkan); `sel` = pilihan sesudahnya. */
  Editor.prototype.gantiSemua = function (v, sel) {
    var lama = this.getValue(), s = this.sel();
    if (lama === v) return;
    sel = sel || [0, 0];
    this.riwayat.push({ at: 0, del: lama, ins: v, selSebelum: s, selSesudah: sel, t: 0 });
    this.maju = [];
    this.terapkanTeks(v, sel);
  };
  /* Keadaan per tab (riwayat urung, kursor, gulir), supaya pindah tab tidak menghapusnya. */
  Editor.prototype.keadaan = function () {
    return { riwayat: this.riwayat, maju: this.maju, sel: this.sel(), atas: this.scroll.scrollTop, kiri: this.scroll.scrollLeft, isi: this.getValue() };
  };
  Editor.prototype.pulihkan = function (k) {
    if (!k || k.isi !== this.getValue()) return;
    this.riwayat = k.riwayat; this.maju = k.maju;
    var n = this.getValue().length, a = Math.min(k.sel[0], n), b = Math.min(k.sel[1], n);
    this.pasangJendela(this.barisDari(a), a, b);
    this.selLama = [a, b];
    this.scroll.scrollTop = k.atas; this.scroll.scrollLeft = k.kiri;
    this.gambarJendela(true);
    this.cursorUI();
  };
  /* Isi baru tanpa mengosongkan riwayat (urung/ulang); pilihan mutlak `sel`. */
  Editor.prototype.terapkanTeks = function (v, sel) {
    this.pre = v; this.post = ''; this.ta.value = ''; this.preLen = 0; this.w0 = 0;
    this.periksaBesar();
    this.render();
    this.pasangJendela(this.barisDari(sel[0]), sel[0], sel[1]);
    this.focus();
    this.cursorUI();
    this.revealCaret();
    if (this.opts.onChange) this.opts.onChange(this.getValue());
  };
  /* Catat perubahan textarea sejak catatan terakhir sebagai langkah urung; ketikan beruntun digabung. */
  Editor.prototype.catat = function () {
    var lama = this.lamaMid, baru = this.ta.value;
    if (lama === baru) return;
    var n = Math.min(lama.length, baru.length), a = 0, z = 0;
    while (a < n && lama.charCodeAt(a) === baru.charCodeAt(a)) a++;
    while (z < n - a && lama.charCodeAt(lama.length - 1 - z) === baru.charCodeAt(baru.length - 1 - z)) z++;
    var l = { at: this.preLen + a, del: lama.slice(a, lama.length - z), ins: baru.slice(a, baru.length - z),
      selSebelum: this.selLama, selSesudah: this.sel(), t: Date.now() };
    var p = this.riwayat[this.riwayat.length - 1];
    var dekat = p && l.t - p.t < 1500;
    if (dekat && !p.del && !l.del && p.at + p.ins.length === l.at && l.ins.indexOf('\n') < 0) {
      p.ins += l.ins; p.selSesudah = l.selSesudah; p.t = l.t;
    } else if (dekat && !p.ins && !l.ins && l.at + l.del.length === p.at && l.del.indexOf('\n') < 0) {
      p.del = l.del + p.del; p.at = l.at; p.selSesudah = l.selSesudah; p.t = l.t;
    } else {
      this.riwayat.push(l);
      if (this.riwayat.length > 400) this.riwayat.shift();
    }
    this.maju = [];
    this.lamaMid = baru;
    this.selLama = l.selSesudah;
  };
  /* berkas raksasa: hanya-baca, dengan sebabnya disebut sekali */
  Editor.prototype.periksaBesar = function () {
    var besar = this.pre.length + this.ta.value.length + this.post.length > BATAS_UBAH;
    if (besar === this.__besar) return besar;
    this.__besar = besar;
    this.ta.readOnly = besar || this.readOnly;
    this.root.classList.toggle('ed--besar', besar);
    if (besar && this.opts.onBesar) this.opts.onBesar(this.ta.value.length);
    return besar;
  };

  Editor.prototype.focus = function () { try { this.ta.focus({ preventScroll: true }); } catch (e) { this.ta.focus(); } };

  /* lebar satu karakter (font berlebar tetap), diukur, bukan ditebak */
  Editor.prototype.charW = function () {
    if (!this.cw) {
      var w = this.probe.getBoundingClientRect().width / 10;
      this.cw = w > 0 ? w : this.fontSize * 0.6;
    }
    return this.cw;
  };

  /* Di atas batas ini sorotan dimatikan: menyorot berkas sebesar itu memakan
     waktu lebih lama daripada nilainya, dan itulah yang dulu membuat tempelan
     teks raksasa terasa seperti aplikasi mati. */
  var BATAS_SOROT = 1500000;
  /* Di atas batas ini isinya dibuka HANYA-BACA: satu textarea berisi lebih dari
     sejuta karakter dilayout ulang tiap ketikan, dan hasilnya bukan lambat
     melainkan aplikasi yang tampak mati. Dikatakan, bukan dibiarkan. */
  var BATAS_UBAH = 1200000;
  var PENYANGGA = 40;             /* baris ekstra di atas/bawah layar */

  /* Sorot SATU baris tanpa konteks baris lain. Dipakai saat mengetik supaya
     hasilnya langsung terlihat; sorotan penuh menyusul lewat sorotPenuh(). */
  Editor.prototype.sorotBaris = function (teks) {
    if (this.lang === 'plain' || teks.length > 4000) return esc(teks);
    var h = M.hl.render(teks, this.lang);
    return h.indexOf('\n') >= 0 ? esc(teks) : h;
  };

  /* Sorotan penuh: satu lintasan atas seluruh teks, menutup-buka span yang
     melewati batas baris. Ditunda sedikit supaya ketikan tidak menunggu. */
  Editor.prototype.sorotPenuh = function () {
    var v = this.getValue(), self = this;
    if (v.length > BATAS_SOROT || this.lang === 'plain') {
      this.barisHtml = this.barisTeks.map(esc);
      this.gambarJendela(true);
      return;
    }
    var p = v.length > 30000 && pekerjaSorot();
    if (!p) { this.terimaSorot(M.hl.barisSeimbang(M.hl.render(v, this.lang))); return; }
    var id = ++nomorSorot;
    delete tungguSorot[this.idSorot];
    this.idSorot = id;
    tungguSorot[id] = function (baris) { if (self.getValue() === v) self.terimaSorot(baris); };
    p.postMessage({ id: id, teks: v, lang: this.lang });
  };
  Editor.prototype.terimaSorot = function (baris) {
    if (baris.length !== this.barisTeks.length) return;
    this.barisHtml = baris;
    this.gambarJendela(true);
  };
  /* Satu pekerja untuk semua penyunting; jawaban yang sudah usang (teksnya berubah lagi) dibuang. */
  var pekerja = null, nomorSorot = 0, tungguSorot = {};
  function pekerjaSorot() {
    if (pekerja === null) {
      try {
        pekerja = new Worker('/web/js/pekerja_sorot.js');
        pekerja.onmessage = function (e) { var f = tungguSorot[e.data.id]; delete tungguSorot[e.data.id]; if (f) f(e.data.baris); };
        pekerja.onerror = function () { pekerja = false; };
      } catch (e) { pekerja = false; }
    }
    return pekerja;
  }

  Editor.prototype.render = function () {
    var v = this.getValue(), self = this;
    var lamaTeks = this.barisTeks;
    var baru = v.split('\n');
    /* baris yang berubah disorot seketika satu per satu; sisanya pakai cache */
    if (this.barisHtml.length !== baru.length) {
      var htmlBaru = new Array(baru.length);
      /* awal dan akhir yang sama dipertahankan supaya menempel teks besar
         tidak memaksa menyorot ulang seluruh berkas */
      var a = 0;
      while (a < baru.length && a < lamaTeks.length && baru[a] === lamaTeks[a]) { htmlBaru[a] = this.barisHtml[a]; a++; }
      var z = 0;
      while (z < baru.length - a && z < lamaTeks.length - a && baru[baru.length - 1 - z] === lamaTeks[lamaTeks.length - 1 - z]) {
        htmlBaru[baru.length - 1 - z] = this.barisHtml[lamaTeks.length - 1 - z]; z++;
      }
      for (var i = a; i < baru.length - z; i++) htmlBaru[i] = null;   /* diisi saat digambar */
      this.barisHtml = htmlBaru;
    } else {
      for (var k = 0; k < baru.length; k++) if (baru[k] !== lamaTeks[k]) this.barisHtml[k] = null;
    }
    this.barisTeks = baru;
    this.nLines = baru.length;

    /* lebar gulir mendatar: dari baris terpanjang, dihitung ulang seperlunya */
    var maks = 0;
    for (k = 0; k < baru.length; k++) if (baru[k].length > maks) maks = baru[k].length;
    this.maksKolom = maks;

    this.gambarJendela(true);
    this.renderGutter();
    clearTimeout(this.sorotTimer);
    if (this.lang !== 'plain' && v.length <= BATAS_SOROT) {
      this.sorotTimer = setTimeout(function () { self.sorotPenuh(); }, v.length > 60000 ? 220 : 80);
    }
    if (!this.raf) this.raf = requestAnimationFrame(function () { self.raf = 0; self.cursorUI(); });
  };

  /* Gambar HANYA baris yang terlihat. Tanpa ini, berkas 100 ribu baris
     membuat 100 ribu elemen sekaligus — itu yang bikin mati, bukan teksnya. */
  /* Tinggi tampak DI-CACHE: membacanya sesudah menulis gaya memaksa layout,
     dan layout satu textarea berisi puluhan ribu baris memakan ratusan
     milidetik — itu yang membuat mengetik terasa berat, bukan sorotannya. */
  Editor.prototype.tinggiTampak = function () {
    if (!this.__ht) this.__ht = this.scroll.clientHeight || 600;
    return this.__ht;
  };
  Editor.prototype.gambarJendela = function (paksa) {
    if (this.wrap) return this.gambarPenuhBungkus();
    var lh = this.lh, sc = this.scroll;
    var atas = Math.max(0, Math.floor((sc.scrollTop - this.padAtas()) / lh) - PENYANGGA);
    var bawah = Math.min(this.nLines, Math.ceil((sc.scrollTop + this.tinggiTampak() - this.padAtas()) / lh) + PENYANGGA);
    if (!paksa && atas === this.jendela[0] && bawah === this.jendela[1]) return;
    var sama = atas === this.jendela[0] && bawah === this.jendela[1] && this.tergambar && this.win.children.length === bawah - atas;
    this.jendela = [atas, bawah];
    var i, h;
    if (sama) {
      /* Jendela yang sama (ketikan biasa): cuma baris yang berubah yang disentuh, jadi layout-nya kecil. */
      for (i = atas; i < bawah; i++) {
        if (this.barisHtml[i] == null) this.barisHtml[i] = this.sorotBaris(this.barisTeks[i] || '');
        h = this.kelasLn(i + 1) + this.barisHtml[i];
        if (this.tergambar[i - atas] === h) continue;
        var el = this.win.children[i - atas];
        el.className = 'ln' + this.kelasLn(i + 1);
        el.innerHTML = this.barisHtml[i];
        this.tergambar[i - atas] = h;
      }
    } else {
      var buf = '';
      this.tergambar = [];
      for (i = atas; i < bawah; i++) {
        if (this.barisHtml[i] == null) this.barisHtml[i] = this.sorotBaris(this.barisTeks[i] || '');
        this.tergambar.push(this.kelasLn(i + 1) + this.barisHtml[i]);
        buf += '<span class="ln' + this.kelasLn(i + 1) + '">' + this.barisHtml[i] + '</span>';
      }
      this.win.innerHTML = buf;
    }
    var geser = 'translateY(' + (atas * lh) + 'px)';
    if (this.__geser !== geser) { this.win.style.transform = geser; this.__geser = geser; }
    var tinggi = (this.nLines * lh) + 'px';
    if (this.__tinggi !== tinggi) { this.sizer.style.height = tinggi; this.__tinggi = tinggi; }
    var lebar = Math.ceil(this.maksKolom * this.charW() + 8) + 'px';
    if (this.__lebar !== lebar) { this.sizer.style.minWidth = lebar; this.__lebar = lebar; }
    this.gambarGutterJendela(atas, bawah);
  };

  /* mode bungkus: tinggi tiap baris berbeda, jadi semuanya digambar */
  Editor.prototype.gambarPenuhBungkus = function () {
    var buf = '';
    for (var i = 0; i < this.nLines; i++) {
      if (this.barisHtml[i] == null) this.barisHtml[i] = this.sorotBaris(this.barisTeks[i] || '');
      buf += '<span class="ln' + this.kelasLn(i + 1) + '">' + this.barisHtml[i] + '</span>';
    }
    this.win.innerHTML = buf;
    this.tergambar = null;
    this.win.style.transform = '';
    this.sizer.style.height = '';
    this.sizer.style.minWidth = '';
    this.jendela = [0, this.nLines];
  };

  Editor.prototype.padAtas = function () {
    if (this.__pad == null) {
      var v = getComputedStyle(this.hl).paddingTop;
      this.__pad = parseFloat(v) || 12;
    }
    return this.__pad;
  };

  /* posisi atas sebuah baris di dalam wadah gulir */
  Editor.prototype.barisTop = function (n) {
    if (!this.wrap) return this.padAtas() + (n - 1) * this.lh;
    var el = this.win.children[n - 1];
    return el ? el.offsetTop + this.padAtas() : this.padAtas() + (n - 1) * this.lh;
  };
  Editor.prototype.barisTinggi = function (n) {
    if (!this.wrap) return this.lh;
    var el = this.win.children[n - 1];
    return el ? el.offsetHeight : this.lh;
  };

  Editor.prototype.renderGutter = function () {
    var n = this.nLines, g = this.gutter;
    var lebar = String(n).length;
    this.root.style.setProperty('--ed-gw', (Math.max(2, lebar) + 1.6) + 'ch');
    if (!this.wrap) { this.gambarGutterJendela(this.jendela[0], this.jendela[1]); return; }
    /* mode bungkus: tinggi tiap baris logis diukur dari <pre> */
    var blok = this.win.children, t = '';
    for (var k = 0; k < blok.length; k++) t += '<span style="height:' + blok[k].offsetHeight + 'px"' + this.kelasTanda(k + 1) + '>' + (k + 1) + '</span>';
    g.innerHTML = t; g.__isi = null;
    g.__wrap = true; g.__dari = 0;
  };

  Editor.prototype.gambarGutterJendela = function (atas, bawah) {
    var g = this.gutter;
    if (this.wrap) return;
    var s2 = '';
    for (var i = atas; i < bawah; i++) s2 += '<span' + this.kelasTanda(i + 1) + '>' + (i + 1) + '</span>';
    if (g.__isi === s2 && !g.__wrap) return;
    g.__isi = s2;
    g.innerHTML = s2;
    var pad = (this.padAtas() + atas * this.lh) + 'px';
    if (g.__pad !== pad) { g.style.paddingTop = pad; g.__pad = pad; }
    g.__wrap = false; g.__dari = atas;
  };

  Editor.prototype.kelasTanda = function (n) {
    var t = this.tanda[n];
    return t ? ' class="' + (this.lemah[n] ? 'is-wrn' : 'is-err') + '" title="' + esc(t) + '"' : '';
  };
  Editor.prototype.kelasLn = function (n) { return this.tanda[n] ? (this.lemah[n] ? ' wrn' : ' err') : ''; };
  /* Tanda per baris (`{12: 'pesan'}`), `lemah` menandai baris yang cuma berperingatan; kosong menghapus semuanya. */
  Editor.prototype.setTanda = function (tanda, lemah) {
    this.tanda = tanda || {};
    this.lemah = lemah || {};
    this.jendela = [-1, -1];
    if (this.wrap) { this.gambarPenuhBungkus(); this.renderGutter(); } else this.gambarJendela(true);
    this.cursorUI();
  };
  Editor.prototype.undo = function () {
    var l = this.riwayat.pop();
    if (!l) return this.focus();
    this.maju.push(l);
    var v = this.getValue();
    this.terapkanTeks(v.slice(0, l.at) + l.del + v.slice(l.at + l.ins.length), l.selSebelum);
    this.lamaMid = this.ta.value;
  };
  Editor.prototype.redo = function () {
    var l = this.maju.pop();
    if (!l) return this.focus();
    this.riwayat.push(l);
    var v = this.getValue();
    this.terapkanTeks(v.slice(0, l.at) + l.ins + v.slice(l.at + l.del.length), l.selSesudah);
    this.lamaMid = this.ta.value;
  };

  /* posisi kursor: baris & kolom (1-based) */
  /* `offset` mutlak di seluruh berkas, `lokal` di dalam textarea (jendela ketik). */
  Editor.prototype.pos = function () {
    var v = this.ta.value, p = this.ta.selectionStart;
    var sebelum = v.slice(0, p);
    var baris = this.w0 + sebelum.split('\n').length;
    var kolom = p - sebelum.lastIndexOf('\n');
    return { baris: baris, kolom: kolom, offset: this.preLen + p, lokal: p, pilih: this.ta.selectionEnd - p };
  };

  Editor.prototype.cursorUI = function () {
    this.jagaJendela();
    var p = this.pos();
    var top = this.barisTop(p.baris) - this.padAtas();
    this.lineHi.style.transform = 'translateY(' + top + 'px)';
    this.lineHi.style.height = this.barisTinggi(p.baris) + 'px';
    var akt = this.gutter.querySelector('.is-on');
    if (akt) akt.classList.remove('is-on');
    var gl = this.gutter.children[p.baris - 1 - (this.gutter.__dari || 0)];
    if (gl) gl.classList.add('is-on');
    this.sorotKurung(p);
    if (this.opts.onCursor) this.opts.onCursor(p);
  };

  /* Pasangan kurung di sebelah kursor diberi bingkai. Teks dan komentar tidak dibedakan (cukup untuk
     menemukan pasangannya); pencarian berhenti sesudah 20 ribu huruf supaya ketikan tetap ringan. */
  var BUKA = { '(': ')', '[': ']', '{': '}' }, TUTUP = { ')': '(', ']': '[', '}': '{' };
  Editor.prototype.pasanganKurung = function (off) {
    var v = this.getValue(), c = v[off], d, i, n = 0, batas = 20000;
    if (BUKA[c]) {
      for (i = off + 1, d = 0; i < v.length && n++ < batas; i++) {
        if (v[i] === c) d++;
        else if (v[i] === BUKA[c]) { if (!d) return i; d--; }
      }
    } else if (TUTUP[c]) {
      for (i = off - 1, d = 0; i >= 0 && n++ < batas; i--) {
        if (v[i] === c) d++;
        else if (v[i] === TUTUP[c]) { if (!d) return i; d--; }
      }
    }
    return -1;
  };
  Editor.prototype.sorotKurung = function (p) {
    var k = this.kurung, self = this;
    k[0].hidden = k[1].hidden = true;
    if (this.wrap || p.pilih || !this.lang || this.lang === 'plain' || this.lang === 'md') return;
    var v = this.getValue(), o = p.offset;
    var di = BUKA[v[o - 1]] || TUTUP[v[o - 1]] ? o - 1 : BUKA[v[o]] || TUTUP[v[o]] ? o : -1;
    if (di < 0) return;
    var lawan = this.pasanganKurung(di);
    if (lawan < 0) return;
    [di, lawan].forEach(function (x, j) {
      var b = self.barisDari(x), awal = self.awalBaris(b);
      var kol = v.slice(awal, x).replace(/\t/g, TAB).length;
      k[j].style.transform = 'translate(' + (kol * self.charW()) + 'px,' + (self.barisTop(b + 1) - self.padAtas()) + 'px)';
      k[j].hidden = false;
    });
  };

  /* jaga kursor tetap terlihat di dalam wadah gulir */
  Editor.prototype.revealCaret = function () {
    var p = this.pos(), sc = this.scroll;
    var y = this.barisTop(p.baris) - this.padAtas();
    var pad = this.lh;
    if (y - pad < sc.scrollTop) sc.scrollTop = Math.max(0, y - pad);
    else if (y + this.lh + pad > sc.scrollTop + sc.clientHeight) sc.scrollTop = y + this.lh + pad - sc.clientHeight;
    if (!this.wrap) {
      var baris = this.ta.value.slice(0, p.lokal).split('\n').pop().replace(/\t/g, TAB);
      if (!this.__gw) this.__gw = this.gutter.offsetWidth;
      var gw = this.__gw;
      var x = baris.length * this.charW() + 12;
      var kiri = sc.scrollLeft, lebar = sc.clientHeight ? sc.clientWidth - gw : 0;
      if (x < kiri + 24) sc.scrollLeft = Math.max(0, x - 48);
      else if (x > kiri + lebar - 24) sc.scrollLeft = x - lebar + 48;
    }
  };

  /* Sisipkan teks menggantikan dari..sampai. Offset-nya LOKAL (di dalam textarea, seperti prefix());
     untuk offset mutlak pakai gantiMutlak. */
  Editor.prototype.replace = function (dari, sampai, teks, pilihDari, pilihSampai) {
    var ta = this.ta;
    ta.focus();
    this.selLama = this.sel();
    ta.setSelectionRange(dari, sampai);
    var ok = false;
    try { ok = document.execCommand('insertText', false, teks); } catch (e) { ok = false; }
    if (!ok) { ta.setRangeText(teks, dari, sampai, 'end'); this.changed(); }
    if (pilihDari != null) ta.setSelectionRange(pilihDari, pilihSampai == null ? pilihDari : pilihSampai);
  };
  Editor.prototype.insert = function (teks, geserKursor) {
    var s = this.ta.selectionStart, e = this.ta.selectionEnd;
    this.replace(s, e, teks);
    if (geserKursor) { var q = s + teks.length + geserKursor; this.ta.setSelectionRange(q, q); }
    this.revealCaret();
  };

  Editor.prototype.gantiMutlak = function (a, b, teks, pilihDari, pilihSampai) {
    /* Rentang yang lebih panjang dari separuh jendela tidak muat di jendela biasa: seluruh berkas dipasang. */
    var la = this.barisDari(a), lb = this.barisDari(b);
    this.pasangJendela(la, a, b, lb - la > JENDELA / 3);
    var o = this.preLen;
    this.replace(a - o, b - o, teks, pilihDari == null ? null : pilihDari - o, pilihSampai == null ? null : pilihSampai - o);
  };

  Editor.prototype.changed = function () {
    this.catat();
    this.periksaBesar();
    this.render();
    this.jagaJendela();
    if (this.opts.onChange) this.opts.onChange(this.getValue());
  };

  Editor.prototype.indentBaris = function (keluar) {
    var ta = this.ta, v = ta.value, s = ta.selectionStart, e = ta.selectionEnd;
    var awal = v.lastIndexOf('\n', s - 1) + 1;
    var akhir = v.indexOf('\n', e > s && v[e - 1] === '\n' ? e - 1 : e); if (akhir < 0) akhir = v.length;
    var blok = v.slice(awal, akhir).split('\n');
    var geserAwal = 0, total = 0;
    var baru = blok.map(function (l, i) {
      if (keluar) {
        var m = /^( {1,4}|\t)/.exec(l); var d = m ? m[0].length : 0;
        if (i === 0) geserAwal = -Math.min(d, s - awal);
        total -= d;
        return l.slice(d);
      }
      if (i === 0) geserAwal = TAB.length;
      total += TAB.length;
      return TAB + l;
    }).join('\n');
    this.replace(awal, akhir, baru);
    ta.setSelectionRange(Math.max(awal, s + geserAwal), Math.max(awal, e + total));
  };

  /* Baris yang dipilih (atau baris kursor) sebagai rentang mutlak [awal, akhir) tanpa '\n' penutup. */
  Editor.prototype.rentangBaris = function () {
    var v = this.getValue(), s = this.sel(), a = s[0], b = s[1];
    var awal = v.lastIndexOf('\n', a - 1) + 1;
    var akhir = v.indexOf('\n', b > a && v[b - 1] === '\n' ? b - 1 : b);
    return { v: v, a: a, b: b, awal: awal, akhir: akhir < 0 ? v.length : akhir };
  };
  /* Alt+atas/bawah: baris terpilih bertukar tempat dengan baris di atas/bawahnya. */
  Editor.prototype.pindahBaris = function (arah) {
    var r = this.rentangBaris(), v = r.v, blok = v.slice(r.awal, r.akhir);
    if (arah < 0) {
      if (r.awal === 0) return;
      var atas = v.lastIndexOf('\n', r.awal - 2) + 1, geser = r.awal - atas;
      this.gantiMutlak(atas, r.akhir, blok + '\n' + v.slice(atas, r.awal - 1), r.a - geser, r.b - geser);
    } else {
      if (r.akhir >= v.length) return;
      var bawah = v.indexOf('\n', r.akhir + 1); if (bawah < 0) bawah = v.length;
      var isi = v.slice(r.akhir + 1, bawah), maju = isi.length + 1;
      this.gantiMutlak(r.awal, bawah, isi + '\n' + blok, r.a + maju, r.b + maju);
    }
    this.revealCaret();
  };
  /* Ctrl+Shift+K: hapus baris terpilih, kursor ke awal baris sesudahnya. */
  Editor.prototype.hapusBaris = function () {
    var r = this.rentangBaris(), v = r.v;
    var dari = r.awal, sampai = r.akhir < v.length ? r.akhir + 1 : r.akhir;
    if (sampai === r.akhir && dari > 0) dari--;
    if (dari === sampai) return;
    this.gantiMutlak(dari, sampai, '', dari === r.awal ? dari : v.lastIndexOf('\n', dari - 1) + 1);
    this.revealCaret();
  };

  Editor.prototype.toggleKomentar = function () {
    var ta = this.ta, v = ta.value, s = ta.selectionStart, e = ta.selectionEnd;
    var tanda = ({ python: '#', sh: '#', yaml: '#', toml: '#', sql: '--', html: null, css: null, md: null, plain: null })[this.lang];
    if (tanda === null) return;
    tanda = tanda || '//';
    var awal = v.lastIndexOf('\n', s - 1) + 1, akhir = v.indexOf('\n', e); if (akhir < 0) akhir = v.length;
    var blok = v.slice(awal, akhir).split('\n');
    var semua = blok.every(function (l) { return !l.trim() || l.trim().indexOf(tanda) === 0; });
    var baru = blok.map(function (l) {
      if (!l.trim()) return l;
      if (semua) return l.replace(new RegExp('^(\\s*)' + tanda.replace(/[/-]/g, '\\$&') + ' ?'), '$1');
      var ind = /^\s*/.exec(l)[0];
      return ind + tanda + ' ' + l.slice(ind.length);
    }).join('\n');
    this.replace(awal, akhir, baru, awal, awal + baru.length);
  };

  Editor.prototype.goto = function (baris, kolom) {
    var i = Math.max(0, Math.min(this.barisTeks.length - 1, baris - 1));
    var p = this.awalBaris(i), panjang = (this.barisTeks[i] || '').length;
    var q = p + Math.min(panjang, Math.max(0, (kolom || 1) - 1));
    this.selectRange(q, q);
  };
  /* Pilih offset MUTLAK a..b (seluruh berkas). */
  Editor.prototype.selectRange = function (a, b) {
    this.focus();
    this.pasangJendela(this.barisDari(a), a, b);
    this.cursorUI();
    this.revealCaret();
  };

  /* kata di sebelah kiri kursor, untuk saran (termasuk ns::) */
  Editor.prototype.prefix = function () {
    var v = this.ta.value, p = this.ta.selectionStart;
    if (this.ta.selectionEnd !== p) return null;
    var m = /[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_]?[A-Za-z0-9_]*)*$/.exec(v.slice(Math.max(0, p - 80), p));
    return m ? { teks: m[0], dari: p - m[0].length, sampai: p } : null;
  };
  /* kata utuh di bawah kursor (untuk bantuan fungsi) */
  Editor.prototype.wordAt = function () {
    var v = this.ta.value, p = this.ta.selectionStart;
    var kiri = /[A-Za-z0-9_:]*$/.exec(v.slice(Math.max(0, p - 80), p))[0];
    var kanan = /^[A-Za-z0-9_:]*/.exec(v.slice(p, p + 80))[0];
    return (kiri + kanan).replace(/^:+|:+$/g, '');
  };

  Editor.prototype.bind = function () {
    var self = this, ta = this.ta;
    /* revealCaret membaca posisi gulir; ditunda ke frame berikutnya supaya
       tidak memaksa layout tepat sesudah jendela digambar ulang */
    ta.addEventListener('input', function () {
      self.changed();
      if (!self.revealRaf) self.revealRaf = requestAnimationFrame(function () { self.revealRaf = 0; self.revealCaret(); });
    });
    ta.addEventListener('scroll', function () { if (ta.scrollTop || ta.scrollLeft) { ta.scrollTop = 0; ta.scrollLeft = 0; } });
    /* Ketukan di baris di luar jendela ketik: jendelanya dipindah ke sana SEBELUM peramban menaruh kursor. */
    /* Mouse sudah diuji-tekan sebelum peristiwa ini, jadi kursornya ditaruh di sini; sentuhan diuji saat ketukan selesai. */
    ta.addEventListener('pointerdown', function (e) {
      if (self.wrap || (!self.pre && !self.post) || e.button) return;
      var r = ta.getBoundingClientRect();
      var n = Math.max(0, Math.min(self.barisTeks.length - 1, Math.floor((e.clientY - r.top - self.padAtas()) / self.lh)));
      var nMid = self.ta.value.split('\n').length;
      if (n >= self.w0 && n < self.w0 + nMid) return;
      var s = self.sel();
      self.pasangJendela(n, s[0], s[1]);
      if (e.pointerType !== 'mouse') return;
      e.preventDefault();
      var kol = Math.max(0, Math.round((e.clientX - r.left - 12) / self.charW()));
      var q = self.awalBaris(n) + Math.min(kol, self.barisTeks[n].length);
      self.selectRange(q, q);
    });
    ta.addEventListener('compositionstart', function () { self.menyusun = true; });
    ta.addEventListener('compositionend', function () { self.menyusun = false; });
    this.scroll.addEventListener('scroll', function () {
      if (self.gulirRaf) return;
      self.gulirRaf = requestAnimationFrame(function () { self.gulirRaf = 0; self.gambarJendela(); });
    }, { passive: true });
    var cur = function () { if (!self.raf) self.raf = requestAnimationFrame(function () { self.raf = 0; self.cursorUI(); }); };
    ta.addEventListener('select', cur);
    ta.addEventListener('click', cur);
    ta.addEventListener('keyup', cur);
    ta.addEventListener('focus', function () { self.root.classList.add('is-focus'); cur(); if (self.opts.onFocus) self.opts.onFocus(true); });
    ta.addEventListener('blur', function () { self.root.classList.remove('is-focus'); if (self.opts.onFocus) self.opts.onFocus(false); });
    document.addEventListener('selectionchange', function () { if (document.activeElement === ta) cur(); });
    window.addEventListener('resize', function () { self.cw = 0; self.__ht = 0; self.__gw = 0; self.__pad = null; if (self.wrap) self.renderGutter(); if (document.activeElement === ta) setTimeout(function () { self.revealCaret(); }, 60); });
    this.gutter.addEventListener('click', function (e) {
      var s = e.target.closest('span');
      if (!s) return;
      var n = Array.prototype.indexOf.call(self.gutter.children, s) + 1 + (self.gutter.__dari || 0);
      if (self.tanda[n] && self.opts.onTanda) self.opts.onTanda(n, self.tanda[n]);
      var a = self.awalBaris(n - 1), b = Math.min(self.getValue().length, a + (self.barisTeks[n - 1] || '').length + 1);
      self.selectRange(a, b);
    });

    ta.addEventListener('keydown', function (e) {
      if (self.readOnly) return;
      var mod = e.ctrlKey || e.metaKey;
      if (e.key === 'Tab' && !e.altKey && !mod) {
        e.preventDefault();
        if (e.shiftKey || ta.selectionStart !== ta.selectionEnd) self.indentBaris(e.shiftKey);
        else self.insert(TAB);
        return;
      }
      if (mod && e.key === 'Enter' && !e.shiftKey && !e.altKey) { e.preventDefault(); if (self.opts.onRun) self.opts.onRun(); return; }
      if (mod && !e.altKey && (e.key === 'z' || e.key === 'Z')) { e.preventDefault(); if (e.shiftKey) self.redo(); else self.undo(); return; }
      if (mod && !e.altKey && (e.key === 'y' || e.key === 'Y')) { e.preventDefault(); self.redo(); return; }
      if (mod && (e.key === 'a' || e.key === 'A') && (self.pre || self.post)) {
        e.preventDefault();
        self.pasangJendela(0, 0, self.getValue().length, true);
        self.cursorUI();
        return;
      }
      if (mod && (e.key === 's' || e.key === 'S')) { e.preventDefault(); if (self.opts.onSave) self.opts.onSave(); return; }
      if (mod && e.key === '/') { e.preventDefault(); self.toggleKomentar(); return; }
      if (mod && (e.key === 'f' || e.key === 'F') && self.opts.onFind) { e.preventDefault(); self.opts.onFind(); return; }
      if (mod && (e.key === 'd' || e.key === 'D')) {
        e.preventDefault();
        var v = ta.value, p = ta.selectionStart, a = v.lastIndexOf('\n', p - 1) + 1, b = v.indexOf('\n', p); if (b < 0) b = v.length;
        self.replace(b, b, '\n' + v.slice(a, b), p + (b - a) + 1);
        return;
      }
      if (e.altKey && !mod && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) { e.preventDefault(); self.pindahBaris(e.key === 'ArrowUp' ? -1 : 1); return; }
      if (mod && e.shiftKey && (e.key === 'k' || e.key === 'K')) { e.preventDefault(); self.hapusBaris(); return; }
      if (e.key === 'Escape' && self.opts.onEscape) self.opts.onEscape();
    });

    ta.addEventListener('beforeinput', function (e) {
      if (e.inputType === 'historyUndo' || e.inputType === 'historyRedo') {
        e.preventDefault();
        if (e.inputType === 'historyUndo') self.undo(); else self.redo();
        return;
      }
      if (!self.menyusun) self.selLama = self.sel();
      if (self.readOnly || e.isComposing) return;
      var v = ta.value, s = ta.selectionStart, en = ta.selectionEnd, t = e.inputType;
      if (t === 'insertText' && e.data && e.data.length === 1) {
        var c = e.data, nx = v[en] || '';
        /* lewati penutup yang sudah ada */
        if (PENUTUP[c] && s === en && nx === c) { e.preventDefault(); ta.setSelectionRange(s + 1, s + 1); self.cursorUI(); return; }
        if (PASANG[c]) {
          var sebelum = v[s - 1] || '';
          var kutip = c === '"' || c === '`';
          if (kutip && (/\w/.test(sebelum) || sebelum === c)) return;
          if (s !== en) { e.preventDefault(); var isi = v.slice(s, en); self.replace(s, en, c + isi + PASANG[c], s + 1, en + 1); return; }
          if (!nx || /[\s)\]},;:.]/.test(nx)) { e.preventDefault(); self.replace(s, en, c + PASANG[c], s + 1); return; }
        }
        return;
      }
      if (t === 'insertLineBreak' || t === 'insertParagraph') {
        e.preventDefault();
        var awal = v.lastIndexOf('\n', s - 1) + 1;
        var ind = /^[ \t]*/.exec(v.slice(awal, s))[0];
        var kiri = v.slice(awal, s).replace(/\s+$/, '');
        var buka = /[{(\[]$/.test(kiri) || (self.lang === 'python' && /:$/.test(kiri));
        var nx2 = v[en] || '';
        if (buka && ({ '{': '}', '(': ')', '[': ']' })[kiri.slice(-1)] === nx2) {
          self.replace(s, en, '\n' + ind + TAB + '\n' + ind, s + 1 + ind.length + TAB.length);
        } else {
          self.replace(s, en, '\n' + ind + (buka ? TAB : ''));
        }
        self.revealCaret();
        return;
      }
      if (t === 'deleteContentBackward' && s === en && s > 0) {
        var pa = v[s - 1], pb = v[s];
        if (PASANG[pa] && PASANG[pa] === pb) { e.preventDefault(); self.replace(s - 1, s + 1, '', s - 1); return; }
        var awal2 = v.lastIndexOf('\n', s - 1) + 1, depan = v.slice(awal2, s);
        if (depan.length >= 4 && /^ +$/.test(depan)) {
          e.preventDefault();
          var buang = depan.length % 4 || 4;
          self.replace(s - buang, s, '', s - buang);
        }
      }
    });
  };

  M.Editor = Editor;
})(window.IDE);
