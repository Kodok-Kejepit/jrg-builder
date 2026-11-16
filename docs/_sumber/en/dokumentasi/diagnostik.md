---
judul: Diagnostics
ringkas: The shape of JRG's error messages, their tone, and how to use them from an editor or CI.
---
## The shape of a message

```teks
berhenti di baris 3. gw tunjukin apa yang salah, di bawah.
error: variabel `jumlas` belum dikenal — maksudnya `jumlah`? kalau bukan, cek lagi penulisannya atau jangan-jangan lupa `let jumlas = ...` dulu
 --> hitung.jrg:3:9
  |
3 |     wok(jumlas)
  |         ^^^^^^
  |
```

1. **A greeting**, one line at the top. The wording varies; `--sopan` drops it.
2. **What is wrong**, with the name you wrote and, when there is one, a spelling suggestion.
3. **Where**: file, line, and column.
4. **A code excerpt** with `^` under the part at fault.
5. **`bantuan:`** (help) gives the way out, and **`catatan:`** (note) extra context, when there is any.
6. **`jejak:`** (trace) for run-time errors: where the failing function was called from.

Messages are in Indonesian today. English messages are on the way, chosen once with `jrgm language en`.

A message names a value's **type**, not its contents. Misusing an array of three numbers or of three thousand gives
an equally short message.

## Tone

The first time an error appears it is always delivered seriously. When the same error comes back, the greeting changes
according to the chosen tone, and at the seventh repetition a help line is added, because the same message seven
times means the explanation has not landed yet.

```sh
jrgc f.jrg --nada hangat     # sopan | galak (bawaan) | judes | hangat | cool | lawak
JRG_NADA=cool jrgc f.jrg
jrgc f.jrg --sopan           # tanpa sapaan sama sekali
```

Tone only changes the greeting line. The body of the message is always the same, so no information is lost in any
tone. The wording is derived from the kind of error and how often it repeated, not rolled at random, so the same
input always gives the same output.

## For editors and CI

- Outside a terminal (piped to a file or another program) there is no colour and no separator line, and the
  repetition counter is not kept between runs.
- `--sopan` or `JRG_SOPAN=1` drops the greeting, which suits comparing output.
- `JRG_GALAT_KE=galat.txt` also writes each error to a file, one line per error: `file<TAB>line<TAB>column<TAB>message`.
- `jrgc --lsp` sends errors and warnings to the editor as you type.

## No silent wrong results

The ground rule: anything that cannot be computed correctly gives an error, not a `0`, a `nil`, or a plausible-looking
empty value. Some examples:

| Case | In JRG |
|---|---|
| index out of range | an error, never clamped |
| `int` overflow | an error, never wraps |
| `300 as u8`, `int("abc")` | an error |
| searching for something that is not there | `Option::None`, never `-1` |
| `x = 5` on a name that does not exist | an error, no new variable is created |
| a built-in called with too few or too many arguments | an error, showing its signature |
| an `Err` leaving `main` | reported, exit code 1 |
