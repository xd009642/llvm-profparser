The data here is mostly exported from LLVM.
You can replicate the export with the `export-llvm.sh` script that lives side-by-side.
Note that the script exports *all* the test fixtures, including sampling and memory profiles, so you should filter out only the instrumented profiles after running it.
To distinguish them, run `cargo profdata -- show <filename>`: if that errors, but passing `--sample` works, it's a sample file.

The following tests are known to be samples:
- tests/data/profdata/llvm-*/compat-sample.profdata (only parsable by LLVM 16 and earlier)
- tests/data/profdata/llvm-*/sample-multiple-nametables.profdata (only parseable by LLVM 16 and earlier)
- all samples in `samples.txt` aside this file.

Additionally, there are many tests that fail when parsed directly by `llvm-profdata`.
I'm not sure what's going on there; perhaps they are fuzz tests.
Those live in `upstream-failures.txt`.
