#!/bin/sh
for version in `seq 11 21`; do
	tarfile=llvm-${version}.tar
	tag=$(git tag | rg -v -- '-rc[0-9]+$' | rg "llvmorg-${version}\." | sort -h | tail -n1)

	echo "exporting $tag -> $tarfile"
	git archive -o $tarfile $tag llvm/test/tools/llvm-profdata/Inputs/
	tar -xf $tarfile -C ../llvm-profparser/tests/data/profdata/llvm-${version} --strip-components=5
done
