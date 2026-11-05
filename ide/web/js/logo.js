/* JRG IDE — js/logo.js
   Pembuat ikon: gambar pemakai dipotong ke satu bentuk di kanvas 512 px, lalu jadi logo IDE
   (PNG di IndexedDB, sama dengan latar) atau PNG di proyek untuk `[android] ikon`. */
(function (IDE) {
  'use strict';

  var UK = 512, BAWAAN = '/web/aset/logo.png', SUMBER_BAWAAN = '/web/aset/logo-512.jpg', KUNCI = 'logo';
  var BENTUK = [['bulat', 'Kotak bulat'], ['squircle', 'Squircle'], ['lingkaran', 'Lingkaran'], ['hex', 'Hex'],
    ['tetes', 'Tetes air'], ['daun', 'Daun'], ['berlian', 'Berlian'], ['kotak', 'Kotak']];
  var url = '';

  /* Poligon bersudut tumpul: `r` jari-jari lengkung tiap sudut (arcTo, ada di WebView lama). */
  function poli(c, titik, r) {
    var n = titik.length, a = titik[n - 1], b = titik[0];
    c.moveTo((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
    for (var i = 0; i < n; i++) {
      var p = titik[i], q = titik[(i + 1) % n];
      c.arcTo(p[0], p[1], q[0], q[1], r);
    }
    c.closePath();
  }
  function kotakBulat(c, x, y, s, r) { poli(c, [[x, y], [x + s, y], [x + s, y + s], [x, y + s]], r); }

  /* Jalur bentuk di dalam kotak (x, y, s). */
  function jalur(c, bentuk, x, y, s) {
    var cx = x + s / 2, cy = y + s / 2, i;
    c.beginPath();
    if (bentuk === 'kotak') c.rect(x, y, s, s);
    else if (bentuk === 'bulat') kotakBulat(c, x, y, s, s * 0.22);
    else if (bentuk === 'lingkaran') c.arc(cx, cy, s / 2, 0, Math.PI * 2);
    else if (bentuk === 'squircle') {
      for (i = 0; i <= 96; i++) {
        var t = i / 96 * Math.PI * 2, co = Math.cos(t), si = Math.sin(t);
        var px = cx + s / 2 * Math.sign(co) * Math.pow(Math.abs(co), 0.5), py = cy + s / 2 * Math.sign(si) * Math.pow(Math.abs(si), 0.5);
        if (i) c.lineTo(px, py); else c.moveTo(px, py);
      }
      c.closePath();
    } else if (bentuk === 'hex') {
      var h = [];
      for (i = 0; i < 6; i++) { var a = (i * 60 - 90) * Math.PI / 180; h.push([cx + s / 2 * Math.cos(a), cy + s / 2 * Math.sin(a)]); }
      poli(c, h, s * 0.07);
    } else if (bentuk === 'berlian') poli(c, [[cx, y], [x + s, cy], [cx, y + s], [x, cy]], s * 0.09);
    else if (bentuk === 'daun') {
      var R = s * 0.5, k = s * 0.08;
      c.moveTo(x + R, y); c.lineTo(x + s - k, y); c.arcTo(x + s, y, x + s, y + k, k);
      c.lineTo(x + s, y + s - R); c.arcTo(x + s, y + s, x + s - R, y + s, R);
      c.lineTo(x + k, y + s); c.arcTo(x, y + s, x, y + s - k, k);
      c.lineTo(x, y + R); c.arcTo(x, y, x + R, y, R);
      c.closePath();
    } else if (bentuk === 'tetes') {
      /* Lingkaran di bawah dan dua garis singgung dari puncaknya. */
      var Rt = s * 0.36, oy = y + s - Rt, pu = y + s * 0.02, d = oy - pu, f = Math.acos(Rt / d);
      var ka = Math.atan2(-Math.cos(f), Math.sin(f)), ki = Math.atan2(-Math.cos(f), -Math.sin(f));
      c.moveTo(cx, pu); c.lineTo(cx + Rt * Math.sin(f), oy - Rt * Math.cos(f));
      c.arc(cx, oy, Rt, ka, ki, false);
      c.closePath();
    }
  }

  /* `o`: {gambar, bentuk, zoom, geserX, geserY, ruang, tepi, warnaTepi, latar: 'transparan'|'warna'|'kabur', warna}. */
  function gambar(kanvas, o) {
    var c = kanvas.getContext('2d'), U = kanvas.width, m = U * (+o.ruang || 0) / 200, s = U - 2 * m;
    c.clearRect(0, 0, U, U);
    c.save();
    jalur(c, o.bentuk, m, m, s);
    c.clip();
    var g = o.gambar, iw = g ? g.naturalWidth || g.width : 0, ih = g ? g.naturalHeight || g.height : 0;
    if (o.latar === 'warna') { c.fillStyle = o.warna || '#0B1230'; c.fillRect(0, 0, U, U); }
    if (g && iw && o.latar === 'kabur') {
      var sk = Math.max(s / iw, s / ih) * 1.25;
      c.filter = 'blur(' + Math.round(U / 24) + 'px)';
      c.drawImage(g, U / 2 - iw * sk / 2, U / 2 - ih * sk / 2, iw * sk, ih * sk);
      c.filter = 'none';
    }
    if (g && iw) {
      var z = Math.max(s / iw, s / ih) * (+o.zoom || 100) / 100, w = iw * z, h = ih * z;
      c.drawImage(g, U / 2 - w / 2 + s * (+o.geserX || 0) / 100, U / 2 - h / 2 + s * (+o.geserY || 0) / 100, w, h);
    }
    if (+o.tepi > 0) {
      c.lineWidth = 2 * o.tepi * U / UK;
      c.strokeStyle = o.warnaTepi || '#FFFFFF';
      c.stroke();
    }
    c.restore();
    return kanvas;
  }
  function kanvas(u) { var k = document.createElement('canvas'); k.width = k.height = u || UK; return k; }
  function png(o) {
    return new Promise(function (ok, gagal) {
      gambar(kanvas(UK), o).toBlob(function (b) { if (b) ok(b); else gagal(new Error('kanvas tidak bisa jadi PNG')); }, 'image/png');
    });
  }
  function muatGambar(src) {
    return new Promise(function (ok, gagal) {
      var g = new Image();
      g.onload = function () { ok(g); };
      g.onerror = function () { gagal(new Error('gambar tidak bisa dibaca')); };
      g.src = src;
    });
  }

  /* Logo IDE: semua img.logo dan favicon mengikuti `url()`. */
  function sekarang() { return url || BAWAAN; }
  function terapkan() {
    var u = sekarang();
    Array.prototype.forEach.call(document.querySelectorAll('img.logo'), function (i) { if (i.getAttribute('src') !== u) i.src = u; });
    var l = document.querySelector('link[rel=icon]');
    if (l) l.href = u;
  }
  function pasang(blob) {
    if (url) URL.revokeObjectURL(url);
    url = blob ? URL.createObjectURL(blob) : '';
    terapkan();
  }
  function muat() {
    return IDE.latar.toko.ambil(KUNCI).then(function (b) { if (b) pasang(b); }, function () { /* logo bawaan */ });
  }
  function simpan(blob) { return IDE.latar.toko.taruh(KUNCI, blob).then(function () { pasang(blob); }); }
  function hapus() { return IDE.latar.toko.buang(KUNCI).then(function () { pasang(null); }, function () { pasang(null); }); }

  IDE.logo = { BENTUK: BENTUK, SUMBER_BAWAAN: SUMBER_BAWAAN, UK: UK, gambar: gambar, kanvas: kanvas, png: png, muatGambar: muatGambar,
    url: sekarang, khusus: function () { return !!url; }, terapkan: terapkan, muat: muat, simpan: simpan, hapus: hapus };
})(window.IDE);
