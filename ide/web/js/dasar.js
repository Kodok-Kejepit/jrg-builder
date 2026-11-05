/* JRG IDE — js/dasar.js
   Pembantu kecil yang dipakai penyunting, penyorot, dan aplikasinya. */
(function () {
  'use strict';
  var IDE = window.IDE = {};
  var U = IDE.util = {
    esc: function (s) {
      return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    },
    clamp: function (x, a, b) { return Math.max(a, Math.min(b, x)); },
    $: function (sel, root) { return (root || document).querySelector(sel); },
    $$: function (sel, root) { return Array.prototype.slice.call((root || document).querySelectorAll(sel)); }
  };
  var timer = 0;
  IDE.toast = function (teks) {
    var t = U.$('#toast');
    if (!t) return;
    t.textContent = teks;
    t.classList.add('is-on');
    clearTimeout(timer);
    timer = setTimeout(function () { t.classList.remove('is-on'); }, 2600);
  };
  /* Ikon garis 24x24, satu gaya; tidak ada emoji di antarmuka. */
  var IKON = {
    menu: '<path d="M4 7h16M4 12h16M4 17h16"/>',
    jalan: '<path d="M8.5 6.2l9.6 5.8-9.6 5.8z" fill="currentColor" stroke-width="3.4"/>',
    henti: '<rect x="6.5" y="6.5" width="11" height="11" rx="2.5" fill="currentColor"/>',
    konsol: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M12.5 15H17"/>',
    besar: '<path d="M8 4H4v4M16 4h4v4M8 20H4v-4M16 20h4v-4"/>',
    atas: '<path d="M6 15l6-6 6 6"/>',
    tambah: '<path d="M12 5v14M5 12h14"/>',
    tutup: '<path d="M6 6l12 12M18 6L6 18"/>',
    berkas: '<path d="M7 3h7l5 5v13H7z"/><path d="M14 3v5h5"/>',
    folder: '<path d="M3 7h6l2 2h10v10H3z"/>',
    lain: '<circle cx="5" cy="12" r="1.4"/><circle cx="12" cy="12" r="1.4"/><circle cx="19" cy="12" r="1.4"/>',
    cari: '<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/>',
    perangkat: '<rect x="7" y="3" width="10" height="18" rx="2"/><path d="M11 18h2"/>',
    bahasa: '<path d="M4 6h10M9 4v2c0 4-2 7-5 9M6 10c1 3 4 5 7 6"/><path d="M13 20l4-9 4 9M14.5 17h5"/>',
    buku: '<path d="M5 4h11a3 3 0 013 3v13H8a3 3 0 01-3-3z"/><path d="M5 17a3 3 0 013-3h11"/>',
    atur: '<circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1"/>',
    hapus: '<path d="M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13"/>',
    ubah: '<path d="M4 20h4l10-10-4-4L4 16z"/><path d="M13 7l4 4"/>',
    simpan: '<path d="M5 4h11l3 3v13H5z"/><path d="M8 4v6h7V4M8 20v-6h8v6"/>',
    bawah: '<path d="M6 9l6 6 6-6"/>',
    unggah: '<path d="M12 16V4M7 9l5-5 5 5"/><path d="M4 15v5h16v-5"/>',
    unduh: '<path d="M12 4v12M7 11l5 5 5-5"/><path d="M4 15v5h16v-5"/>',
    simbol: '<path d="M9 5L4 12l5 7M15 5l5 7-5 7"/>',
    masalah: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5v5.5M12 16.3v.2"/>',
    urung: '<path d="M9 14L4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 010 11H11"/>',
    ulang: '<path d="M15 14l5-5-5-5"/><path d="M20 9H9.5a5.5 5.5 0 000 11H13"/>',
    perintah: '<path d="M5 7l5 5-5 5M12 17h7"/>',
    uji: '<path d="M9 3h6M10 3v6l-5 9a2 2 0 002 3h10a2 2 0 002-3l-5-9V3"/><path d="M7.5 14h9"/>',
    gambar: '<rect x="3.5" y="4.5" width="17" height="15" rx="2.5"/><circle cx="9" cy="10" r="1.8"/><path d="M20.5 16l-5-5-8 8.5"/>',
    salin: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 00-1-1H5a1 1 0 00-1 1v10a1 1 0 001 1h3"/>'
  };
  IDE.ikon = function (nama) {
    return '<svg class="ik" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + (IKON[nama] || '') + '</svg>';
  };
})();
