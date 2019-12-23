run: patch cargo-run check-build clean

build: cargo-build

patch:
	pushd rust-src && git apply --check ../rust-src.patch && git apply ../rust-src.patch && false || popd

cargo-build:
	cargo build

cargo-run:
	cargo run libsyntax libsyntax librustc_parse

check-build:
	pushd rustfmt-syntax && cargo check && false || popd

clean:
	pushd rust-src && git checkout -- . && false || popd
