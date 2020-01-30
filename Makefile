SHELL=bash

.PHONY: clean ./builds

release: tarball
	. ./scripts/release.sh
	
tarball: ./builds	
	@echo "do build"
	# . ./scripts/bundle.sh

./builds: 
	mkdir -p ./builds
	cargo build --release

clean:
	test -e .git/PR_EDITMSG || echo no file
	rm -Rf ./builds
