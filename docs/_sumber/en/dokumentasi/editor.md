---
judul: Editor, REPL, and IDE
menu: Editor and IDE
ringkas: A terminal editor, a language server for your favourite editor, and JRG IDE, which runs in the browser and on phones.
---
## jrgedit

`jrgc --sunting` opens a nano-style full-screen editor. It is written in JRG and ships inside `jrgc`, so it is always
there.

```sh
jrgc --sunting program.jrg
jrgc --sunting a.jrg b.jrg:120     # several files; open b.jrg at line 120
jrgc --sunting project/            # browse a folder
```

| Key | What it does |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>S</kbd> / <kbd>Ctrl</kbd>+<kbd>Q</kbd> | save / quit |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> / <kbd>Ctrl</kbd>+<kbd>Y</kbd> | undo / redo |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> / <kbd>Ctrl</kbd>+<kbd>P</kbd> | find / replace |
| <kbd>Ctrl</kbd>+<kbd>R</kbd> | save and run; afterwards the cursor jumps to the error |
| <kbd>Alt</kbd>+<kbd>P</kbd> | check without running |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | open another file; <kbd>Alt</kbd>+arrow switches files |
| <kbd>Shift</kbd>+arrow, <kbd>Ctrl</kbd>+<kbd>C</kbd>/<kbd>X</kbd>/<kbd>V</kbd> | select, copy, cut, paste |
| <kbd>F1</kbd> or <kbd>Esc</kbd> | the full list of keys |

It has syntax highlighting, self-closing brackets, indentation that follows the previous line, and mouse support.

## REPL

`jrgc` with no arguments opens the REPL. Up/down arrows walk the history, an unclosed block continues on the next
line, and an error does not end the session.

| Command | What it does |
|---|---|
| `:cari word`, `:doc name`, `:daftar ns` | find built-in functions |
| `:tipe expression` | the type, without printing the value |
| `:simpan file.jrg` | write the session out as a runnable program |
| `:riwayat`, `:hapus`, `:keluar` | history, restart, quit |

## Language server (LSP)

`jrgc --lsp` (or `jrgm lsp`) is a Language Server Protocol server. An editor that supports LSP gets: errors and
warnings as you type, name completion (every built-in included), hover information, go to definition, and a file's
symbol list.

For example, in Neovim:

```teks
vim.lsp.start({ name = "jrg", cmd = { "jrgc", "--lsp" }, root_dir = vim.fn.getcwd() })
```

In VS Code, use a generic LSP extension with the command `jrgc --lsp` for `*.jrg` files.

## JRG IDE

JRG IDE is a development environment in the browser, built with JRG itself. It has an editor with highlighting and
completion, a Problems panel (errors and lint warnings), a terminal, a file tree, `jrgm` projects, and a Run button
for JRG and other languages.

```sh
cd ide && jrgm run       # open http://127.0.0.1:8080
```

On phones there are two APK versions:

| | JRG IDE | JRG IDE Terminal |
|---|---|---|
| for | the Play Store | installing directly |
| languages that run | JRG, Lua, JavaScript, SQL, plus Python and Ruby in the page | all of them, plus anything installed in Ubuntu |
| Linux | none | Ubuntu inside the app (`pkg install gcc`, `rustc`, `python3`, …) |
| Native engine (LLVM) | none | after `pkg install clang` in Ubuntu |
| runs in the background | no | yes, with a notification |
