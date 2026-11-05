/* JRG IDE — js/pekerja_sorot.js: sorotan SELURUH berkas besar di luar utas halaman (230 ms untuk 170 KB
   di laptop, jauh lebih lama di HP). Jawabannya baris HTML yang span-nya sudah ditutup per baris. */
self.window = self;
self.IDE = { util: { esc: function (s) { return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); } } };
importScripts('sorot.js');
self.onmessage = function (e) {
  var d = e.data;
  postMessage({ id: d.id, baris: IDE.hl.barisSeimbang(IDE.hl.render(d.teks, d.lang)) });
};
