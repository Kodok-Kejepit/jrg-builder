---
judul: Benchmark
ringkas: How fast JRG's native backend is compared with C and Go, and how to measure it yourself.
---
The figures below are from the LLVM backend (`--emit-llvm`), measured on an Intel i5-1235U with clang 21, rustc 1.97,
and Go 1.26. Each figure is the best time of six rounds, pinned to one CPU core.

| Kernel | JRG (ms) | C (ms) | JRG : C |
|---|---|---|---|
| `numeric_kernel` | 732 | 748 | 0.98 |
| `call_recursion` | 595 | 589 | 1.01 |
| `struct_particles` | 1044 | 981 | 1.06 |
| `array_sort` | 1060 | 880 | 1.20 |
| `match_dispatch` | 788 | 585 | 1.35 |
| `mixed_report` | 523 | 243 | 2.15 |
| `string_ops` | 516 | 147 | 3.51 |

In short: on a par with C for numbers and function calls, a little behind for structs, arrays, and `match`, and
clearly slower for text processing. Against Go, JRG wins three kernels (`struct_particles` 5.8 times,
`call_recursion` 1.5 times, `match_dispatch` 1.45 times), ties two, and loses two.

The gap in `string_ops` comes from temporary strings that are not freed while the program runs; that is the next
piece of work for this backend.

## Measuring it yourself

```sh
ROUNDS=5 CPU=0 benchmarks/llvm/run.sh
```

For the interpreter and the VM there are nine separate kernels:

```sh
sh benchmarks/acuan/jalankan.sh "$PWD/target/release/jrgc" 3
```

Differences below about 6–10 percent cannot be told apart from measurement noise on an ordinary machine. To prove a
change is faster, count its instructions with `valgrind --tool=callgrind`, rather than only comparing clocks.
