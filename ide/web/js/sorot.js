/* JRG IDE — js/sorot.js (diturunkan dari penyorot aplikasi Mawar). Keluarannya memuat TEPAT teks
   masukan, hanya dibungkus <span>, tanpa tebal/miring: satu karakter hilang atau melebar membuat
   kursor penyunting meleset. */
(function (M) {
  'use strict';

  var esc = M.util.esc;
  function set(s) { var o = {}; s.split(/\s+/).forEach(function (w) { if (w) o[w] = true; }); return o; }

  var C_LIKE_OPS = '+-*/%=<>!&|^~?:';
  var L = {};
  L.jrg = {
    kw: set('as break chan const continue defer do else enum extern fn for go if impl import in let loop match mod move mut package pub return select self spawn static struct trait type var where while'),
    val: set('true false nil Some None Ok Err Self'),
    ty: set('int uint float f32 f64 bool string rune byte i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize unit array dict tuple'),
    line: ['//'], block: [['/*', '*/', true]], str: ['"', '\''], raw: ['`'], tmpl: [['«', '»'], ['"""', '"""']], ns: true
  };
  L.js = {
    kw: set('async await break case catch class const continue debugger default delete do else export extends finally for from function get if import in instanceof let new of return set static super switch this throw try typeof var void while with yield'),
    val: set('true false null undefined NaN Infinity'), ty: set('Array Object String Number Boolean Promise Map Set Date RegExp Error JSON Math console window document'),
    line: ['//'], block: [['/*', '*/']], str: ['"', '\''], tmplJs: true
  };
  L.ts = L.js;
  L.json = { kw: {}, val: set('true false null'), ty: {}, line: [], block: [], str: ['"'], key: true };
  L.python = {
    kw: set('and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield match case'),
    val: set('True False None self cls'), ty: set('int float str bool list dict tuple set bytes object print len range'),
    line: ['#'], block: [], str: ['"""', '\'\'\'', '"', '\'']
  };
  L.rust = {
    kw: set('as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut pub ref return static struct super trait type unsafe use where while'),
    val: set('true false Some None Ok Err self Self'), ty: set('i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char str String Vec Option Result Box'),
    line: ['//'], block: [['/*', '*/', true]], str: ['"'], ns: true
  };
  L.go = {
    kw: set('break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var'),
    val: set('true false nil iota'), ty: set('int int8 int16 int32 int64 uint uint8 uint16 uint32 uint64 float32 float64 string bool byte rune error any'),
    line: ['//'], block: [['/*', '*/']], str: ['"', '\''], raw: ['`']
  };
  L.c = {
    kw: set('auto break case const continue default do else enum extern for goto if inline register return sizeof static struct switch typedef union volatile while class public private protected virtual template typename namespace using new delete try catch throw override final constexpr nullptr this operator friend explicit'),
    val: set('true false NULL nullptr'), ty: set('int char float double void long short signed unsigned bool size_t uint8_t uint16_t uint32_t uint64_t int8_t int16_t int32_t int64_t std string vector'),
    line: ['//'], block: [['/*', '*/']], str: ['"', '\''], pre: true, ns: true
  };
  L.java = {
    kw: set('abstract assert break case catch class const continue default do else enum extends final finally for goto if implements import instanceof interface native new package private protected public return static strictfp super switch synchronized this throw throws transient try void volatile while var record yield fun val object when is companion data sealed open override internal lateinit suspend'),
    val: set('true false null'), ty: set('int long short byte char float double boolean String Integer Long Double Boolean Object List Map Set Int Unit Any'),
    line: ['//'], block: [['/*', '*/']], str: ['"', '\'']
  };
  L.kotlin = L.java;
  L.cs = L.java;
  L.php = {
    kw: set('abstract and array as break callable case catch class clone const continue declare default do echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile extends final finally fn for foreach function global goto if implements include include_once instanceof insteadof interface isset list match namespace new or print private protected public readonly require require_once return static switch throw trait try unset use var while xor yield'),
    val: set('true false null TRUE FALSE NULL'), ty: set('int float string bool array object mixed void self parent'),
    line: ['//', '#'], block: [['/*', '*/']], str: ['"', '\''], vars: true
  };
  L.sql = {
    kw: set('select from where and or not insert into values update set delete create table drop alter add primary key foreign references join left right inner outer on group by order having limit offset as distinct union all index view if exists null is in like between case when then else end default unique check SELECT FROM WHERE AND OR NOT INSERT INTO VALUES UPDATE SET DELETE CREATE TABLE DROP ALTER ADD PRIMARY KEY FOREIGN REFERENCES JOIN LEFT RIGHT INNER OUTER ON GROUP BY ORDER HAVING LIMIT OFFSET AS DISTINCT UNION ALL INDEX VIEW IF EXISTS NULL IS IN LIKE BETWEEN CASE WHEN THEN ELSE END DEFAULT UNIQUE CHECK'),
    val: set('true false TRUE FALSE'), ty: set('int integer text varchar real float boolean date blob INT INTEGER TEXT VARCHAR REAL FLOAT BOOLEAN DATE BLOB'),
    line: ['--'], block: [['/*', '*/']], str: ['\'', '"']
  };
  L.sh = {
    kw: set('if then else elif fi for while until do done case esac in function return exit export local readonly source alias unset shift break continue'),
    val: set('true false'), ty: set('echo cd ls cat grep sed awk printf read test mkdir rm cp mv chmod'),
    line: ['#'], block: [], str: ['"', '\''], vars: true
  };
  L.yaml = { kw: {}, val: set('true false null yes no on off'), ty: {}, line: ['#'], block: [], str: ['"', '\''], key: true };
  L.toml = { kw: {}, val: set('true false'), ty: {}, line: ['#'], block: [], str: ['"""', '"', '\''], key: true };
  L.lua = { kw: set('and break do else elseif end for function goto if in local not or repeat return then until while'),
    val: set('true false nil self'), ty: set('print pairs ipairs type tonumber tostring table string math io os require pcall error'),
    line: ['--'], block: [['--[[', ']]']], str: ['"', '\''] };
  L.ruby = { kw: set('def end if elsif else unless while until for in do then begin rescue ensure raise class module return yield case when next break require require_relative attr_accessor attr_reader lambda proc'),
    val: set('true false nil self nil?'), ty: set('puts print p String Integer Float Array Hash Symbol Struct Comparable Enumerable'),
    line: ['#'], block: [], str: ['"', '\''], vars: true };
  L.plain = { kw: {}, val: {}, ty: {}, line: [], block: [], str: [] };

  var EXT = {
    jrg: 'jrg', js: 'js', mjs: 'js', cjs: 'js', jsx: 'js', ts: 'ts', tsx: 'ts', json: 'json', webmanifest: 'json', map: 'json',
    py: 'python', pyw: 'python', rs: 'rust', go: 'go', c: 'c', h: 'c', cpp: 'c', cc: 'c', cxx: 'c', hpp: 'c', hh: 'c',
    java: 'java', kt: 'kotlin', kts: 'kotlin', cs: 'cs', php: 'php', sql: 'sql', sh: 'sh', bash: 'sh', zsh: 'sh',
    yml: 'yaml', yaml: 'yaml', toml: 'toml', ini: 'toml', cfg: 'toml', conf: 'toml', properties: 'toml', gradle: 'java',
    html: 'html', htm: 'html', xml: 'html', svg: 'html', xhtml: 'html', vue: 'html', css: 'css', scss: 'css', less: 'css',
    md: 'md', markdown: 'md', txt: 'plain', log: 'plain', csv: 'plain', tsv: 'plain',
    lua: 'lua', rb: 'ruby', ruby: 'ruby'
  };
  function langOf(name) {
    var m = /\.([a-z0-9]+)$/i.exec(name || '');
    return m ? (EXT[m[1].toLowerCase()] || 'plain') : 'plain';
  }

  function span(cls, text) { return '<span class="' + cls + '">' + esc(text) + '</span>'; }
  function isIdStart(c) { return /[A-Za-z_$À-￿]/.test(c); }
  function isId(c) { return /[A-Za-z0-9_$À-￿]/.test(c); }

  function generic(src, g) {
    var out = [], i = 0, n = src.length, text = '';
    function flush() { if (text) { out.push(esc(text)); text = ''; } }
    function push(cls, t) { flush(); out.push(span(cls, t)); }
    function startsWith(s) { return src.substr(i, s.length) === s; }
    while (i < n) {
      var c = src[i], k, j, hit = false;
      /* komentar baris */
      for (k = 0; k < g.line.length; k++) {
        if (startsWith(g.line[k]) && !(g.line[k] === '#' && g.vars && src[i + 1] === '{')) {
          j = src.indexOf('\n', i); if (j < 0) j = n;
          push('hc', src.slice(i, j)); i = j; hit = true; break;
        }
      }
      if (hit) continue;
      /* komentar blok (JRG & Rust: boleh bersarang) */
      for (k = 0; k < g.block.length; k++) {
        var b = g.block[k];
        if (startsWith(b[0])) {
          j = i + b[0].length; var depth = 1;
          while (j < n && depth) {
            if (b[2] && src.substr(j, b[0].length) === b[0]) { depth++; j += b[0].length; }
            else if (src.substr(j, b[1].length) === b[1]) { depth--; j += b[1].length; }
            else j++;
          }
          push('hc', src.slice(i, j)); i = j; hit = true; break;
        }
      }
      if (hit) continue;
      /* templat JRG «...» dan """...""" dengan ${...} */
      if (g.tmpl) {
        for (k = 0; k < g.tmpl.length; k++) {
          var t = g.tmpl[k];
          if (startsWith(t[0])) {
            j = i + t[0].length;
            while (j < n && src.substr(j, t[1].length) !== t[1]) j++;
            j = Math.min(n, j + t[1].length);
            push('hs', src.slice(i, j)); i = j; hit = true; break;
          }
        }
        if (hit) continue;
      }
      /* string */
      if (g.tmplJs && c === '`') {
        j = i + 1;
        while (j < n && src[j] !== '`') { if (src[j] === '\\') j++; j++; }
        j = Math.min(n, j + 1);
        push('hs', src.slice(i, j)); i = j; continue;
      }
      if (g.raw && g.raw.indexOf(c) >= 0) {
        j = src.indexOf(c, i + 1); j = j < 0 ? n : j + 1;
        push('hs', src.slice(i, j)); i = j; continue;
      }
      for (k = 0; k < g.str.length; k++) {
        var q = g.str[k];
        if (startsWith(q)) {
          /* JRG/Rust: 'a' itu rune, tapi 'label: bukan string */
          if (q === '\'' && (g === L.jrg || g === L.rust) && !/^'(\\.|[^\\'\n])'/.test(src.substr(i, 12))) break;
          j = i + q.length;
          while (j < n) {
            if (src[j] === '\\' && q.length === 1) { j += 2; continue; }
            if (src.substr(j, q.length) === q) { j += q.length; break; }
            if (src[j] === '\n' && q.length === 1) break;
            j++;
          }
          j = Math.min(n, j);
          var s = src.slice(i, j);
          if (g.key) {
            var r = /^\s*:/.exec(src.slice(j, j + 20));
            push(r ? 'hk' : 'hs', s);
          } else push('hs', s);
          i = j; hit = true; break;
        }
      }
      if (hit) continue;
      /* variabel $x (PHP, shell) */
      if (g.vars && c === '$' && i + 1 < n && (isIdStart(src[i + 1]) || src[i + 1] === '{')) {
        j = i + 1;
        if (src[j] === '{') { j = src.indexOf('}', j); j = j < 0 ? n : j + 1; }
        else while (j < n && isId(src[j])) j++;
        push('hd', src.slice(i, j)); i = j; continue;
      }
      /* praprosesor C */
      if (g.pre && c === '#' && (i === 0 || src[i - 1] === '\n')) {
        j = src.indexOf('\n', i); if (j < 0) j = n;
        push('hd', src.slice(i, j)); i = j; continue;
      }
      /* angka */
      if (/[0-9]/.test(c) && !(i > 0 && isId(src[i - 1]))) {
        var m = /^(0[xX][0-9a-fA-F_]+|0[bB][01_]+|0[oO][0-7_]+|[0-9][0-9_]*(\.[0-9_]+)?([eE][+-]?[0-9]+)?)([a-z][a-z0-9]*)?/.exec(src.slice(i, i + 60));
        push('hn', m[0]); i += m[0].length; continue;
      }
      /* kata */
      if (isIdStart(c)) {
        j = i + 1;
        while (j < n && isId(src[j])) j++;
        var w = src.slice(i, j);
        var nx = src.slice(j, j + 2);
        var cls = null;
        if (g.kw[w]) cls = 'hk';
        else if (g.val[w]) cls = 'hv';
        else if (g.ty[w]) cls = 'ht';
        else if (g.ns && nx === '::') cls = 'hd';
        else if (nx[0] === '(' || (g === L.jrg && nx[0] === '!')) cls = 'hf';
        else if (g.key && /^\s*[:=]/.test(src.slice(j, j + 8)) && (i === 0 || /\n\s*$/.test(src.slice(Math.max(0, i - 40), i)))) cls = 'hk';
        if (cls) push(cls, w); else text += w;
        i = j; continue;
      }
      if (C_LIKE_OPS.indexOf(c) >= 0) {
        j = i + 1;
        while (j < n && C_LIKE_OPS.indexOf(src[j]) >= 0 && j - i < 3) j++;
        push('ho', src.slice(i, j)); i = j; continue;
      }
      text += c; i++;
    }
    flush();
    return out.join('');
  }

  /* ---------- HTML / XML ---------- */
  function html(src) {
    var out = [], i = 0, n = src.length;
    while (i < n) {
      if (src.substr(i, 4) === '<!--') {
        var e = src.indexOf('-->', i + 4); e = e < 0 ? n : e + 3;
        out.push(span('hc', src.slice(i, e))); i = e; continue;
      }
      if (src[i] === '<' && /[A-Za-z\/!?]/.test(src[i + 1] || '')) {
        var j = i + 1, q = null;
        while (j < n) { var ch = src[j]; if (q) { if (ch === q) q = null; } else if (ch === '"' || ch === '\'') q = ch; else if (ch === '>') break; j++; }
        j = Math.min(n, j + 1);
        var tag = src.slice(i, j);
        out.push(tag.replace(/^(<\/?[!?]?)([\w:.-]*)|(\s)([\w:@.-]+)(=?)("[^"]*"|'[^']*'|[^\s>]*)?|(\/?>)$/g, function (all, open, name, sp, attr, eq, val, close) {
          if (open !== undefined) return esc(open) + (name ? span('hx', name) : '');
          if (attr !== undefined) return sp + span('ha', attr) + (eq ? esc(eq) : '') + (val ? span('hs', val) : '');
          if (close !== undefined) return esc(close);
          return esc(all);
        }));
        var nama = /^<([a-zA-Z]+)/.exec(tag);
        i = j;
        if (nama && /^(script|style)$/i.test(nama[1]) && !/\/>$/.test(tag)) {
          var akhir = src.toLowerCase().indexOf('</' + nama[1].toLowerCase(), i);
          if (akhir < 0) akhir = n;
          var isi = src.slice(i, akhir);
          out.push(nama[1].toLowerCase() === 'script' ? generic(isi, L.js) : css(isi));
          i = akhir;
        }
        continue;
      }
      var k = src.indexOf('<', i + 1); if (k < 0) k = n;
      var teks = src.slice(i, k);
      out.push(esc(teks).replace(/&amp;[a-zA-Z#0-9]+;/g, function (m) { return '<span class="hd">' + m + '</span>'; }));
      i = k;
    }
    return out.join('');
  }

  /* ---------- CSS ---------- */
  function css(src) {
    var out = [], i = 0, n = src.length, dalam = 0;
    while (i < n) {
      var c = src[i];
      if (src.substr(i, 2) === '/*') { var e = src.indexOf('*/', i + 2); e = e < 0 ? n : e + 2; out.push(span('hc', src.slice(i, e))); i = e; continue; }
      if (c === '"' || c === '\'') { var j = i + 1; while (j < n && src[j] !== c && src[j] !== '\n') { if (src[j] === '\\') j++; j++; } j = Math.min(n, j + 1); out.push(span('hs', src.slice(i, j))); i = j; continue; }
      if (c === '{') { dalam++; out.push('{'); i++; continue; }
      if (c === '}') { dalam = Math.max(0, dalam - 1); out.push('}'); i++; continue; }
      if (dalam) {
        var p = /^([-\w]+)(\s*):/.exec(src.slice(i, i + 60));
        if (p && (i === 0 || /[\s;{]/.test(src[i - 1]))) { out.push(span('hk', p[1]) + esc(p[2]) + ':'); i += p[0].length; continue; }
        var num = /^-?\d*\.?\d+(px|em|rem|%|vh|vw|dvh|s|ms|deg|fr|ch|ex|pt)?/.exec(src.slice(i, i + 20));
        if (num && num[0] && !/[\w-]/.test(src[i - 1] || '')) { out.push(span('hn', num[0])); i += num[0].length; continue; }
        var col = /^#[0-9a-fA-F]{3,8}\b/.exec(src.slice(i, i + 10));
        if (col) { out.push(span('hn', col[0])); i += col[0].length; continue; }
        var fn = /^([-\w]+)\(/.exec(src.slice(i, i + 40));
        if (fn && !/[\w-]/.test(src[i - 1] || '')) { out.push(span('hf', fn[1])); i += fn[1].length; continue; }
        if (src.substr(i, 10) === '!important') { out.push(span('hv', '!important')); i += 10; continue; }
        out.push(esc(c)); i++; continue;
      }
      var at = /^@[-\w]+/.exec(src.slice(i, i + 30));
      if (at) { out.push(span('hk', at[0])); i += at[0].length; continue; }
      var sel = /^[^{};/"']+/.exec(src.slice(i));
      if (sel) { out.push(span('hx', sel[0]).replace(/<span class="hx">(\s*)/, '$1<span class="hx">')); i += sel[0].length; continue; }
      out.push(esc(c)); i++;
    }
    return out.join('');
  }

  /* ---------- Markdown (sumber) ---------- */
  function md(src) {
    var dalamKode = false;
    return src.split('\n').map(function (line) {
      if (/^\s*```/.test(line)) { dalamKode = !dalamKode; return span('hd', line); }
      if (dalamKode) return span('hs', line);
      if (/^#{1,6}\s/.test(line)) return span('hk', line);
      if (/^\s*>/.test(line)) return span('hc', line);
      var s = esc(line);
      s = s.replace(/^(\s*)([-*+]|\d+\.)(\s)/, '$1<span class="ho">$2</span>$3');
      s = s.replace(/`[^`]+`/g, function (m) { return '<span class="hs">' + m + '</span>'; });
      s = s.replace(/(\*\*|__)(?=\S)([^<]*?\S)\1/g, '<span class="hv">$1$2$1</span>');
      s = s.replace(/(\[[^\]<]*\])(\([^)<]*\))/g, '<span class="hf">$1</span><span class="hd">$2</span>');
      return s;
    }).join('\n');
  }

  /* HTML sorotan → satu entri per baris, span yang melewati batas baris ditutup dan dibuka lagi. */
  function barisSeimbang(html) {
    var lines = html.split('\n'), buka = [], out = new Array(lines.length), re = /<span class="[^"]*">|<\/span>/g, m;
    for (var i = 0; i < lines.length; i++) {
      var tumpuk = buka.slice();
      re.lastIndex = 0;
      while ((m = re.exec(lines[i]))) { if (m[0] === '</span>') tumpuk.pop(); else tumpuk.push(m[0]); }
      out[i] = buka.join('') + lines[i] + '</span>'.repeat(tumpuk.length);
      buka = tumpuk;
    }
    return out;
  }

  M.hl = {
    barisSeimbang: barisSeimbang,
    langOf: langOf,
    langs: L,
    render: function (code, lang) {
      code = String(code == null ? '' : code);
      if (lang === 'html') return html(code);
      if (lang === 'css') return css(code);
      if (lang === 'md') return md(code);
      return generic(code, L[lang] || L.plain);
    },
    label: function (lang) {
      return ({ jrg: 'JRG', js: 'JavaScript', ts: 'TypeScript', json: 'JSON', python: 'Python', rust: 'Rust', go: 'Go', c: 'C/C++',
        java: 'Java', kotlin: 'Kotlin', cs: 'C#', php: 'PHP', sql: 'SQL', sh: 'Shell', yaml: 'YAML', toml: 'TOML/INI', html: 'HTML/XML',
        css: 'CSS', md: 'Markdown', lua: 'Lua', ruby: 'Ruby', plain: 'Teks' })[lang] || 'Teks';
    }
  };
})(window.IDE);
