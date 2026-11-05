/* JRG IDE — js/terminal.js: terminal sungguhan di panel Konsol. Tiap sesi satu shell di PTY pelayan
   (`/api/pty/...`, src/terminal.jrg); layar digambar js/vt.js. Baris tombol bawah meniru Termux.
   Rancangannya: ide/README.md, "Terminal". */
(function (IDE) {
  'use strict';
  var U = IDE.util, esc = U.esc, $ = U.$;

  /* Tata letak bawaan baris tombol, sama dengan Termux. `/` ke atas = `\`, `-` ke atas = `|`. */
  var KUNCI_BAWAAN = [['ESC', '/', '-', 'HOME', 'UP', 'END', 'PGUP'], ['TAB', 'CTRL', 'ALT', 'LEFT', 'DOWN', 'RIGHT', 'PGDN']];
  var POPUP = { '/': '\\', '-': '|' };
  var LABEL = { UP: '↑', DOWN: '↓', LEFT: '←', RIGHT: '→', PGUP: 'PgUp', PGDN: 'PgDn', HOME: 'Home', END: 'End',
    ESC: 'Esc', TAB: '⇥', CTRL: 'Ctrl', ALT: 'Alt', SHIFT: 'Shift', FN: 'Fn', DEL: 'Del', INS: 'Ins', BKSP: '⌫',
    ENTER: '⏎', KBD: '⌨', SPACE: '␣', PASTE: 'Tempel' };
  var NAMA_KEY = { UP: 'ArrowUp', DOWN: 'ArrowDown', LEFT: 'ArrowLeft', RIGHT: 'ArrowRight', HOME: 'Home', END: 'End',
    PGUP: 'PageUp', PGDN: 'PageDown', ESC: 'Escape', TAB: 'Tab', DEL: 'Delete', INS: 'Insert', BKSP: 'Backspace', ENTER: 'Enter' };
  var ULANG = { UP: 1, DOWN: 1, LEFT: 1, RIGHT: 1, PGUP: 1, PGDN: 1, DEL: 1, BKSP: 1 };
  var FN_ANGKA = { '1': 'F1', '2': 'F2', '3': 'F3', '4': 'F4', '5': 'F5', '6': 'F6', '7': 'F7', '8': 'F8', '9': 'F9', '0': 'F10', '-': 'F11', '=': 'F12' };
  var FN_PANAH = { ArrowUp: 'PageUp', ArrowDown: 'PageDown', ArrowLeft: 'Home', ArrowRight: 'End' };
  var BASIS = '  ';

  /* `o`: { api, akar (elemen), atur, simpanAtur, buka(nama), segarkan(), jalankanHalaman, info(), lembar, tutupLembar, keBaris() } */
  IDE.terminal = function (o) {
    var T = { sesi: [], aktif: -1, akar: '', putar: null, nomorPutar: 0, mod: { ctrl: 0, alt: 0, shift: 0, fn: 0 }, siap: false };
    var wadah, masuk, kunciEl, tabEl, ro;

    function atur(k, bawaan) { return o.atur[k] === undefined ? bawaan : o.atur[k]; }

    function bangun() {
      o.akar.innerHTML =
        '<div class="t-atas"><button class="tb kecil t-laci" id="t-laci" aria-label="Daftar berkas">' + IDE.ikon('menu') + '</button>' +
          '<div class="t-tab" id="t-tab" role="tablist" aria-label="Sesi terminal"></div>' +
          '<button class="tb kecil" id="t-baru" aria-label="Sesi baru" title="Sesi baru (Ctrl+Shift+T)">' + IDE.ikon('tambah') + '</button>' +
          '<button class="tb kecil" id="t-menu" aria-label="Menu terminal">' + IDE.ikon('lain') + '</button>' +
          '<button class="tb kecil" id="t-penuh" aria-label="Besarkan atau kecilkan">' + IDE.ikon('besar') + '</button>' +
          '<button class="tb kecil" id="t-turun" aria-label="Tutup panel">' + IDE.ikon('bawah') + '</button></div>' +
        '<div class="t-wadah" id="t-wadah"><button class="t-kebawah" id="t-kebawah" hidden aria-label="Ke baris terbaru">' + IDE.ikon('bawah') + '</button><input class="t-masuk" id="t-masuk" aria-label="Ketik ke terminal" autocapitalize="off" autocomplete="off" autocorrect="off" spellcheck="false" enterkeyhint="enter"></div>' +
        '<div class="t-kunci" id="t-kunci" aria-label="Tombol tambahan"></div>';
      wadah = $('#t-wadah'); masuk = $('#t-masuk'); kunciEl = $('#t-kunci'); tabEl = $('#t-tab');
      jenisMasuk();
      gambarKunci();
      $('#t-baru').onclick = function () { baru(); };
      $('#t-menu').onclick = menu;
      $('#t-laci').onclick = function () { if (o.laci) o.laci(); };
      $('#t-kebawah').onclick = function () { var s = sesiAktif(); if (s) { s.bawah = true; s.vt.kotorSemua(); } this.hidden = true; fokus(); };
      $('#t-penuh').onclick = function () { $('#tb-penuh').click(); sesuaikanNanti(); };
      $('#t-turun').onclick = function () { $('#tb-tutup-keluar').click(); };
      pasangMasuk();
      pasangSentuh();
      if (window.ResizeObserver) { ro = new ResizeObserver(function () { sesuaikanNanti(); }); ro.observe(wadah); }
      else window.addEventListener('resize', sesuaikanNanti);
    }
    /* Tanpa saran (bawaan, seperti Termux): papan ketik tidak menahan kata, tiap huruf langsung terkirim. */
    function jenisMasuk() {
      masuk.type = atur('tSaran', false) ? 'text' : 'password';
      masuk.setAttribute('autocomplete', atur('tSaran', false) ? 'off' : 'new-password');
      masuk.value = BASIS; T.lama = BASIS;
    }

    /* ---------- sesi ---------- */
    function sesiAktif() { return T.sesi[T.aktif] || null; }
    function buatTampilan(d) {
      var el = document.createElement('div');
      el.className = 't-layar';
      el.style.fontSize = atur('tHuruf', 13) + 'px';
      el.hidden = true;
      wadah.insertBefore(el, masuk);
      var s = { id: d.id, judul: d.judul || 'sh', sampai: 0, selesai: false, kode: d.kode || 0, el: el, antri: '', kirimSedang: false, dir: '' };
      s.vt = new IDE.Vt(el, {
        scrollback: atur('tSb', 3000),
        kirim: function (t) { if (!s.diam) kirim(s, t); },
        judul: function (t) { s.judul = t.slice(0, 60) || s.judul; gambarTab(); simpanJudul(s); },
        osc: function (k, isi) { osc(s, k, isi); },
        bel: function () { if (!s.diam) bel(s); },
        ubahLayar: function () { s.bawah = true; },
        digambar: function () {
          if (s.bawah !== false) { el.scrollTop = el.scrollHeight; s.posGulir = el.scrollTop; }
          if (T.sesi[T.aktif] === s) letakkanMasuk();
        }
      });
      /* Yang dianggap "pemakai menggulir ke atas" cuma posisi yang lebih tinggi dari yang terakhir kami
         pasang; scrollback yang bertambah di atas tidak memutus ikut-ke-bawah. */
      el.addEventListener('scroll', function () {
        if (el.hidden) return;
        var dekat = el.scrollHeight - el.scrollTop - el.clientHeight < (s.vt.tinggiSel || 16) * 1.5 + 2;
        if (dekat) s.bawah = true;
        else if (el.scrollTop < (s.posGulir || 0) - 2) s.bawah = false;
        if (T.sesi[T.aktif] === s) $('#t-kebawah').hidden = s.bawah !== false;
      }, { passive: true });
      el.addEventListener('click', function (e) { ketuk(s, e); });
      el.addEventListener('wheel', function (e) { roda(s, e); }, { passive: false });
      T.sesi.push(s);
      return s;
    }
    var timerJudul = 0;
    function simpanJudul(s) {
      clearTimeout(timerJudul);
      timerJudul = setTimeout(function () { o.api('/api/pty/judul', { id: s.id, judul: s.judul }).catch(function () {}); }, 800);
    }
    function pilihSesi(i) {
      if (!T.sesi[i]) return;
      T.aktif = i;
      T.sesi.forEach(function (s, j) { s.el.hidden = j !== i; });
      T.sesi[i].bawah = true;
      $('#t-kebawah').hidden = true;
      o.atur.tSesi = T.sesi[i].id; o.simpanAtur();
      gambarTab();
      sesuaikan(true);
      var s = T.sesi[i];
      s.vt.kotorSemua();
    }
    /* Notifikasi latar seperti Termux ("2 sesi berjalan", Keluar, Wake lock) lewat `JRGAndroid.latar` (izin
       `latar` jrgm) atau `JRGLatar` (layanan sisipan versi Terminal); tanpa sesi hidup notifikasinya dihentikan. */
    function pintuLatar() {
      /* Versi Play tidak membawa izin `latar`: pilihannya tidak ditampilkan. */
      if (o.info() && o.info().varian === 'play') return null;
      if (window.JRGAndroid && JRGAndroid.latar) return JRGAndroid;
      return window.JRGLatar && JRGLatar.latar ? JRGLatar : null;
    }
    function perbaruiLatar() {
      var A = pintuLatar();
      if (!A) return;
      var n = T.sesi.filter(function (s) { return !s.selesai; }).length;
      var t = n && atur('tLatar', true) !== false ? n + ' sesi berjalan' : '';
      if (t === T.latar) return;
      T.latar = t;
      var r;
      try { r = A.latar(t); } catch (e) { r = 'gagal'; }
      /* Pelayan perlu tahu: Keluar di notifikasi jrgm cuma menghentikan penjaganya, pelayan menyusul sendiri. */
      o.api('/api/latar', { aktif: !!t && (r === undefined || r === 'ok') }).catch(function () {});
    }
    function gambarTab() {
      perbaruiLatar();
      if (!tabEl) return;
      tabEl.innerHTML = T.sesi.map(function (s, i) {
        return '<button role="tab" data-i="' + i + '" aria-selected="' + (i === T.aktif) + '"' + (s.selesai ? ' class="usai"' : '') + '>' +
          '<span>' + (i + 1) + ' · ' + esc(s.judul) + '</span><span class="x" data-x="' + i + '" aria-label="Tutup sesi">×</span></button>';
      }).join('');
      U.$$('#t-tab [data-i]').forEach(function (b) {
        b.onclick = function (e) {
          var x = e.target.closest('[data-x]');
          if (x) tutup(T.sesi[+x.getAttribute('data-x')]); else { pilihSesi(+b.getAttribute('data-i')); fokus(); }
        };
      });
      var a = tabEl.children[T.aktif];
      if (a && a.scrollIntoView) a.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
    /* Linux (Ubuntu, versi Terminal) terpasang: sesi baru mengikuti pilihan "Sesi baru" kalau `linux` tidak disebut. */
    function adaLinux() { var l = o.info() && o.info().linux; return !!(l && l.terpasang); }
    function baru(dir, linux) {
      var u = ukuranPerkiraan();
      if (linux === undefined) linux = adaLinux() && atur('tLinux', false) === true;
      return o.api('/api/pty/buka', { kolom: u.kolom, baris: u.baris, dir: dir || '', linux: !!linux }).then(function (d) {
        T.akar = d.akar || T.akar;
        var s = buatTampilan({ id: d.id, judul: linux ? 'ubuntu' : 'sh' });
        pilihSesi(T.sesi.length - 1);
        mulaiPutar();
        fokus();
        return s;
      }, function (e) {
        IDE.toast('Terminal tidak bisa dibuka: ' + (e.message || e));
        if (!T.sesi.length && o.keBaris) o.keBaris();
        throw e;
      });
    }
    function tutup(s) {
      if (!s) return;
      var i = T.sesi.indexOf(s);
      o.api('/api/pty/tutup', { id: s.id }).catch(function () {});
      s.el.remove();
      T.sesi.splice(i, 1);
      if (T.aktif >= T.sesi.length) T.aktif = T.sesi.length - 1;
      if (T.sesi.length) pilihSesi(T.aktif); else { gambarTab(); T.aktif = -1; }
      mulaiPutar();
    }
    function ukuranPerkiraan() {
      var s = sesiAktif();
      if (s) return { kolom: s.vt.kolom, baris: s.vt.baris };
      var uji = document.createElement('div');
      uji.className = 't-layar'; uji.style.fontSize = atur('tHuruf', 13) + 'px';
      wadah.insertBefore(uji, masuk);
      var vt = new IDE.Vt(uji, { kirim: function () {} }), u = vt.ukurSel();
      uji.remove();
      return u;
    }

    /* ---------- keluaran: satu long-poll untuk semua sesi ---------- */
    function mulaiPutar() {
      if (T.putar) T.putar.abort();
      T.putar = null;
      var hidup = T.sesi.filter(function (s) { return !s.selesai; });
      if (!hidup.length) return;
      var nomor = ++T.nomorPutar, ac = window.AbortController ? new AbortController() : null;
      T.putar = ac || { abort: function () {} };
      fetch('/api/pty/baca', { method: 'POST', body: JSON.stringify({ sesi: hidup.map(function (s) { return [s.id, s.sampai]; }), ms: 20000 }), signal: ac && ac.signal })
        .then(function (r) { return r.json(); })
        .then(function (d) {
          if (nomor !== T.nomorPutar) return;
          T.sesi.forEach(function (s) { var h = d[String(s.id)]; if (h) terima(s, h); });
          mulaiPutar();
        }, function (e) {
          if (nomor !== T.nomorPutar || (e && e.name === 'AbortError')) return;
          setTimeout(function () { if (nomor === T.nomorPutar) mulaiPutar(); }, 1200);
        });
    }
    function terima(s, h) {
      if (h.hilang) { selesai(s, null); return; }
      if (h.data) {
        if (h.potong && s.sampai === 0) s.vt.tulis('\x1b[2m[keluaran lama sebagian sudah dibuang]\x1b[0m\r\n');
        /* Sesi yang dipulihkan: keluaran lamanya digambar ulang tanpa menjalankan lagi perintah IDE,
           bel, papan klip, atau jawaban terminal di dalamnya (dulu `bahasa` mengetik "0" ke shell). */
        s.diam = !!s.pulih;
        try { s.vt.tulis(h.data); } finally { s.diam = false; }
      }
      s.pulih = false;
      s.sampai = h.sampai;
      if (h.judul && s.judul === 'sh') s.judul = h.judul;
      if (h.selesai) selesai(s, h.kode);
    }
    function selesai(s, kode) {
      if (s.selesai) return;
      s.selesai = true; s.kode = kode;
      s.vt.tulis('\r\n\x1b[0;2m[' + (kode === null ? 'sesi hilang: pelayan dimulai ulang' : 'proses selesai, kode ' + kode) + ' — tekan Enter untuk menutup]\x1b[0m');
      gambarTab();
    }

    /* ---------- masukan ke PTY ---------- */
    function kirim(s, t) {
      if (!s || !t) return;
      if (s.selesai) { if (/\r/.test(t)) tutup(s); return; }
      if (s.halaman) { if (t.indexOf('\x03') >= 0) s.halaman.hentikan(); return; }
      s.antri += t;
      if (s.bawah === false) { s.bawah = true; s.vt.minta(); $('#t-kebawah').hidden = true; }
      kirimAntri(s);
    }
    function kirimAntri(s) {
      if (s.kirimSedang || !s.antri) return;
      var t = s.antri.replace(/\x00/g, '');
      s.antri = '';
      s.kirimSedang = true;
      o.api('/api/pty/tulis', { id: s.id, teks: t }).catch(function (e) { IDE.toast(String(e.message || e)); })
        .then(function () { s.kirimSedang = false; kirimAntri(s); });
    }
    /* Pengubah satu-ketuk dari baris tombol (Ctrl, Alt, Fn): 1 sekali pakai, 2 terkunci. */
    function pakaiMod() {
      var m = { ctrl: !!T.mod.ctrl, alt: !!T.mod.alt, shift: !!T.mod.shift, fn: !!T.mod.fn };
      ['ctrl', 'alt', 'shift', 'fn'].forEach(function (k) { if (T.mod[k] === 1) T.mod[k] = 0; });
      gambarMod();
      return m;
    }
    function adaMod() { return T.mod.ctrl || T.mod.alt || T.mod.shift || T.mod.fn; }
    function tombol(key, m) {
      var s = sesiAktif();
      if (!s) return;
      if (m.fn && FN_ANGKA[key]) key = FN_ANGKA[key];
      if (m.fn && FN_PANAH[key]) key = FN_PANAH[key];
      kirim(s, s.vt.urutanTombol(key, m));
    }
    function teksMasuk(t, tempel) {
      var s = sesiAktif();
      if (!s || !t) return;
      t = t.replace(/\r?\n/g, '\r');
      if (adaMod()) {
        var cp = String.fromCodePoint(t.codePointAt(0)), sisa = t.slice(cp.length);
        tombol(cp, pakaiMod());
        t = sisa;
        if (!t) return;
      }
      if (tempel && s.vt.mode.tempel) t = '\x1b[200~' + t + '\x1b[201~';
      kirim(s, t);
    }
    function pasangMasuk() {
      masuk.addEventListener('keydown', function (e) {
        if (e.isComposing || e.keyCode === 229) return;
        var k = e.key;
        if (e.ctrlKey && e.shiftKey && (k === 'C' || k === 'c')) { e.preventDefault(); salinPilihan(); return; }
        if (e.ctrlKey && e.shiftKey && (k === 'V' || k === 'v')) { e.preventDefault(); tempel(); return; }
        if (e.ctrlKey && e.shiftKey && (k === 'T' || k === 't')) { e.preventDefault(); baru(); return; }
        if (e.ctrlKey && (k === '=' || k === '+')) { e.preventDefault(); ubahHuruf(1); return; }
        if (e.ctrlKey && k === '-') { e.preventDefault(); ubahHuruf(-1); return; }
        if (e.shiftKey && (k === 'PageUp' || k === 'PageDown') && sesiAktif()) {
          e.preventDefault(); var el = sesiAktif().el; el.scrollTop += (k === 'PageUp' ? -1 : 1) * el.clientHeight * 0.9; return;
        }
        var khusus = k.length > 1 && k !== 'Unidentified' && k !== 'Dead' && k !== 'Process' && !/^(Shift|Control|Alt|Meta|CapsLock|AltGraph)$/.test(k);
        var denganMod = k.length <= 2 && (e.ctrlKey || e.altKey) && !e.metaKey;
        if (!khusus && !denganMod) return;
        e.preventDefault();
        var m = pakaiMod();
        tombol(k, { ctrl: e.ctrlKey || m.ctrl, alt: e.altKey || m.alt, shift: e.shiftKey || m.shift, fn: m.fn });
        masuk.value = BASIS; T.lama = BASIS;
      });
      masuk.addEventListener('compositionstart', function () { T.susun = true; });
      masuk.addEventListener('compositionend', function () { T.susun = false; setTimeout(function () { if (!T.susun) { selisih(); kosongkan(); } }, 0); });
      masuk.addEventListener('input', function (e) {
        if (e.inputType === 'insertLineBreak') { teksMasuk('\r'); kosongkan(); return; }
        selisih(e.inputType === 'insertFromPaste');
        if (!T.susun && !e.isComposing) kosongkan();
      });
      masuk.addEventListener('focus', function () { o.akar.classList.add('fokus'); var s = sesiAktif(); if (s) { s.vt.sembunyiKursor = false; s.vt.tandai(s.vt.y); } });
      masuk.addEventListener('blur', function () { o.akar.classList.remove('fokus'); });
      /* Tempelan dari papan ketik HP datang tanpa event paste, dan `<input>` membuang baris barunya
         sebelum `input`; di `beforeinput` teksnya masih utuh. */
      masuk.addEventListener('beforeinput', function (e) {
        if (!/^insert(Text|FromPaste|ReplacementText)$/.test(e.inputType)) return;
        var t = e.data || (e.dataTransfer && e.dataTransfer.getData('text/plain')) || '';
        if (!/[\r\n]/.test(t)) return;
        e.preventDefault(); teksMasuk(t, true); kosongkan();
      });
      masuk.addEventListener('paste', function (e) {
        var t = e.clipboardData && e.clipboardData.getData('text');
        if (t) { e.preventDefault(); teksMasuk(t, true); }
      });
    }
    /* Isian disimpan berisi BASIS; yang berubah darinya (ketikan, hapus, ganti oleh koreksi) jadi urutan untuk PTY. */
    function selisih(tempel) {
      var a = T.lama, b = masuk.value, i = 0;
      while (i < a.length && i < b.length && a.charCodeAt(i) === b.charCodeAt(i)) i++;
      var hapus = Array.from(a.slice(i)).length, tambah = b.slice(i);
      T.lama = b;
      var s = sesiAktif();
      if (!s) return;
      if (hapus) { var m = pakaiMod(); for (var j = 0; j < hapus; j++) kirim(s, s.vt.urutanTombol('Backspace', m)); }
      if (tambah) teksMasuk(tambah, tempel && tambah.length > 1);
    }
    function kosongkan() {
      if (masuk.value !== BASIS) { masuk.value = BASIS; }
      T.lama = BASIS;
      try { masuk.setSelectionRange(BASIS.length, BASIS.length); } catch (e) { /* jenis password */ }
    }
    function letakkanMasuk() {
      var s = sesiAktif();
      if (!s || !s.vt.lebarSel) return;
      var tk = s.el.querySelector('.vkur');
      if (!tk) return;
      var r = tk.getBoundingClientRect(), w = wadah.getBoundingClientRect();
      masuk.style.left = Math.max(0, r.left - w.left) + 'px';
      masuk.style.top = Math.max(0, r.top - w.top) + 'px';
    }

    /* ---------- baris tombol ---------- */
    function tataKunci() {
      var t = atur('tKunci', '');
      if (t) { try { var x = JSON.parse(t); if (Array.isArray(x) && x.length) return x.map(function (r) { return Array.isArray(r) ? r : [r]; }); } catch (e) { /* bawaan */ } }
      return KUNCI_BAWAAN;
    }
    function gambarKunci() {
      kunciEl.hidden = !atur('tTombol', true);
      kunciEl.innerHTML = tataKunci().map(function (r) {
        return '<div class="t-deret">' + r.map(function (k) {
          var kk = String(k), lbl = LABEL[kk] || kk, p = POPUP[kk];
          return '<button tabindex="-1" data-kunci="' + esc(kk) + '"' + (p ? ' data-pop="' + esc(p) + '"' : '') + (/^(CTRL|ALT|SHIFT|FN)$/.test(kk) ? ' class="mod"' : '') + '>' +
            esc(lbl) + (p ? '<i>' + esc(LABEL[p] || p) + '</i>' : '') + '</button>';
        }).join('') + '</div>';
      }).join('');
      gambarMod();
    }
    function gambarMod() {
      U.$$('#t-kunci .mod').forEach(function (b) {
        var v = T.mod[b.getAttribute('data-kunci').toLowerCase()] || 0;
        b.setAttribute('data-n', v);
      });
    }
    function tekanKunci(k) {
      var mod = k.toLowerCase();
      if (/^(ctrl|alt|shift|fn)$/.test(mod)) {
        var kini = Date.now(), v = T.mod[mod];
        T.mod[mod] = v === 2 ? 0 : v === 1 && kini - (T.ketukMod || 0) < 350 && T.modTerakhir === mod ? 2 : v ? 0 : 1;
        T.ketukMod = kini; T.modTerakhir = mod;
        gambarMod();
        return;
      }
      if (k === 'KBD') { if (document.activeElement === masuk) masuk.blur(); else fokus(); return; }
      if (k === 'PASTE') { tempel(); return; }
      var m = pakaiMod();
      if (NAMA_KEY[k]) { tombol(NAMA_KEY[k], m); return; }
      if (/^F([1-9]|1[0-2])$/.test(k)) { tombol(k, m); return; }
      if (k === 'SPACE') k = ' ';
      var s = sesiAktif();
      if (!s) return;
      if (m.ctrl || m.alt || m.fn) tombol(k, m); else kirim(s, k);
    }
    (function () {
      var tekan = null, ulang = 0, tunda = 0;
      function henti() { clearTimeout(tunda); clearInterval(ulang); tunda = ulang = 0; }
      document.addEventListener('pointerdown', function (e) {
        var b = e.target.closest && e.target.closest('#t-kunci button');
        if (!b) return;
        e.preventDefault();
        tekan = { b: b, y: e.clientY, x: e.clientX, ulang: false };
        b.classList.add('ditekan');
        var k = b.getAttribute('data-kunci');
        if (ULANG[k]) tunda = setTimeout(function () { tekan.ulang = true; tekanKunci(k); ulang = setInterval(function () { tekanKunci(k); }, 55); }, 380);
      });
      document.addEventListener('pointermove', function (e) {
        if (tekan && Math.abs(e.clientX - tekan.x) > 24) { henti(); tekan.b.classList.remove('ditekan'); tekan = null; }
      });
      function lepas(e, batal) {
        if (!tekan) return;
        var t = tekan;
        tekan = null; henti();
        t.b.classList.remove('ditekan');
        if (batal || t.ulang) return;
        var k = t.b.getAttribute('data-kunci'), pop = t.b.getAttribute('data-pop');
        if (pop && t.y - e.clientY > 18) k = pop;
        tekanKunci(k);
        if (navigator.vibrate && atur('tGetarTombol', true)) try { navigator.vibrate(8); } catch (x) { /* tidak didukung */ }
      }
      document.addEventListener('pointerup', function (e) { lepas(e, false); });
      document.addEventListener('pointercancel', function (e) { lepas(e, true); });
    })();

    /* ---------- sentuh, tetikus, cubit ---------- */
    function sel(s, e) {
      var r = s.el.querySelector('.t-scr').getBoundingClientRect();
      return { x: Math.floor((e.clientX - r.left) / s.vt.lebarSel), y: Math.floor((e.clientY - r.top) / s.vt.tinggiSel) };
    }
    function ketuk(s, e) {
      if (String(window.getSelection && window.getSelection()) !== '') return;
      var a = e.target.closest('a[href]');
      if (a) return;
      if (s.vt.mode.tetikus) {
        var c = sel(s, e);
        kirim(s, s.vt.urutanTetikus('tekan', 0, c.x, c.y) + s.vt.urutanTetikus('lepas', 0, c.x, c.y));
      }
      fokus();
    }
    function roda(s, e) {
      var alt = s.vt.buf !== s.vt.utama;
      if (!alt && !s.vt.mode.tetikus) return;
      e.preventDefault();
      var n = Math.max(1, Math.min(10, Math.round(Math.abs(e.deltaY) / 40))), atas = e.deltaY < 0, c = sel(s, e), i;
      if (s.vt.mode.tetikus) for (i = 0; i < n; i++) kirim(s, s.vt.urutanTetikus('roda', atas ? 0 : 1, c.x, c.y));
      else for (i = 0; i < n; i++) kirim(s, s.vt.urutanTombol(atas ? 'ArrowUp' : 'ArrowDown', {}));
    }
    function pasangSentuh() {
      var awal = null;
      wadah.addEventListener('touchstart', function (e) {
        var s = sesiAktif();
        if (!s) return;
        if (e.touches.length === 2) {
          awal = { cubit: jarak(e.touches), huruf: atur('tHuruf', 13) };
        } else if (e.touches.length === 1) {
          awal = { y: e.touches[0].clientY, sisa: 0, alt: s.vt.buf !== s.vt.utama };
        }
      }, { passive: true });
      wadah.addEventListener('touchmove', function (e) {
        var s = sesiAktif();
        if (!s || !awal) return;
        if (awal.cubit && e.touches.length === 2) {
          e.preventDefault();
          var h = Math.round(awal.huruf * jarak(e.touches) / awal.cubit);
          h = Math.max(7, Math.min(32, h));
          if (h !== atur('tHuruf', 13)) setHuruf(h, true);
          return;
        }
        if (!awal.alt || e.touches.length !== 1) return;
        /* Di layar alternatif (less, vi, htop) geser menggulir programnya, bukan halaman. */
        e.preventDefault();
        var dy = e.touches[0].clientY - awal.y + awal.sisa, langkah = s.vt.tinggiSel || 16, n = Math.trunc(dy / langkah);
        if (!n) return;
        awal.y = e.touches[0].clientY; awal.sisa = dy - n * langkah;
        var c = { x: 0, y: 0 }, i;
        for (i = 0; i < Math.abs(n); i++) {
          if (s.vt.mode.tetikus) kirim(s, s.vt.urutanTetikus('roda', n > 0 ? 0 : 1, c.x, c.y));
          else kirim(s, s.vt.urutanTombol(n > 0 ? 'ArrowUp' : 'ArrowDown', {}));
        }
      }, { passive: false });
      wadah.addEventListener('touchend', function () { if (awal && awal.cubit) { simpanHuruf(); } awal = null; });
    }
    function jarak(t) { var dx = t[0].clientX - t[1].clientX, dy = t[0].clientY - t[1].clientY; return Math.sqrt(dx * dx + dy * dy) || 1; }

    /* ---------- ukuran dan huruf ---------- */
    var timerUkur = 0, timerKirimUkur = 0;
    function sesuaikanNanti() { clearTimeout(timerUkur); timerUkur = setTimeout(function () { sesuaikan(false); }, 60); }
    function sesuaikan(segera) {
      var s = sesiAktif();
      if (!s || s.el.hidden || !s.el.clientWidth) return;
      var u = s.vt.ukurSel();
      if (s.vt.ubahUkuran(u.kolom, u.baris) || segera) {
        clearTimeout(timerKirimUkur);
        var kirimUkur = function () { if (!s.selesai) o.api('/api/pty/ukuran', { id: s.id, kolom: s.vt.kolom, baris: s.vt.baris }).catch(function () {}); };
        if (segera) kirimUkur(); else timerKirimUkur = setTimeout(kirimUkur, 150);
      }
    }
    function setHuruf(h, sementara) {
      o.atur.tHuruf = h;
      T.sesi.forEach(function (s) { s.el.style.fontSize = h + 'px'; });
      sesuaikanNanti();
      if (!sementara) simpanHuruf();
    }
    function simpanHuruf() { o.simpanAtur(); T.sesi.forEach(function (s) { s.vt.kotorSemua(); }); }
    function ubahHuruf(d) { setHuruf(Math.max(7, Math.min(32, atur('tHuruf', 13) + d)), false); }

    /* ---------- salin, tempel, bel ---------- */
    function salinTeks(t) {
      if (!t) return;
      var jadi = function () { IDE.toast('Tersalin'); };
      if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(t).then(jadi, function () { salinCadangan(t); jadi(); });
      else { salinCadangan(t); jadi(); }
    }
    function salinCadangan(t) {
      var ta = document.createElement('textarea');
      ta.value = t; ta.style.position = 'fixed'; ta.style.opacity = '0';
      document.body.appendChild(ta); ta.select();
      try { document.execCommand('copy'); } catch (e) { /* tidak didukung */ }
      ta.remove();
    }
    function salinPilihan() {
      var t = String(window.getSelection ? window.getSelection() : '');
      if (t) salinTeks(t.replace(/ /g, ' '));
      else IDE.toast('Pilih teksnya dulu (tekan lama)');
    }
    function tempel() {
      if (navigator.clipboard && navigator.clipboard.readText) {
        navigator.clipboard.readText().then(function (t) { if (t) { teksMasuk(t, true); fokus(); } else lembarTempel(); }, lembarTempel);
      } else lembarTempel();
    }
    function lembarTempel() {
      o.lembar('Tempel ke terminal', '<textarea class="isian" id="tt-isi" rows="6" placeholder="Tempel teksnya di sini" autocapitalize="off" spellcheck="false"></textarea>' +
        '<div class="deret"><button class="tombol utama" id="tt-ok">Kirim</button></div>');
      setTimeout(function () { $('#tt-isi').focus(); }, 200);
      $('#tt-ok').onclick = function () { var t = $('#tt-isi').value; o.tutupLembar(); if (t) teksMasuk(t, true); fokus(); };
    }
    function bel(s) {
      var cara = atur('tBel', 'getar');
      if (cara === 'mati') return;
      if (cara === 'getar') {
        try { if (window.JRGAndroid && JRGAndroid.getar) JRGAndroid.getar('40'); else if (navigator.vibrate) navigator.vibrate(40); } catch (e) { /* tidak didukung */ }
      }
      s.el.classList.remove('bel'); void s.el.offsetWidth; s.el.classList.add('bel');
    }

    /* ---------- OSC: judul, direktori, papan klip, perintah IDE ---------- */
    function relatif(mutlak) {
      if (T.akar && mutlak.indexOf(T.akar + '/') === 0) return mutlak.slice(T.akar.length + 1);
      return null;
    }
    function osc(s, k, isi) {
      if (k === '7') { s.dir = isi.replace(/^file:\/\/[^/]*/, ''); return; }
      if (k === '52' && !s.diam) {
        var b = isi.split(';')[1];
        if (b && b !== '?') { try { salinTeks(decodeURIComponent(escape(atob(b)))); } catch (e) { /* bukan base64 */ } }
        return;
      }
      if (k !== '7770' || s.diam) return;
      var p = isi.split(';'), cmd = p[0], arg = p.slice(1).join(';');
      if (cmd === 'buka') {
        var r = relatif(arg);
        if (r) o.buka(r); else s.vt.tulis('\x1b[33mbuka: cuma berkas di dalam folder berkas IDE (' + T.akar + ')\x1b[0m\r\n');
        return;
      }
      perintahIde(s, cmd, arg);
    }
    /* Perintah yang dikerjakan halaman (pasang, bahasa, jalankan bahasa-halaman). Shell menunggu dengan
       `read`; jawabannya kode keluar yang dikirim sebagai satu baris. */
    function perintahIde(s, cmd, arg) {
      function cetak(t) { s.vt.tulis(t.replace(/\r?\n/g, '\r\n')); }
      function akhiri(kode) { s.halaman = null; kirim(s, String(kode) + '\r'); }
      var P = IDE.paket;
      if (cmd === 'bahasa') {
        var baris = o.info().bahasa.map(function (b) {
          return '  ' + b.label + (b.program ? '  \x1b[32mada\x1b[0m' : P.untuk(b.ekst[0]) ? '' : '  \x1b[2m' + b.sebab + '\x1b[0m');
        }).filter(function (x) { return /  /.test(x.trim()) || /\x1b/.test(x); });
        Object.keys(P.daftar).forEach(function (id) {
          baris.push('  ' + P.daftar[id].label + (P.terpasang(id) ? '  \x1b[32mterpasang\x1b[0m' : '  \x1b[2mpasang ' + id + ' (' + P.daftar[id].ukuran + ')\x1b[0m'));
        });
        var belum = P.belumDi(o.info());
        if (belum.length) baris.push('  \x1b[2mBelum bisa: ' + belum.map(function (b) { return b.label; }).join(', ') + '\x1b[0m');
        cetak(baris.join('\n') + '\n');
        akhiri(0);
        return;
      }
      if (cmd === 'pasang' || cmd === 'copot') {
        var id = cariPaket(arg);
        if (!arg) { cetak('pakai: ' + cmd + ' <bahasa>; daftarnya: bahasa\n'); akhiri(2); return; }
        if (!id) { cetak(cmd + ': bahasa `' + arg + '` tidak dikenal di sini. Ketik: bahasa\n'); akhiri(1); return; }
        if (cmd === 'copot') { P.copot(id).then(function () { cetak(P.daftar[id].label + ' dicopot.\n'); akhiri(0); }, function (e) { cetak(String(e.message || e) + '\n'); akhiri(1); }); return; }
        if (P.terpasang(id)) { cetak(P.daftar[id].label + ' sudah terpasang.\n'); akhiri(0); return; }
        var kini = '';
        s.halaman = { hentikan: function () {} };
        P.pasang(id, function (t) { var a = t.replace(/ \d+%$/, ''); if (a !== kini) { kini = a; cetak(a + '\n'); } }, o.info().cermin_paket).then(function () {
          cetak(P.daftar[id].label + ' terpasang.\n'); o.segarkan(); akhiri(0);
        }, function (e) { cetak('\x1b[31m' + String(e.message || e) + '\x1b[0m\n'); akhiri(1); });
        return;
      }
      /* `setup-storage` (Ubuntu dan shell Android): dialog izin Android, jawabannya 0 diberi, 1 ditolak, 2 bukan Android. */
      if (cmd === 'izin') {
        /* Penyimpanan lewat JRGLatar kalau ada: baca DAN tulis dalam satu dialog. */
        var A = arg === 'penyimpanan' && window.JRGLatar && JRGLatar.mintaIzin ? JRGLatar : window.JRGAndroid, kini = A ? A.izin(arg) : '';
        if (!A) { cetak('Izin cuma ada di aplikasi Android.\n'); akhiri(2); return; }
        if (kini === 'diberi') { akhiri(0); return; }
        if (kini !== 'belum') { cetak('Izin ' + arg + ': ' + (kini === 'tidak_di_manifes' ? 'tidak ada di versi ini' : kini) + '\n'); akhiri(1); return; }
        cetak('Izinkan akses ' + arg + ' di dialog Android…\n');
        var dengar = function (e) {
          if (!e.detail || e.detail.nama !== arg) return;
          window.removeEventListener('jrg-izin', dengar);
          akhiri(e.detail.diberi ? 0 : 1);
        };
        window.addEventListener('jrg-izin', dengar);
        s.halaman = { hentikan: function () { window.removeEventListener('jrg-izin', dengar); akhiri(130); } };
        if (A.mintaIzin(arg) !== 'ditanyakan') { window.removeEventListener('jrg-izin', dengar); akhiri(A.izin(arg) === 'diberi' ? 0 : 1); }
        return;
      }
      if (cmd === 'jalankan') {
        var r = relatif(arg);
        if (!r) { cetak('cuma berkas di dalam folder berkas IDE yang bisa dijalankan bahasa-halaman\n'); akhiri(2); return; }
        var h = o.jalankanHalaman(r, function (x) { if (x.keluaran) cetak(x.keluaran); if (x.galat) cetak('\x1b[31m' + x.galat + '\x1b[0m'); }, function () {});
        s.halaman = h;
        h.janji.then(function (x) { akhiri(x.dihentikan ? 130 : x.sukses ? 0 : (x.kode || 1)); }, function (e) { cetak('\x1b[31m' + String(e.message || e) + '\x1b[0m\n'); akhiri(1); });
        return;
      }
      akhiri(127);
    }
    function cariPaket(kata) {
      var P = IDE.paket, k = (kata || '').toLowerCase();
      for (var id in P.daftar) if (id === k || (' ' + P.daftar[id].alias + ' ').indexOf(' ' + k + ' ') >= 0) return id;
      return null;
    }

    /* ---------- menu ---------- */
    function menu() {
      var s = sesiAktif();
      o.lembar('Terminal',
        '<div class="kisi">' +
          '<button class="tombol" id="tm-baru">Sesi baru</button><button class="tombol" id="tm-tempel">Tempel</button>' +
          '<button class="tombol" id="tm-salin">Salin pilihan</button><button class="tombol" id="tm-salin-semua">Salin semua</button>' +
          '<button class="tombol" id="tm-besar">Huruf +</button><button class="tombol" id="tm-kecil">Huruf −</button>' +
          '<button class="tombol" id="tm-cc">Kirim Ctrl+C</button><button class="tombol" id="tm-cd">Kirim Ctrl+D</button>' +
          '<button class="tombol" id="tm-reset">Atur ulang layar</button><button class="tombol" id="tm-tutup">Tutup sesi</button>' +
          '<button class="tombol" id="tm-jalan">Jalankan berkas</button><button class="tombol" id="tm-ide">Menu IDE</button>' +
          (adaLinux() ? '<button class="tombol" id="tm-ubuntu">Sesi Ubuntu</button><button class="tombol" id="tm-android">Sesi Android</button>' : '') +
        '</div>' +
        (adaLinux() ? '<div class="geser"><span>Sesi baru</span>' + pilihan('tLinux', [[false, 'Android'], [true, 'Ubuntu']]) + '</div>' : '') +
        (pintuLatar() ? '<div class="geser"><span>Jalan di latar</span>' + pilihan('tLatar', [[true, 'Ya'], [false, 'Tidak']]) + '</div>' : '') +
        '<div class="geser"><span>Baris tombol</span>' + pilihan('tTombol', [[true, 'Tampil'], [false, 'Sembunyi']]) + '</div>' +
        '<div class="geser"><span>Papan ketik</span>' + pilihan('tSaran', [[false, 'Langsung'], [true, 'Dengan saran']]) + '</div>' +
        '<div class="geser"><span>Bel</span>' + pilihan('tBel', [['getar', 'Getar'], ['kedip', 'Kedip'], ['mati', 'Mati']]) + '</div>' +
        '<div class="geser"><span>Layar penuh di HP</span>' + pilihan('tPenuh', [[true, 'Ya'], [false, 'Tidak']]) + '</div>' +
        '<div class="geser"><span>Baris diingat</span>' + pilihan('tSb', [[1000, '1000'], [3000, '3000'], [10000, '10000']]) + '</div>' +
        '<p class="subjudul">Tata letak tombol</p>' +
        '<textarea class="isian mono" id="tm-kunci" rows="3" spellcheck="false" autocapitalize="off">' + esc(JSON.stringify(tataKunci())) + '</textarea>' +
        '<div class="deret"><button class="tombol" id="tm-kunci-bawaan">Bawaan</button><button class="tombol utama" id="tm-kunci-ok">Pakai</button></div>' +
        '<p class="catatan">Nama tombol: ESC TAB CTRL ALT SHIFT FN UP DOWN LEFT RIGHT HOME END PGUP PGDN INS DEL BKSP ENTER SPACE KBD PASTE F1–F12, atau teks apa pun. ' +
          'Geser tombol ke atas untuk isi kecilnya. Fn + angka = F1–F12. Ketuk Ctrl dua kali untuk menguncinya. Cubit layar untuk ukuran huruf.</p>' +
        '<button class="baris" id="tm-baris">' + IDE.ikon('konsol') + '<span class="nm"><b>Konsol baris</b><span>Mode lama: satu baris perintah, tanpa PTY</span></span></button>');
      function tutupLalu(f) { return function () { o.tutupLembar(); f(); }; }
      $('#tm-baru').onclick = tutupLalu(function () { baru(); });
      $('#tm-tempel').onclick = tutupLalu(tempel);
      $('#tm-salin').onclick = tutupLalu(salinPilihan);
      $('#tm-salin-semua').onclick = tutupLalu(function () { if (s) salinTeks(s.vt.semuaTeks()); });
      $('#tm-besar').onclick = function () { ubahHuruf(1); };
      $('#tm-kecil').onclick = function () { ubahHuruf(-1); };
      $('#tm-cc').onclick = tutupLalu(function () { kirim(s, '\x03'); fokus(); });
      $('#tm-cd').onclick = tutupLalu(function () { kirim(s, '\x04'); fokus(); });
      $('#tm-reset').onclick = tutupLalu(function () { if (s) { s.vt.reset(false); kirim(s, '\x0c'); } });
      $('#tm-tutup').onclick = tutupLalu(function () { tutup(s); });
      $('#tm-baris').onclick = tutupLalu(function () { if (o.keBaris) o.keBaris(); });
      $('#tm-jalan').onclick = tutupLalu(function () { if (o.jalankan) o.jalankan(); });
      $('#tm-ide').onclick = function () { o.tutupLembar(); if (o.menuIde) setTimeout(o.menuIde, 230); };
      if ($('#tm-ubuntu')) {
        $('#tm-ubuntu').onclick = tutupLalu(function () { baru(undefined, true); });
        $('#tm-android').onclick = tutupLalu(function () { baru(undefined, false); });
      }
      $('#tm-kunci-bawaan').onclick = function () { $('#tm-kunci').value = JSON.stringify(KUNCI_BAWAAN); };
      $('#tm-kunci-ok').onclick = function () {
        var v = $('#tm-kunci').value.trim();
        try { var x = JSON.parse(v); if (!Array.isArray(x)) throw new Error(); } catch (e) { IDE.toast('Tata letak harus JSON: daftar baris, tiap baris daftar tombol'); return; }
        o.atur.tKunci = JSON.stringify(x) === JSON.stringify(KUNCI_BAWAAN) ? '' : v;
        o.simpanAtur(); gambarKunci(); sesuaikanNanti(); IDE.toast('Tombol dipasang');
      };
      U.$$('#lembar [data-tk]').forEach(function (b) {
        b.onclick = function () {
          var k = b.getAttribute('data-tk'), v = b.getAttribute('data-v');
          o.atur[k] = v === 'true' ? true : v === 'false' ? false : /^\d+$/.test(v) ? +v : v;
          o.simpanAtur();
          if (k === 'tTombol') { gambarKunci(); sesuaikanNanti(); }
          if (k === 'tSaran') jenisMasuk();
          if (k === 'tSb') T.sesi.forEach(function (x) { x.vt.maksSb = o.atur.tSb; });
          if (k === 'tPenuh' && o.ubahPenuh) o.ubahPenuh();
          if (k === 'tLatar') perbaruiLatar();
          U.$$('#lembar [data-tk="' + k + '"]').forEach(function (c) { c.setAttribute('aria-pressed', String(c === b)); });
        };
      });
    }
    var BAWAAN_ATUR = { tTombol: true, tSaran: false, tBel: 'getar', tPenuh: true, tSb: 3000, tLinux: false, tLatar: true };
    function pilihan(k, daftar) {
      var kini = String(atur(k, BAWAAN_ATUR[k]));
      return '<div class="pilih">' + daftar.map(function (d) {
        return '<button data-tk="' + k + '" data-v="' + esc(String(d[0])) + '" aria-pressed="' + (String(d[0]) === kini) + '">' + esc(d[1]) + '</button>';
      }).join('') + '</div>';
    }

    /* ---------- mulai ---------- */
    function fokus() {
      var s = sesiAktif();
      if (!s) return;
      try { masuk.focus({ preventScroll: true }); } catch (e) { masuk.focus(); }
      kosongkan();
    }
    /* Panel Konsol dibuka: sesi lama dipulihkan (layar dibangun ulang dari keluarannya), atau sesi
       pertama dibuat. */
    /* Panggilan yang datang selagi sesi pertama dipulihkan atau dibuat memakai janji yang sama (jawabannya
       false), jadi membuka panel sambil meminta sesi tidak membuat dua sesi. */
    function tampil(dir, linux) {
      if (T.janji) return T.janji.then(function () { return false; });
      if (!T.siap) {
        T.siap = true;
        T.janji = o.api('/api/pty').then(function (d) {
          T.akar = d.akar;
          d.sesi.forEach(function (x) { buatTampilan(x).pulih = true; });
          if (!T.sesi.length) return baru(dir, linux).then(function () { return true; });
          var i = T.sesi.map(function (s) { return s.id; }).indexOf(o.atur.tSesi);
          pilihSesi(i < 0 ? T.sesi.length - 1 : i);
          mulaiPutar();
          return false;
        }).catch(function (e) { T.siap = false; IDE.toast('Terminal: ' + (e.message || e)); if (o.keBaris) o.keBaris(); throw e; });
        return selesaiJanji();
      }
      if (!T.sesi.length) { T.janji = baru(dir, linux).then(function () { return true; }); return selesaiJanji(); }
      sesuaikan(true);
      return Promise.resolve(false);
    }
    function selesaiJanji() {
      var j = T.janji;
      j.then(function () { T.janji = null; }, function () { T.janji = null; });
      return j;
    }

    bangun();
    return {
      tampil: tampil,
      fokus: fokus,
      aktif: function () { return document.activeElement === masuk; },
      sisip: function (t) { teksMasuk(t); },
      salin: salinTeks,
      kirim: function (t) { var s = sesiAktif(); if (s) kirim(s, t); },
      baru: baru,
      sesuaikan: function () { sesuaikanNanti(); },
      teks: function () { var s = sesiAktif(); return s ? s.vt.semuaTeks() : ''; },
      sesi: function () { return T.sesi; }
    };
  };
})(window.IDE);
