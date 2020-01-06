SHELL=bash

.PHONY: clean ./builds

test: 
	cargo run test

release: tarball
	. ./scripts/release.sh
	
tarball: ./builds	
	. ./scripts/bundle.sh

./builds: 
	mkdir -p ./builds
	cargo build --release

clean:
	test -e .git/PR_EDITMSG || echo no file
	rm -Rf ./builds
