# 🏛️ Matrix Discovery

![Matrix Discovery - Event Log Analyzer](https://img.shields.io/badge/Matrix%20Discovery-Event%20Log%20Analyzer-blue)
![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-blue)

Matrix Discovery is a Rust-based web application for processing and analyzing event logs in the XES format. It provides an interface to import, process, and generate XES files.

You can find a live demo of the application at [https://anonymoushlmnop.github.io/matrix-discovery/](https://anonymoushlmnop.github.io/matrix-discovery/)

## ✨ Features

- **Import XES files** for comprehensive analysis
- **Convert text input to XES format** (ideal for testing purposes)
- **Generate adjacency matrices** and other key metrics from event logs
- **Interactive web interface** for visualizing process mining results

## 🔧 Prerequisites

- **Rust and Cargo**: Install from [rustup.rs](https://rustup.rs/)
- **[Trunk](https://trunkrs.dev/)**: WASM web application bundler for Rust
  ```sh
  cargo install trunk
  ```
- **Nix** (optional): Development shell available with:
  ```sh
  nix-shell
  ```

## 🚀 Getting Started

### 1. Clone the repository

```sh
git clone https://github.com/anonymoushlmnop/matrix-discovery.git && cd matrix-discovery
```

### 2. Start the web application

```sh
trunk serve
```

### 3. Access the application

Open your web browser and navigate to [http://localhost:8000](http://localhost:8000)

> **Tips:** 
> - Use `trunk serve --open` to automatically open in your default browser
> - Specify a custom port with `trunk serve --port 1234`

## 📋 Usage Guide

### Importing XES Files
1. Click on the "Import XES" button
2. Select an XES file from your system
3. The file will be processed automatically

### Analyzing Results
After importing, the application will:
- Generate an adjacency matrix based on event traces
- Display the matrix directly in the interface

## 🧩 Core Dependencies

| Dependency | Purpose |
|------------|---------|
| [Yew](https://yew.rs/) | Modern Rust framework for front-end web apps using WebAssembly |
| [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) | High-level interactions between Rust and JavaScript |
| [web-sys](https://rustwasm.github.io/wasm-bindgen/web-sys/) | Bindings for Web APIs |
| [process_mining](https://crates.io/crates/process_mining) | Process mining library for Rust |

## 📜 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## 👏 Acknowledgments

- Thanks to the contributors of the Rust and Yew communities for their support and tools
