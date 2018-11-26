all: build data

build:
	cargo build --release --target=wasm32-unknown-emscripten
	cp target/wasm32-unknown-emscripten/release/deps/project4.wasm public/project4.wasm
	cp target/wasm32-unknown-emscripten/release/project4.js public/project4.js
data: data/
	cp data/* public/",
