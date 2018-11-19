all:
	cargo build --release --target=wasm32-unknown-emscripten
	cp target/wasm32-unknown-emscripten/release/deps/rust_webgl2_example.wasm public/rust-webgl2-example.wasm
	cp target/wasm32-unknown-emscripten/release/rust-webgl2-example.js public/rust-webgl2-example.js
