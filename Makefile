.PHONY: build patch

build: patch cargo-build clean

run: patch cargo-run clean

patch:
	pushd rust-src && git apply --check ../rust-src.patch && git apply ../rust-src.patch && false || popd

cargo-build:
	cargo build

cargo-run:
	cargo run libsyntax libsyntax librustc_parse && pushd rustfmt-syntax && cargo check && false || popd

clean:
	pushd rust-src && git checkout -- . && false || popd
