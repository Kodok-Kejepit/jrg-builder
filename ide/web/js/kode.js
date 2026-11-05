/* JRG IDE — js/kode.js
   Alat atas kode: ke definisi, rujukan, ganti nama simbol di seluruh berkas, rapikan, riwayat berkas
   (dengan beda baris), berkas terhapus, dan daftar TODO. Rancangan: ide/README.md, "Alat kode". */
(function (IDE) {
  'use strict';
  var U = IDE.util, esc = U.esc, $ = U.$, ikon = IDE.ikon;

  /* ---------- beda baris ---------- */
  /* Awalan dan akhiran yang sama dibuang dulu; sisanya LCS. Kalau sisanya terlalu besar untuk tabel
     (lebih dari 3 juta sel), seluruh bagian tengah dianggap diganti: tetap benar, cuma kurang rinci. */
  function beda(lama, baru) {
    var a = lama.split('\n'), b = baru.split('\n'), i = 0, ja = a.length, jb = b.length, hasil = [];
    while (i < ja && i < jb && a[i] === b[i]) i++;
    while (ja > i && jb > i && a[ja - 1] === b[jb - 1]) { ja--; jb--; }
    for (var k = 0; k < i; k++) hasil.push({ t: ' ', s: a[k], a: k + 1, b: k + 1 });
    var n = ja - i, m = jb - i;
    if (n * m > 3000000) {
      for (k = i; k < ja; k++) hasil.push({ t: '-', s: a[k], a: k + 1 });
      for (k = i; k < jb; k++) hasil.push({ t: '+', s: b[k], b: k + 1 });
    } else {
      var L = new Uint32Array((n + 1) * (m + 1)), x, y, w = m + 1;
      for (x = n - 1; x >= 0; x--) for (y = m - 1; y >= 0; y--)
        L[x * w + y] = a[i + x] === b[i + y] ? L[(x + 1) * w + y + 1] + 1 : Math.max(L[(x + 1) * w + y], L[x * w + y + 1]);
      x = 0; y = 0;
      while (x < n || y < m) {
        if (x < n && y < m && a[i + x] === b[i + y]) { hasil.push({ t: ' ', s: a[i + x], a: i + x + 1, b: i + y + 1 }); x++; y++; }
        else if (x < n && (y >= m || L[(x + 1) * w + y] >= L[x * w + y + 1])) { hasil.push({ t: '-', s: a[i + x], a: i + x + 1 }); x++; }
        else { hasil.push({ t: '+', s: b[i + y], b: i + y + 1 }); y++; }
      }
    }
    for (k = ja; k < a.length; k++) hasil.push({ t: ' ', s: a[k], a: k + 1, b: k - ja + jb + 1 });
    return hasil;
  }
  /* Potongan berubah dengan 3 baris konteks; sisanya diringkas. */
  function gambarBeda(d) {
    var tambah = 0, buang = 0, tampil = [], i;
    d.forEach(function (x) { if (x.t === '+') tambah++; else if (x.t === '-') buang++; });
    if (!tambah && !buang) return { tambah: 0, buang: 0, html: '<p class="catatan">Isinya sama persis.</p>' };
    var dekat = new Uint8Array(d.length);
    for (i = 0; i < d.length; i++) if (d[i].t !== ' ') for (var j = Math.max(0, i - 3); j <= Math.min(d.length - 1, i + 3); j++) dekat[j] = 1;
    var lewat = 0;
    for (i = 0; i < d.length && tampil.length < 3000; i++) {
      if (!dekat[i]) { lewat++; continue; }
      if (lewat) { tampil.push('<span class="bd-lewat">··· ' + lewat + ' baris sama ···</span>'); lewat = 0; }
      var x = d[i];
      tampil.push('<span class="bd-' + (x.t === '+' ? 'plus' : x.t === '-' ? 'min' : 'sama') + '"><i>' + (x.t === '+' ? x.b : x.a) + '</i>' + esc(x.t + ' ' + x.s) + '</span>');
    }
    if (lewat) tampil.push('<span class="bd-lewat">··· ' + lewat + ' baris sama ···</span>');
    return { tambah: tambah, buang: buang, html: '<pre class="beda">' + tampil.join('') + '</pre>' };
  }

  /* ---------- rapikan ---------- */
  var KURAWAL = { jrg: 1, js: 1, ts: 1, c: 1, java: 1, kotlin: 1, cs: 1, go: 1, rust: 1, css: 1 };
  /* Indentasi ulang menurut kedalaman kurung, 4 spasi per tingkat. Teks, karakter, komentar dan
     string banyak baris dilewati; baris yang DIMULAI di dalam string banyak baris tidak disentuh. */
  function indentUlang(teks, lang) {
    var baris = teks.split('\n'), keluar = [], dalam = 0, mode = '', tutupMode = '';
    var lanjut = /^(\.(?!\.)|\+|&&|\|\||\?|\$|:(?!:)|=>|->)/;
    for (var n = 0; n < baris.length; n++) {
      var l = baris[n], mulaiDalam = mode;
      if (!mulaiDalam) {
        var isi = l.trim(), awalTutup = /^[)\]}]+/.exec(isi), t = awalTutup ? awalTutup[0].length : 0;
        var tingkat = Math.max(0, dalam - t) + (isi && !t && lanjut.test(isi) ? 1 : 0);
        keluar.push(isi ? new Array(tingkat + 1).join('    ') + isi : '');
      } else if (mulaiDalam === 'kom' && /^\s*\*/.test(l)) {
        /* Badan komentar blok gaya ` * teks`: ikut tingkat kodenya, bintangnya sejajar. */
        keluar.push(new Array(dalam + 1).join('    ') + ' ' + l.trim());
      } else keluar.push(mulaiDalam === 'kom' ? l.replace(/[ \t]+$/, '') : l);
      for (var i = 0; i < l.length; i++) {
        var c = l[i], d = l.substr(i, 3);
        if (mode === 'kom') { if (l.substr(i, 2) === '*/') { mode = ''; i++; } continue; }
        if (mode === 'teks') { if (c === '\\') { i++; continue; } if (l.substr(i, tutupMode.length) === tutupMode) { mode = ''; i += tutupMode.length - 1; } continue; }
        if (l.substr(i, 2) === '//') break;
        if (l.substr(i, 2) === '/*') { mode = 'kom'; i++; continue; }
        if (d === '"""') { mode = 'teks'; tutupMode = '"""'; i += 2; continue; }
        if (c === '"' || c === '`' || (c === '\'' && lang !== 'rust')) { mode = 'teks'; tutupMode = c; continue; }
        if (c === '«') { mode = 'teks'; tutupMode = '»'; continue; }
        if (c === '{' || c === '(' || c === '[') dalam++;
        else if (c === '}' || c === ')' || c === ']') dalam = Math.max(0, dalam - 1);
      }
      /* Kutip satu dan dua tidak menyeberang baris; yang belum tertutup di akhir baris dianggap selesai. */
      if (mode === 'teks' && tutupMode !== '"""' && tutupMode !== '`' && tutupMode !== '»') mode = '';
    }
    return keluar.join('\n');
  }
  function rapikanTeks(teks, lang) {
    if (lang === 'json') return JSON.stringify(JSON.parse(teks), null, 4) + '\n';
    var hasil = KURAWAL[lang] ? indentUlang(teks, lang) : teks.split('\n').map(function (l) { return l.replace(/[ \t]+$/, ''); }).join('\n');
    return hasil.replace(/\n{3,}$/, '\n').replace(/([^\n])$/, '$1\n');
  }

  IDE.kode = function (K) {
    var ed = K.ed, api = K.api;

    function aktif() { return K.S.tab[K.S.aktif]; }
    function lang() { var t = aktif(); return t ? IDE.hl.langOf(t.nama) : ''; }
    /* Nama di kursor, tanpa awalan modul (`hitung::tambah` → `tambah`). */
    function kata() {
      var w = ed.wordAt ? ed.wordAt() : '', pilih = ed.terpilih();
      if (pilih && /^[A-Za-z_$][\w$]*$/.test(pilih)) return pilih;
      return w.split('::').pop();
    }
    function polaKata(nama) { return new RegExp('(^|[^\\w$])' + nama.replace(/\$/g, '\\$') + '(?![\\w$])', 'g'); }
    /* Semua kemunculan utuh `nama` di semua berkas: [{nama, baris, kolom, teks}]. */
    function kemunculan(nama) {
      return K.simpanSemua().then(function () { return api('/api/cari?persis=1&kata=' + encodeURIComponent(nama)); }).then(function (d) {
        var hasil = [];
        d.forEach(function (x) {
          var re = polaKata(nama), m;
          while ((m = re.exec(x.teks))) hasil.push({ nama: x.nama, baris: x.baris, kolom: m.index + m[1].length + 1, teks: x.teks });
        });
        return hasil;
      });
    }
    var DEF = '(?:fn|def|function\\*?|class|struct|enum|trait|type|interface|const|let|var|local\\s+function|local|func|module|object|record|fun|extern\\s+fn|macro_rules!)';
    function apakahDefinisi(x, nama) {
      var n = nama.replace(/\$/g, '\\$');
      return new RegExp('(^|[^\\w$])' + DEF + '\\s+(?:mut\\s+)?' + n + '(?![\\w$])').test(x.teks) ||
        new RegExp('^\\s*(?:pub\\s+)?' + n + '\\s*(?::=|=\\s*(?:function|\\(|async))').test(x.teks) ||
        new RegExp('^\\s*' + n + '\\s*\\(\\)\\s*\\{').test(x.teks) ||
        new RegExp('^\\s*(?:pub\\s+)?' + n + '\\s*(?:[({,]|$)').test(x.teks) && /^\s*(?:pub\s+)?[A-Z]/.test(x.teks) && x.nama.slice(-4) === '.jrg';
    }
    function lompat(x) {
      K.tutupLembar();
      return K.buka(x.nama).then(function () {
        var p = ed.awalBaris(x.baris - 1) + x.kolom - 1;
        ed.selectRange(p, p + (x.panjang || 0));
      });
    }
    function daftarTemuan(judul, d, nama, catatan) {
      K.lembar(judul, (catatan ? '<p class="catatan">' + catatan + '</p>' : '') + '<div id="kd-hasil">' + d.map(function (x, i) {
        var awal = x.teks.replace(/^\s+/, ''), geser = x.teks.length - awal.length;
        var teks = esc(awal.slice(0, x.kolom - 1 - geser)) + '<mark>' + esc(awal.substr(x.kolom - 1 - geser, nama.length)) + '</mark>' + esc(awal.slice(x.kolom - 1 - geser + nama.length));
        return '<button class="baris" data-i="' + i + '">' + ikon('berkas') + '<span class="nm"><b>' + esc(x.nama) + ':' + x.baris + ':' + x.kolom + '</b><span class="mono">' + teks + '</span></span></button>';
      }).join('') + '</div>');
      U.$$('#kd-hasil [data-i]').forEach(function (b) {
        b.onclick = function () { var x = d[+b.getAttribute('data-i')]; x.panjang = nama.length; lompat(x).catch(K.gagal); };
      });
    }

    function definisi() {
      var t = aktif(), nama = kata();
      if (!t) return;
      if (!/^[A-Za-z_$][\w$]*$/.test(nama)) { IDE.toast('Taruh kursor di sebuah nama dulu'); return; }
      kemunculan(nama).then(function (d) {
        var def = d.filter(function (x) { return apakahDefinisi(x, nama); });
        /* Berkas ini dulu, lalu yang lain. */
        def.sort(function (a, b) { return (a.nama === t.nama ? 0 : 1) - (b.nama === t.nama ? 0 : 1) || (a.nama < b.nama ? -1 : a.nama > b.nama ? 1 : a.baris - b.baris); });
        if (def.length === 1) { def[0].panjang = nama.length; return lompat(def[0]); }
        if (def.length > 1) return daftarTemuan('Definisi ' + nama, def, nama, def.length + ' tempat yang tampak seperti definisi');
        var w = ed.wordAt();
        if (lang() === 'jrg') return api('/api/dok?nama=' + encodeURIComponent(w)).then(function (x) {
          var teks = (x.teks || '').trim();
          if (teks && !/tidak (ada|dikenal|terdaftar|ketemu)/.test(teks.split('\n')[0])) return tampilDok(w, teks);
          IDE.toast('Definisi `' + nama + '` tidak ditemukan di berkas mana pun');
        });
        IDE.toast('Definisi `' + nama + '` tidak ditemukan di berkas mana pun');
      }).catch(K.gagal);
    }
    function tampilDok(nama, teks) {
      K.lembar(nama, '<div class="dok">' + esc(teks) + '</div>');
    }
    function dokumentasi() {
      var w = ed.wordAt();
      if (!w) { IDE.toast('Taruh kursor di sebuah nama dulu'); return; }
      api('/api/dok?nama=' + encodeURIComponent(w)).then(function (x) {
        var teks = (x.teks || '').trim();
        if (teks && !/tidak (ada|dikenal|terdaftar|ketemu)/.test(teks.split('\n')[0])) tampilDok(w, teks);
        else definisi();
      }).catch(K.gagal);
    }
    function rujukan() {
      var nama = kata();
      if (!/^[A-Za-z_$][\w$]*$/.test(nama)) { IDE.toast('Taruh kursor di sebuah nama dulu'); return; }
      kemunculan(nama).then(function (d) {
        if (!d.length) { IDE.toast('`' + nama + '` tidak muncul di berkas mana pun'); return; }
        var berkas = {};
        d.forEach(function (x) { berkas[x.nama] = 1; });
        daftarTemuan('Rujukan ' + nama, d, nama, d.length + ' kemunculan di ' + Object.keys(berkas).length + ' berkas' + (d.length >= 3000 ? ' (dibatasi 3000)' : ''));
      }).catch(K.gagal);
    }

    /* Ganti nama di seluruh berkas yang dicentang; berkas terbuka diganti di tabnya (bisa diurungkan). */
    function gantiNama() {
      var lamaNama = kata();
      if (!/^[A-Za-z_$][\w$]*$/.test(lamaNama)) { IDE.toast('Taruh kursor di sebuah nama dulu'); return; }
      kemunculan(lamaNama).then(function (d) {
        var per = {};
        d.forEach(function (x) { per[x.nama] = (per[x.nama] || 0) + 1; });
        var daftar = Object.keys(per).sort();
        K.lembar('Ganti nama ' + lamaNama,
          '<input class="isian mono" id="gn-baru" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Nama baru">' +
          '<p class="catatan">' + d.length + ' kemunculan utuh di ' + daftar.length + ' berkas. Yang di dalam teks dan komentar ikut diganti; hilangkan centang berkas yang tidak mau diubah. Berkas yang terbuka bisa diurungkan.</p>' +
          '<div id="gn-berkas">' + daftar.map(function (n, i) {
            return '<label class="centang"><input type="checkbox" data-i="' + i + '" checked> ' + esc(n) + ' <span class="hc">(' + per[n] + ')</span></label>';
          }).join('') + '</div>' +
          '<div class="deret"><button class="tombol utama" id="gn-ok">Ganti</button></div><p class="catatan" id="gn-kabar"></p>');
        var inp = $('#gn-baru');
        inp.value = lamaNama;
        setTimeout(function () { inp.focus(); inp.select(); }, 200);
        $('#gn-ok').onclick = function () {
          var baru = inp.value.trim();
          if (!/^[A-Za-z_$][\w$]*$/.test(baru)) { $('#gn-kabar').textContent = 'Nama baru harus satu nama: huruf, angka, garis bawah, tidak diawali angka.'; return; }
          if (baru === lamaNama) { K.tutupLembar(); return; }
          var pilih = U.$$('#gn-berkas input:checked').map(function (c) { return daftar[+c.getAttribute('data-i')]; });
          var ganti = function (teks) { return teks.replace(polaKata(lamaNama), function (_, depan) { return depan + baru; }); };
          var jumlah = 0;
          var kerja = pilih.map(function (n) {
            var tab = K.S.tab.filter(function (t) { return t.nama === n; })[0];
            if (tab) {
              var isiBaru = ganti(tab.isi);
              if (tab === aktif()) { var s = ed.sel(); ed.gantiSemua(isiBaru, [Math.min(s[0], isiBaru.length), Math.min(s[0], isiBaru.length)]); }
              else { tab.isi = isiBaru; tab.kotor = true; tab.keadaan = null; }
              jumlah++;
              return Promise.resolve();
            }
            return api('/api/baca?nama=' + encodeURIComponent(n)).then(function (x) {
              return api('/api/simpan', { nama: n, isi: ganti(x.isi) });
            }).then(function () { jumlah++; });
          });
          Promise.all(kerja).then(function () { return K.simpanSemua(); }).then(function () {
            K.tutupLembar(); K.segarkan();
            IDE.toast('`' + lamaNama + '` jadi `' + baru + '` di ' + jumlah + ' berkas');
          }).catch(K.gagal);
        };
      }).catch(K.gagal);
    }

    /* Rapikan: perapi resmi bahasanya lewat pelayan (`/api/rapikan`: jrgc --fmt / jrgm fmt untuk JRG,
       gofmt, rustfmt, clang-format, prettier, ruff/black, shfmt) kalau terpasang; kalau tidak, indentasi
       menurut kurung di sini. `diam`: dari simpan — tanpa kabar. Jawabannya selalu janji. */
    function pasangHasil(v, baru) {
      var p = ed.pos(), lamaB = v.split('\n'), baruB = baru.split('\n'), i = Math.min(p.baris - 1, baruB.length - 1);
      var geser = (/^\s*/.exec(baruB[i])[0].length) - (/^\s*/.exec(lamaB[p.baris - 1] || '')[0].length);
      var awal = 0;
      for (var k = 0; k < i; k++) awal += baruB[k].length + 1;
      var q = awal + Math.max(0, Math.min(baruB[i].length, p.kolom - 1 + geser));
      var ubah = 0;
      for (k = 0; k < Math.max(lamaB.length, baruB.length); k++) if (lamaB[k] !== baruB[k]) ubah++;
      ed.gantiSemua(baru, [q, q]);
      return ubah;
    }
    function rapikanLokal(t, v, l, diam) {
      if (diam === true && !KURAWAL[l] && l !== 'json') return;
      var baru;
      try { baru = rapikanTeks(v, l); }
      catch (e) { if (diam !== true) IDE.toast('Tidak bisa dirapikan: ' + (e.message || e)); return; }
      var dalam = KURAWAL[l] && l !== 'json' ? ' Spasi di dalam baris belum bisa dirapikan di perangkat ini' + (l === 'jrg' ? ' (perlu jrgc --fmt)' : ' (perapi bahasanya tidak terpasang)') + '.' : '';
      if (baru === v) { if (diam !== true) IDE.toast((KURAWAL[l] ? 'Indentasi sudah rapi.' : 'Sudah rapi.') + dalam); return; }
      var ubah = pasangHasil(v, baru);
      if (diam !== true) IDE.toast((KURAWAL[l] || l === 'json' ? 'Indentasi dirapikan: ' : 'Spasi di ujung baris dirapikan: ') + ubah + ' baris.' + dalam);
    }
    function rapikan(diam) {
      var t = aktif();
      if (!t) return Promise.resolve();
      var l = lang(), v = ed.getValue();
      if (l === 'json') { rapikanLokal(t, v, l, diam); return Promise.resolve(); }
      return api('/api/rapikan', { nama: t.nama, isi: v }).then(function (d) {
        if (aktif() !== t || ed.getValue() !== v) return;
        if (!d.alat) { rapikanLokal(t, v, l, diam); return; }
        if (d.galat) { if (diam !== true) IDE.toast('Tidak bisa dirapikan (' + d.alat + '): ' + d.galat); return; }
        var baru = d.isi.replace(/\n*$/, '\n');
        if (baru === v) { if (diam !== true) IDE.toast('Sudah rapi menurut ' + d.alat); return; }
        var ubah = pasangHasil(v, baru);
        if (diam !== true) IDE.toast('Dirapikan dengan ' + d.alat + ': ' + ubah + ' baris berubah');
      }, function () { if (aktif() === t && ed.getValue() === v) rapikanLokal(t, v, l, diam); });
    }

    /* ---------- riwayat ---------- */
    function kapan(ms) {
      var d = (Date.now() - ms) / 1000, tgl = new Date(ms);
      var jam = ('0' + tgl.getHours()).slice(-2) + '.' + ('0' + tgl.getMinutes()).slice(-2);
      var rel = d < 90 ? 'baru saja' : d < 3600 ? Math.round(d / 60) + ' menit lalu' : d < 86400 ? Math.round(d / 3600) + ' jam lalu' : Math.round(d / 86400) + ' hari lalu';
      return rel + ' · ' + tgl.getDate() + '/' + (tgl.getMonth() + 1) + ' ' + jam;
    }
    function riwayat() {
      var t = aktif();
      if (!t) return;
      api('/api/riwayat?nama=' + encodeURIComponent(t.nama)).then(function (d) {
        K.lembar('Riwayat ' + t.nama.slice(t.nama.lastIndexOf('/') + 1),
          (t.kotor ? '<button class="baris" id="rw-kotor">' + ikon('ubah') + '<span class="nm"><b>Perubahan yang belum disimpan</b><span>Dibandingkan dengan isi berkas di perangkat</span></span></button>' : '') +
          (d.length ? d.map(function (x, i) {
            return '<button class="baris" data-i="' + i + '">' + ikon('urung') + '<span class="nm"><b>' + kapan(x.t) + '</b><span>' + x.ukuran + ' byte</span></span></button>';
          }).join('') : '<p class="catatan">Belum ada salinan. Isi lama berkas disalin ke riwayatnya sebelum ditimpa (paling sering sekali per 2 menit) atau dihapus; paling banyak 40 salinan.</p>'));
        if ($('#rw-kotor')) $('#rw-kotor').onclick = function () {
          api('/api/baca?nama=' + encodeURIComponent(t.nama)).then(function (x) { lihatBeda('Belum disimpan', x.isi, t.isi, null); }).catch(K.gagal);
        };
        U.$$('#lembar [data-i]').forEach(function (b) {
          b.onclick = function () {
            var x = d[+b.getAttribute('data-i')];
            api('/api/riwayat/isi?nama=' + encodeURIComponent(t.nama) + '&t=' + x.t).then(function (r) {
              lihatBeda(kapan(x.t), r.isi, ed.getValue(), r.isi);
            }).catch(K.gagal);
          };
        });
      }).catch(K.gagal);
    }
    /* `lama` → `baru`; `pulih` (boleh null) = isi yang dipasang ke penyunting kalau dipulihkan. */
    function lihatBeda(judul, lama, baru, pulih) {
      var g = gambarBeda(beda(lama, baru));
      K.lembar(judul, '<p class="catatan">Dari versi ini ke yang sekarang: <b class="bd-plus-t">+' + g.tambah + '</b> <b class="bd-min-t">−' + g.buang + '</b> baris</p>' + g.html +
        '<div class="deret">' + (pulih != null ? '<button class="tombol utama" id="rw-pulih">Pulihkan versi ini</button>' : '') + '<button class="tombol" id="rw-kembali">Kembali</button></div>');
      $('#rw-kembali').onclick = riwayat;
      if ($('#rw-pulih')) $('#rw-pulih').onclick = function () {
        ed.gantiSemua(pulih, [0, 0]);
        K.tutupLembar();
        IDE.toast('Dipulihkan; Urungkan mengembalikan isi sebelumnya');
      };
    }
    /* Bandingkan berkas ini dengan berkas lain: pilih dari daftar (disaring ketikan), lalu beda baris. */
    function bandingkan() {
      var t = aktif();
      if (!t) return;
      K.lembar('Bandingkan dengan…', '<input class="isian" id="bd-cari" placeholder="Cari berkas" autocapitalize="off" autocorrect="off" spellcheck="false" aria-label="Cari berkas pembanding"><div id="bd-daftar"></div>');
      var inp = $('#bd-cari');
      function gambar() {
        var q = inp.value.toLowerCase();
        var d = K.S.berkas.filter(function (f) { return f.nama !== t.nama && f.nama.toLowerCase().indexOf(q) >= 0; }).slice(0, 80);
        $('#bd-daftar').innerHTML = d.length ? d.map(function (f) {
          return '<button class="baris" data-bd="' + esc(f.nama) + '">' + ikon('berkas') + '<span class="nm"><b>' + esc(f.nama) + '</b></span></button>';
        }).join('') : '<p class="catatan">Tidak ada berkas lain yang cocok.</p>';
        Array.prototype.forEach.call(document.querySelectorAll('#bd-daftar [data-bd]'), function (b) {
          b.onclick = function () {
            var nama = b.getAttribute('data-bd');
            api('/api/baca?nama=' + encodeURIComponent(nama)).then(function (d) {
              var g = gambarBeda(beda(d.isi, ed.getValue()));
              K.lembar('Beda', '<p class="catatan"><span class="mono">' + esc(nama) + '</span> → <span class="mono">' + esc(t.nama) + '</span>: <b class="bd-plus-t">+' + g.tambah + '</b> <b class="bd-min-t">−' + g.buang + '</b> baris</p>' + g.html +
                '<div class="deret"><button class="tombol" id="bd-lain">Bandingkan dengan yang lain</button></div>');
              $('#bd-lain').onclick = bandingkan;
            }).catch(K.gagal);
          };
        });
      }
      inp.oninput = gambar;
      gambar();
    }
    function terhapus() {
      api('/api/riwayat/terhapus').then(function (d) {
        K.lembar('Berkas terhapus', d.length ? d.map(function (x, i) {
          return '<button class="baris" data-i="' + i + '">' + ikon('berkas') + '<span class="nm"><b>' + esc(x.nama) + '</b><span>' + kapan(x.t) + ' · ' + x.jumlah + ' salinan</span></span></button>';
        }).join('') + '<p class="catatan">Berkas yang dihapus, atau yang pernah ada dengan nama itu, beserta salinan terakhirnya.</p>' :
          '<p class="catatan">Tidak ada. Berkas yang dihapus dari IDE disalin ke riwayatnya dulu, jadi bisa dipulihkan dari sini.</p>');
        U.$$('#lembar [data-i]').forEach(function (b) {
          b.onclick = function () {
            var x = d[+b.getAttribute('data-i')];
            api('/api/riwayat/isi?nama=' + encodeURIComponent(x.nama) + '&t=' + x.t).then(function (r) {
              K.lembar(x.nama, '<p class="catatan">' + kapan(x.t) + '</p><pre class="beda">' + esc(r.isi.slice(0, 200000)) + '</pre>' +
                '<div class="deret"><button class="tombol utama" id="th-pulih">Pulihkan ke ' + esc(x.nama) + '</button><button class="tombol" id="th-kembali">Kembali</button></div>');
              $('#th-kembali').onclick = terhapus;
              $('#th-pulih').onclick = function () {
                api('/api/simpan', { nama: x.nama, isi: r.isi }).then(function () { K.tutupLembar(); return K.muatBerkas(); })
                  .then(function () { return K.buka(x.nama); }).then(function () { IDE.toast(x.nama + ' dipulihkan'); }).catch(K.gagal);
              };
            }).catch(K.gagal);
          };
        });
      }).catch(K.gagal);
    }

    /* TODO, FIXME, XXX di semua berkas (huruf besar, kata utuh). */
    function todo() {
      K.simpanSemua().then(function () {
        return Promise.all(['TODO', 'FIXME', 'XXX'].map(function (k) { return api('/api/cari?persis=1&kata=' + k); }));
      }).then(function (hasil) {
        var d = [], lihat = {};
        hasil.forEach(function (h, j) {
          var k = ['TODO', 'FIXME', 'XXX'][j];
          h.forEach(function (x) {
            var m = polaKata(k).exec(x.teks), kunci = x.nama + ':' + x.baris;
            if (!m || lihat[kunci]) return;
            lihat[kunci] = 1;
            d.push({ nama: x.nama, baris: x.baris, kolom: m.index + m[1].length + 1, teks: x.teks, k: k });
          });
        });
        d.sort(function (a, b) { return a.nama < b.nama ? -1 : a.nama > b.nama ? 1 : a.baris - b.baris; });
        if (!d.length) { K.lembar('TODO', '<p class="catatan">Tidak ada TODO, FIXME, atau XXX di berkas mana pun.</p>'); return; }
        K.lembar('TODO', '<p class="catatan">' + d.length + ' catatan</p><div id="kd-hasil">' + d.map(function (x, i) {
          return '<button class="baris" data-i="' + i + '">' + ikon('masalah') + '<span class="nm"><b>' + esc(x.nama) + ':' + x.baris + '</b><span class="mono">' + esc(x.teks.trim()) + '</span></span></button>';
        }).join('') + '</div>');
        U.$$('#kd-hasil [data-i]').forEach(function (b) {
          b.onclick = function () { var x = d[+b.getAttribute('data-i')]; x.panjang = x.k.length; lompat(x).catch(K.gagal); };
        });
      }).catch(K.gagal);
    }

    return { definisi: definisi, rujukan: rujukan, gantiNama: gantiNama, dokumentasi: dokumentasi, rapikan: function () { return rapikan(false); }, rapikanDiam: function () { return rapikan(true); },
      riwayat: riwayat, terhapus: terhapus, todo: todo, kata: kata, bandingkan: bandingkan };
  };
  IDE.kode.beda = beda;
  IDE.kode.rapikanTeks = rapikanTeks;
})(window.IDE);
