# llvm-profparser

[![Build Status](https://github.com/xd009642/llvm-profparser/workflows/Build/badge.svg)](https://github.com/xd009642/llvm-profparser/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Coverage Status](https://coveralls.io/repos/github/xd009642/llvm-profparser/badge.svg?branch=master)](https://coveralls.io/github/xd009642/llvm-profparser?branch=master)

This is a WIP to parse the llvm instrumentation profraw file format and avoid
the need to install and use the llvm-profdata binary. 

**This project is not affilated with the llvm-project in anyway!** It is merely
a parser for some of their file formats to aid interoperability in Rust.

## License

llvm\_profparser is currently licensed under the terms of the Apache License
(Version 2.0). See LICENSE for details. Test data included from the llvm-project
residing in `tests/data` retains the llvm license. See the llvm-project for 
details. 
