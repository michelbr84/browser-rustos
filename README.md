# Browser for rustOS

A simple terminal-based web browser application for rustOS.

## Installation

In rustOS Terminal:
```
install michelbr84/browser-rustos
grant browser Network
run browser
```

## Features

- 🌐 HTTP GET requests via rustOS networking
- 📄 HTML-to-text conversion for terminal display
- 🔒 Sandboxed execution via WASI

## Building from Source

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
copy target\wasm32-wasip1\release\browser_rustos.wasm app.wasm
```

## License

MIT
