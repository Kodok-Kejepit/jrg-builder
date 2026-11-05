/* JRG IDE — js/pekerja_bahasa.js: bahasa yang penjalannya pustaka JavaScript (Ruby lewat Opal,
   Scheme lewat BiwaScheme, Prolog lewat Tau), tiap run di Web Worker baru supaya bisa dihentikan. */
function kirim(jenis, teks) { postMessage({ jenis: jenis, teks: teks }); }
function selesai(kode) { postMessage({ jenis: 'selesai', kode: kode }); }
/* Pustakanya menyentuh `window`/`document` saat dimuat; di pekerja keduanya tidak ada, jadi
   diberi tiruan yang menjawab apa pun dengan dirinya sendiri. */
var tiruan = function () { return tiruan; };
tiruan = new Proxy(tiruan, {
  get: function (t, k) { return k === Symbol.toPrimitive ? function () { return ''; } : k === 'length' ? 0 : k === 'nodeType' ? 9 : tiruan; },
  set: function () { return true; }, apply: function () { return tiruan; }
});
self.window = self; self.document = tiruan;

var JALAN = {
  ruby: function (d) {
    importScripts(d.akar + 'opal.min.js', d.akar + 'opal-parser.min.js');
    Opal.load('opal-parser');
    var baris = d.masukan ? d.masukan.replace(/\n$/, '').split('\n') : [], i = 0;
    Opal.gvars.stdout.write_proc = function (s) { kirim('keluaran', s); };
    Opal.gvars.stderr.write_proc = function (s) { kirim('galat', s); };
    Opal.gvars.stdin.read_proc = function () { return i < baris.length ? baris[i++] + '\n' : Opal.nil; };
    var js;
    try { js = Opal.compile(d.kode, { file: d.nama }); }
    catch (x) { kirim('galat', d.nama + ': ' + x.message + '\n'); return selesai(1); }
    try { (0, eval)(js); selesai(0); }
    catch (x) {
      kirim('galat', (x.$class ? x.$class().$name() : x.name) + ': ' + x.message + '\n');
      selesai(1);
    }
  },
  scheme: function (d) {
    importScripts(d.akar + 'biwascheme-min.js');
    BiwaScheme.Port.current_output = new BiwaScheme.Port.CustomOutput(function (s) { kirim('keluaran', s); });
    var gagal = false;
    var mesin = new BiwaScheme.Interpreter(function (x) { gagal = true; kirim('galat', String(x && x.message || x) + '\n'); selesai(1); });
    mesin.evaluate(d.kode, function () { if (!gagal) selesai(0); });
  },
  prolog: function (d) {
    importScripts(d.akar + 'core.js', d.akar + 'lists.js');
    /* Tau menulis keluaran lewat console.log, sepotong tiap `write`. */
    console.log = function (s) { kirim('keluaran', String(s)); };
    var sesi = pl.create(1000000);
    sesi.consult(d.kode, {
      script: false,
      success: function () { selesai(0); },
      error: function (x) { kirim('galat', pl.format_answer(x) + '\n'); selesai(1); }
    });
  }
};
onmessage = function (e) {
  var d = e.data;
  try { JALAN[d.bahasa](d); }
  catch (x) { kirim('galat', String(x && x.message || x) + '\n'); selesai(1); }
};
