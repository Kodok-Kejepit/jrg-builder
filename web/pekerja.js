// web/pekerja.js — badan Web Worker Praktik JRG. Dikirim ke pekerja sebagai
// teks (`'(' + fungsi + ')()'`), jadi ia tidak boleh memakai apa pun di luar
// dirinya; `perkakas/praktik.py periksa` menjalankan salinan terbitnya lewat node.
function pekerjaJrg() {
  self.onmessage = function (ev) {
    const modul = ev.data.modul, sumber = ev.data.sumber;
    const dekoder = new TextDecoder();
    let mem = null;
    function Keluar(kode) { this.kode = kode; }
    const impor = { jrg: {
      // Dikirim di TIAP tulisan: yang ditahan akan hilang kalau programnya
      // lalu berputar selamanya dan pekerjanya dihentikan.
      tulis: function (fd, p, n) {
        self.postMessage({ jenis: 'keluaran', potongan: [[fd, dekoder.decode(new Uint8Array(mem.buffer, p, n))]] });
      },
      sekarang_ms: function () { return Date.now(); },
      isi_acak: function (p, n) { crypto.getRandomValues(new Uint8Array(mem.buffer, p, n)); },
      keluar: function (kode) { throw new Keluar(kode); },
    } };
    let kode = 1, sebab = null;
    try {
      const inst = new WebAssembly.Instance(modul, impor);
      mem = inst.exports.memory;
      const byte = new TextEncoder().encode(sumber);
      const p = inst.exports.jrg_alokasi(byte.length);
      new Uint8Array(mem.buffer, p, byte.length).set(byte);
      kode = inst.exports.jrg_jalankan(p, byte.length);
    } catch (e) {
      if (e instanceof Keluar) kode = e.kode;
      // Stack mesin V8 habis lebih dulu daripada batas rekursi interpreter.
      else if (e instanceof RangeError) sebab = 'rekursi';
      else sebab = 'trap: ' + (e && e.message);
    }
    self.postMessage({ jenis: 'selesai', kode: kode, sebab: sebab });
  };
}
