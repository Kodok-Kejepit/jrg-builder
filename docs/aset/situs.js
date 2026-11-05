// Perilaku bersama: tema, menu samping, pencarian, dan tombol di blok kode.
(function () {
  const akar = document.documentElement;
  const naik = (document.querySelector('link[rel="stylesheet"]').getAttribute('href') || '').replace(/aset\/situs\.css$/, '');

  function simpan(k, v) { try { localStorage.setItem(k, v); } catch (e) { /* mode privat */ } }

  // Tema: ikut sistem sampai pembaca memilih sendiri.
  function temaKini() {
    if (akar.dataset.tema) return akar.dataset.tema;
    return matchMedia('(prefers-color-scheme: dark)').matches ? 'gelap' : 'terang';
  }
  document.addEventListener('click', function (e) {
    const t = e.target.closest('[data-aksi]');
    if (!t) {
      if (document.body.classList.contains('sisi-buka') && !e.target.closest('.sisi')) document.body.classList.remove('sisi-buka');
      return;
    }
    const aksi = t.dataset.aksi;
    if (aksi === 'tema') {
      const baru = temaKini() === 'gelap' ? 'terang' : 'gelap';
      akar.dataset.tema = baru;
      simpan('jrg-tema', baru);
    } else if (aksi === 'menu') {
      e.stopPropagation();
      document.body.classList.toggle('sisi-buka');
    } else if (aksi === 'cari') {
      bukaCari();
    }
  });
  const aktif = document.querySelector('.sisi a.aktif');
  if (aktif) aktif.scrollIntoView({ block: 'center' });

  // ── cari ──
  const dialog = document.querySelector('.dialog-cari');
  const masukan = dialog.querySelector('input');
  const daftar = dialog.querySelector('.hasil-cari');
  let pilih = 0;
  function bukaCari() {
    dialog.hidden = false;
    masukan.value = '';
    tampilkan('');
    masukan.focus();
  }
  function tutupCari() { dialog.hidden = true; }
  function lipat(s) { return (s || '').toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, ''); }
  function skor(e, kata) {
    let n = 0;
    const j = lipat(e.j), k = lipat(e.k), t = lipat(e.t);
    for (const w of kata) {
      if (j.includes(w)) n += j.startsWith(w) ? 12 : 8;
      else if (k.includes(w)) n += 4;
      else if (t.includes(w)) n += 1;
      else return 0;
    }
    return n;
  }
  function esc(s) { return String(s).replace(/[&<>"]/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]; }); }
  function tampilkan(q) {
    const kata = lipat(q).split(/\s+/).filter(Boolean);
    const semua = window.JRG_CARI || [];
    let hasil = kata.length ? semua.map(function (e) { return [skor(e, kata), e]; }).filter(function (x) { return x[0] > 0; })
      .sort(function (a, b) { return b[0] - a[0]; }).slice(0, 30).map(function (x) { return x[1]; })
      : semua.filter(function (e) { return e.g === 'Dokumentasi'; }).slice(0, 12);
    pilih = 0;
    if (!hasil.length) { daftar.innerHTML = '<li class="kosong">Tidak ada yang cocok.</li>'; return; }
    daftar.innerHTML = hasil.map(function (e, i) {
      return '<li' + (i === 0 ? ' class="pilih"' : '') + '><a href="' + naik + e.u + '">' + esc(e.j) +
        '<small>' + esc(e.g) + (e.k ? ' · ' + esc(e.k) : '') + '</small></a></li>';
    }).join('');
  }
  masukan.addEventListener('input', function () { tampilkan(masukan.value); });
  masukan.addEventListener('keydown', function (e) {
    const li = daftar.querySelectorAll('li a');
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!li.length) return;
      pilih = (pilih + (e.key === 'ArrowDown' ? 1 : li.length - 1)) % li.length;
      daftar.querySelectorAll('li').forEach(function (x, i) { x.classList.toggle('pilih', i === pilih); });
      li[pilih].scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter' && li[pilih]) {
      location.href = li[pilih].href;
      tutupCari();
    }
  });
  dialog.addEventListener('click', function (e) { if (e.target === dialog) tutupCari(); });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') { tutupCari(); document.body.classList.remove('sisi-buka'); }
    const ketik = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName);
    if (!ketik && (e.key === '/' || ((e.ctrlKey || e.metaKey) && e.key === 'k'))) { e.preventDefault(); bukaCari(); }
  });

  // ── blok kode: Salin, Jalankan, Ubah ──
  function salin(teks, tombol) {
    const beres = function (ok) {
      tombol.textContent = ok ? 'Tersalin' : 'Gagal';
      setTimeout(function () { tombol.textContent = 'Salin'; }, 1400);
    };
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(teks).then(function () { beres(true); }, function () { beres(false); });
      return;
    }
    const ta = document.createElement('textarea');
    ta.value = teks;
    ta.style.cssText = 'position:fixed;top:-1000px;opacity:0';
    document.body.appendChild(ta);
    ta.select();
    let ok = false;
    try { ok = document.execCommand('copy'); } catch (e) { ok = false; }
    ta.remove();
    beres(ok);
  }
  let muatPraktik = null;
  function praktik() {
    if (window.JRG_PRAKTIK) return Promise.resolve(window.JRG_PRAKTIK);
    if (!muatPraktik) muatPraktik = new Promise(function (ok, gagal) {
      const s = document.createElement('script');
      s.src = naik + 'aset/praktik.js';
      s.onload = function () { ok(window.JRG_PRAKTIK); };
      s.onerror = function () { muatPraktik = null; gagal(new Error('praktik.js tidak bisa dimuat')); };
      document.head.appendChild(s);
    });
    return muatPraktik;
  }
  function sumberDari(fig) {
    const ta = fig.querySelector('textarea');
    return ta ? ta.value : fig.querySelector('pre:not(.keluaran) code').textContent;
  }
  function jalankan(fig, tombol) {
    let out = fig.querySelector('.keluaran');
    if (!out) {
      out = document.createElement('pre');
      out.className = 'keluaran';
      out.appendChild(document.createElement('code'));
      fig.appendChild(out);
    }
    const kode = out.querySelector('code');
    out.classList.add('berjalan');
    kode.textContent = '';
    tombol.disabled = true;
    praktik().then(function (p) { return p.jalankan(sumberDari(fig)).then(function (h) { return [p, h]; }); })
      .then(function (ph) {
        const p = ph[0], h = ph[1];
        kode.textContent = '';
        for (const [fd, t] of h.potongan) {
          const s = document.createElement('span');
          if (fd === 2) s.className = 'galat';
          s.textContent = t;
          kode.appendChild(s);
        }
        const sebab = p.kalimatSebab(h);
        if (sebab) { const s = document.createElement('span'); s.className = 'galat'; s.textContent = '\n' + sebab; kode.appendChild(s); }
        if (h.terpotong) kode.appendChild(document.createTextNode('\n[keluaran dipotong]'));
        if (!kode.textContent.trim()) kode.textContent = '(tidak ada keluaran)';
      }, function (e) {
        kode.innerHTML = '';
        const s = document.createElement('span');
        s.className = 'galat';
        s.textContent = 'Tidak bisa memuat mesin JRG: ' + e.message;
        kode.appendChild(s);
      }).then(function () { out.classList.remove('berjalan'); tombol.disabled = false; });
  }
  function ubah(fig, tombol) {
    const pre = fig.querySelector('pre:not(.keluaran)');
    if (fig.querySelector('textarea')) return;
    const ta = document.createElement('textarea');
    ta.value = pre.textContent;
    ta.spellcheck = false;
    ta.rows = Math.min(30, ta.value.split('\n').length + 1);
    pre.replaceWith(ta);
    ta.focus();
    ta.addEventListener('keydown', function (e) {
      if (e.key === 'Tab') {
        e.preventDefault();
        const a = ta.selectionStart;
        ta.setRangeText('    ', a, ta.selectionEnd, 'end');
      } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        jalankan(fig, fig.querySelector('[data-k="jalan"]'));
      }
    });
    tombol.remove();
  }
  document.querySelectorAll('figure.kode').forEach(function (fig) {
    const cap = fig.querySelector('figcaption');
    const alat = document.createElement('span');
    alat.className = 'alat';
    const b = function (label, k, utama) {
      const t = document.createElement('button');
      t.type = 'button';
      t.textContent = label;
      t.dataset.k = k;
      if (utama) t.className = 'utama';
      alat.appendChild(t);
      return t;
    };
    b('Salin', 'salin').addEventListener('click', function () { salin(sumberDari(fig), this); });
    if (fig.hasAttribute('data-jalan')) {
      b('Ubah', 'ubah').addEventListener('click', function () { ubah(fig, this); });
      b('Jalankan', 'jalan', true).addEventListener('click', function () { jalankan(fig, this); });
    }
    cap.appendChild(alat);
  });

  // ── di halaman ini: tandai bagian yang sedang dibaca ──
  const tautan = document.querySelectorAll('.di-halaman a');
  if (tautan.length && 'IntersectionObserver' in window) {
    const peta = new Map();
    tautan.forEach(function (a) { const h = document.getElementById(a.getAttribute('href').slice(1)); if (h) peta.set(h, a); });
    const io = new IntersectionObserver(function (es) {
      es.forEach(function (e) {
        if (!e.isIntersecting) return;
        tautan.forEach(function (a) { a.classList.remove('aktif'); });
        peta.get(e.target).classList.add('aktif');
      });
    }, { rootMargin: '-15% 0px -75% 0px' });
    peta.forEach(function (_, h) { io.observe(h); });
  }
})();
