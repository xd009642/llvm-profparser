#!/bin/bash

# Here we'll just create an empty corpus folder and copy all the profile data
# files into it.

mkdir -p corpus/profile_data/

cp ../tests/data/profdata/llvm-11/* corpus/profile_data/
cp ../tests/data/profdata/llvm-12/* corpus/profile_data/
cp ../tests/data/profdata/llvm-13/* corpus/profile_data/
cp ../tests/data/profdata/llvm-14/* corpus/profile_data/
cp ../tests/data/profdata/llvm-15/* corpus/profile_data/
