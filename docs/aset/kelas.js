// Kelas Matematika x JRG: kuis dan progres. Progres disimpan di perangkat pembaca (localStorage).
(function () {
  const KUNCI = 'jrg-kelas';
  const data = window.JRG_KELAS || { bab: [], soal: {}, proyek: [] };
  let keadaan = { benar: {}, proyek: {} };
  try { keadaan = Object.assign(keadaan, JSON.parse(localStorage.getItem(KUNCI) || '{}')); } catch (e) { /* kosong */ }
  function simpan() { try { localStorage.setItem(KUNCI, JSON.stringify(keadaan)); } catch (e) { /* mode privat */ } }

  function babSelesai(id) {
    if (data.proyek.indexOf(id) >= 0) return !!keadaan.proyek[id];
    const soal = data.soal[id] || [];
    return soal.length > 0 && soal.every(function (q) { return keadaan.benar[q]; });
  }
  function perbarui() {
    let n = 0;
    data.bab.forEach(function (id) {
      const ok = babSelesai(id);
      if (ok) n++;
      const a = document.querySelector('.sisi a[data-bab="' + id + '"]');
      if (a) a.classList.toggle('selesai', ok);
    });
    const angka = document.querySelector('.progres-angka');
    const batang = document.querySelector('.progres-batang div');
    if (angka) angka.textContent = n;
    if (batang) batang.style.width = (data.bab.length ? n / data.bab.length * 100 : 0) + '%';
    document.querySelectorAll('[data-proyek]').forEach(function (t) {
      const ok = !!keadaan.proyek[t.dataset.proyek];
      t.textContent = ok ? 'Proyek sudah ditandai selesai' : 'Tandai proyek selesai';
      t.disabled = ok;
    });
    document.querySelectorAll('[data-lanjut]').forEach(function (a) {
      const id = data.bab.find(function (b) { return !babSelesai(b); });
      if (n > 0 && id) {
        a.hidden = false;
        a.href = id + '.html';
      }
    });
  }

  document.querySelectorAll('.kuis').forEach(function (k) {
    const qid = k.dataset.kuis;
    const tombol = k.querySelectorAll('.kuis-pilihan button');
    const umpan = function (benar) {
      k.querySelector('.kuis-umpan.benar').hidden = !benar;
      k.querySelector('.kuis-umpan.salah').hidden = benar;
    };
    const kunci = function () {
      tombol.forEach(function (b) {
        b.disabled = true;
        if (b.dataset.benar === '1') b.classList.add('benar');
      });
    };
    if (keadaan.benar[qid]) { kunci(); umpan(true); }
    tombol.forEach(function (b) {
      b.addEventListener('click', function () {
        if (b.dataset.benar === '1') {
          tombol.forEach(function (x) { x.classList.remove('salah'); });
          kunci();
          umpan(true);
          keadaan.benar[qid] = true;
          simpan();
          perbarui();
        } else {
          b.classList.add('salah');
          umpan(false);
        }
      });
    });
  });
  document.querySelectorAll('[data-proyek]').forEach(function (t) {
    t.addEventListener('click', function () {
      keadaan.proyek[t.dataset.proyek] = true;
      simpan();
      perbarui();
    });
  });
  document.querySelectorAll('[data-ulang]').forEach(function (t) {
    t.addEventListener('click', function () {
      if (!confirm('Hapus semua progres kuis di perangkat ini?')) return;
      keadaan = { benar: {}, proyek: {} };
      simpan();
      location.reload();
    });
  });
  perbarui();
})();
