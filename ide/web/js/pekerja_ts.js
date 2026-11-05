/* JRG IDE — js/pekerja_ts.js: bahasa yang DIKOMPILASI jadi JavaScript di Web Worker (TypeScript,
   CoffeeScript). TypeScript diperiksa SINTAKSNYA; kecocokan tipe tidak (butuh pustaka tipe yang
   jauh lebih besar). Impor relatif tanpa ekstensi diberi `.js` supaya penjalannya menemukannya. */
function beriEkstensi(js) {
  return js.replace(/(\bfrom\s*|\bimport\s*)(["'])(\.{1,2}\/[^"']*?)\2/g, function (semua, depan, kutip, jalur) {
    return /\.(m?js|json)$/.test(jalur) ? semua : depan + kutip + jalur.replace(/\.(ts|coffee)$/, '') + '.js' + kutip;
  });
}
var KOMPILASI = {
  ts: function (d) {
    if (!self.ts) importScripts(d.akar + 'typescript.min.js');
    var h = ts.transpileModule(d.kode, {
      fileName: d.nama, reportDiagnostics: true,
      compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext, removeComments: false }
    });
    var galat = (h.diagnostics || []).map(function (g) {
      var p = g.file ? g.file.getLineAndCharacterOfPosition(g.start) : null;
      return d.nama + (p ? ':' + (p.line + 1) + ':' + (p.character + 1) : '') + ' ' + ts.flattenDiagnosticMessageText(g.messageText, '\n');
    });
    return { js: h.outputText, galat: galat };
  },
  coffee: function (d) {
    if (!self.CoffeeScript) { self.window = self; importScripts(d.akar + 'coffeescript.js'); }
    try { return { js: CoffeeScript.compile(d.kode, { bare: true, filename: d.nama }), galat: [] }; }
    catch (x) { return { js: '', galat: [String(x).replace(/^\[stdin\]/, d.nama)] }; }
  }
};
onmessage = function (e) {
  var d = e.data;
  try {
    var h = KOMPILASI[d.bahasa || 'ts'](d);
    h.js = beriEkstensi(h.js);
    postMessage(h);
  } catch (x) {
    postMessage({ js: '', galat: [String(x && x.message || x)] });
  }
};
