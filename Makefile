all: build

build:
	cargo build --release --target=wasm32-unknown-emscripten
	cp target/wasm32-unknown-emscripten/release/deps/project4.wasm public/project4.wasm
	cp target/wasm32-unknown-emscripten/release/project4.js public/project4.js
debug:
	cargo build --target=wasm32-unknown-emscripten
	cp target/wasm32-unknown-emscripten/debug/deps/project4.wasm public/project4.wasm
	cp target/wasm32-unknown-emscripten/debug/project4.js public/project4.js
deploy:
	git add -f public/project4.wasm public/project4.js
	git commit -m "Updated built project"
package:
	zip -r project4.zip public/*
