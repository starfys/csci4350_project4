# rust-webgl2-example

Rust + WebAssembly + WebGL 2.0 project for CSCI4250 based on https://github.com/likr/rust-webgl2-example

# Dependencies
* emscripten sdk
* make
* rustup

# Preparation
This project runs on the current rust stable, so install that
```console
$ rustup update stable
```
You should set an override for this project as well. Navigate into where this is is cloned to do that
```console
$ cd csci4350_project4
$ rustup override set stable
```
You need to install the rustup emscripten backend
```console
$ rustup target add --toolchain stable wasm32-unknown-emscripten
```

# How to build

```console
$ make
```

Then open public/index.html in a browser
