# llvm-profparser fuzz

This requires cargo-fuzz to be installed and a nightly compiler, to install:

```
cargo install -f cargo-fuzz
```

And then to run for the first time:

```
./setup_corpus.sh
cargo +nightly fuzz run profile_data
```

The script `setup_corpus.sh` copies the test files into the corpus directory in
order to give the fuzzer a good place to start from.
