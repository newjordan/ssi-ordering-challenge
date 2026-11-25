# Development corpus

`patterns.jsonl` is the public development corpus: one symmetric CSC sparsity
pattern per line. It is stored with Git LFS. The pointer checked in at this
path records `size 103879806`.

Install Git LFS before cloning, or fetch the blob afterward:

```sh
git lfs install
git lfs pull
```

Without Git LFS, this path is the pointer text rather than JSONL. The harness
stops and tells you to run `git lfs pull`. Tests that need the resolved blob
skip when they still see the pointer.

## Line format

Each line is one pattern:

```json
{"n": 4, "nnz": 12, "indptr": [0, 3, 6, 8, 12], "indices": [0, 1, 3, 0, 1, 3, 2, 3, 0, 1, 2, 3], "hash": "9cce0c0e", "source": "st_e09"}
```

- `n` is the dimension. `indptr` has length `n + 1`. `indices` has one entry per stored nonzero.
- The stored pattern includes the diagonal. `ssi_scoring::pattern_from_jsonl_line` drops the diagonal and requires the off-diagonal pattern to be symmetric.
- `source` is the display name. `hash` is metadata. Neither is an input to `order()`.
- Keys are matched as JSON keys. A `source` value equal to `n` or `indices` does not hide the real field.

## Another corpus for one run

```sh
SSI_CORPUS_FILE=/path/to/other.jsonl cargo run --release --offline --locked -- --note "other corpus"
```

Unset, the harness grades this file. A path that is not this file is treated as a hidden corpus: matrix names stay out of the log. The competition's evaluation corpus is not in this repository.
