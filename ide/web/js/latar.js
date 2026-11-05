/* JRG IDE — js/latar.js
   Latar dari foto atau video milik pemakai. Medianya disimpan UTUH di IndexedDB peramban (tanpa
   kompres ulang) dan diputar dari URL blob; pelayan JRG tidak ikut membawanya. */
(function (IDE) {
  'use strict';

  var DB = 'jrgide', TOKO = 'latar', KUNCI = 'media';
  var POKOK = ['--bg', '--bg-2', '--bg-3', '--c-bg', '--c-bg2'];
  var el = null, media = null, url = '', info = null;

  function db() {
    return new Promise(function (ok, gagal) {
      var r = indexedDB.open(DB, 1);
      r.onupgradeneeded = function () { r.result.createObjectStore(TOKO); };
      r.onsuccess = function () { ok(r.result); };
      r.onerror = function () { gagal(r.error || new Error('IndexedDB tidak bisa dibuka')); };
    });
  }
  function aksi(mode, kerja) {
    return db().then(function (d) {
      return new Promise(function (ok, gagal) {
        var t = d.transaction(TOKO, mode), r = kerja(t.objectStore(TOKO));
        t.oncomplete = function () { d.close(); ok(r && r.result); };
        t.onerror = t.onabort = function () { d.close(); gagal(t.error || new Error('penyimpanan latar gagal')); };
      });
    });
  }

  /* "#14171C" → "rgba(20, 23, 28, a)"; warna lain dibiarkan (sudah ada di tema). */
  function tembus(warna, a) {
    var m = /^#([0-9a-f]{6})$/i.exec(warna.trim());
    if (!m) return warna;
    var n = parseInt(m[1], 16);
    return 'rgba(' + (n >> 16) + ', ' + ((n >> 8) & 255) + ', ' + (n & 255) + ', ' + a + ')';
  }

  function siapkanEl() {
    if (el) return;
    el = document.createElement('div');
    el.className = 'latar'; el.id = 'latar'; el.setAttribute('aria-hidden', 'true');
    el.innerHTML = '<div class="latar-media"></div><div class="latar-redup"></div>';
    document.body.insertBefore(el, document.body.firstChild);
    /* Video berhenti saat aplikasi tidak terlihat: tidak ada dekode yang terbuang di latar belakang. */
    document.addEventListener('visibilitychange', function () { putar(); });
  }
  function putar() {
    if (!media || media.tagName !== 'VIDEO') return;
    if (document.hidden || !info || !info.gerak) media.pause();
    else { var p = media.play(); if (p && p.catch) p.catch(function () { /* diputar saat ketukan berikutnya */ }); }
  }

  /* `rekam`: {blob, jenis: 'gambar'|'video', nama, ukuran} atau null; `atur`: pengaturan IDE. */
  function pasang(rekam) {
    siapkanEl();
    if (url) { URL.revokeObjectURL(url); url = ''; }
    el.firstChild.innerHTML = '';
    media = null;
    if (!rekam) return;
    url = URL.createObjectURL(rekam.blob);
    if (rekam.jenis === 'video') {
      media = document.createElement('video');
      media.muted = true; media.loop = true; media.playsInline = true; media.autoplay = true;
      media.setAttribute('playsinline', ''); media.setAttribute('disablepictureinpicture', ''); media.setAttribute('disableremoteplayback', '');
      media.preload = 'auto';
    } else {
      media = document.createElement('img');
      media.decoding = 'async'; media.alt = '';
    }
    media.src = url;
    el.firstChild.appendChild(media);
  }

  function terapkan(atur) {
    siapkanEl();
    var akar = document.documentElement, app = document.getElementById('app'), ada = !!(info && media);
    akar.classList.toggle('ada-latar', ada);
    POKOK.forEach(function (k) { app.style.removeProperty(k); app.style.removeProperty(k + '-p'); });
    el.hidden = !ada;
    if (!ada) return;
    var gaya = getComputedStyle(akar), a = (100 - (+atur.latarTembus || 0)) / 100;
    POKOK.forEach(function (k) {
      var w = gaya.getPropertyValue(k).trim();
      app.style.setProperty(k + '-p', w);
      app.style.setProperty(k, tembus(w, a));
    });
    el.style.setProperty('--latar-redup', String((+atur.latarRedup || 0) / 100));
    media.style.objectFit = atur.latarIsi === 'utuh' ? 'contain' : 'cover';
    /* Kabur cuma untuk gambar: filter di atas video dihitung ulang tiap bingkai. */
    media.style.filter = info.jenis === 'gambar' && +atur.latarKabur > 0 ? 'blur(' + (+atur.latarKabur) + 'px)' : '';
    media.style.transform = info.jenis === 'gambar' && +atur.latarKabur > 0 ? 'scale(1.06)' : '';
    info.gerak = atur.latarGerak !== false;
    putar();
  }

  function muat(atur) {
    if (!window.indexedDB) return Promise.resolve(null);
    return aksi('readonly', function (s) { return s.get(KUNCI); }).then(function (r) {
      info = r ? { jenis: r.jenis, nama: r.nama, ukuran: r.ukuran } : null;
      pasang(r || null);
      terapkan(atur);
      return info;
    }).catch(function () { return null; });
  }

  /* `f`: File dari pemilih. Jawabannya info media, atau galat yang menyebut sebabnya. */
  function simpan(f, atur) {
    var jenis = /^video\//.test(f.type) ? 'video' : /^image\//.test(f.type) ? 'gambar' : '';
    if (!jenis && /\.(mp4|webm|mkv|mov|m4v|3gp)$/i.test(f.name)) jenis = 'video';
    if (!jenis && /\.(png|jpe?g|gif|webp|avif|bmp|svg)$/i.test(f.name)) jenis = 'gambar';
    if (!jenis) return Promise.reject(new Error('`' + f.name + '` bukan gambar atau video'));
    if (!window.indexedDB) return Promise.reject(new Error('peramban ini tidak punya IndexedDB, jadi latar tidak bisa disimpan'));
    var rekam = { blob: f, jenis: jenis, nama: f.name, ukuran: f.size };
    return aksi('readwrite', function (s) { return s.put(rekam, KUNCI); }).then(function () {
      info = { jenis: jenis, nama: f.name, ukuran: f.size };
      pasang(rekam);
      terapkan(atur);
      return info;
    });
  }

  function hapus(atur) {
    return aksi('readwrite', function (s) { return s.delete(KUNCI); }).catch(function () { /* sudah tidak ada */ }).then(function () {
      info = null; pasang(null); terapkan(atur);
    });
  }

  /* Toko kunci-nilai yang sama, dipakai juga oleh logo (js/logo.js). */
  var toko = {
    ambil: function (k) { return window.indexedDB ? aksi('readonly', function (s) { return s.get(k); }) : Promise.resolve(null); },
    taruh: function (k, v) { return aksi('readwrite', function (s) { return s.put(v, k); }); },
    buang: function (k) { return aksi('readwrite', function (s) { return s.delete(k); }); }
  };
  IDE.latar = { muat: muat, simpan: simpan, hapus: hapus, terapkan: terapkan, toko: toko, info: function () { return info; } };
})(window.IDE);
