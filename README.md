# Nexa Lab
# Localized STEM Virtual Lab Tutor (ADTC 2026 Submission)

A lightweight, 100% offline desktop application designed for standard budget laptops. It combines real-time interactive physics simulations (running at 60 FPS) with a localized virtual science tutor powered by an offline Large Language Model (LLM).

This project is a submission for the **Africa Deep Tech Challenge (ADTC) 2026** under the **Laptop LLM track** (specifically the `math_scientific_reasoning` domain).

---

## 🚀 Key Features

*   **100% Offline Inference:** Zero external network dependencies or APIs.
*   **Virtual Science Tutor:** Interactive explanations, derivations, and formulas powered by the quantized `Qwen2.5-1.5b-instruct-q4_k_m` model.
*   **Real-time 2D Lab Simulations:**
    *   **Optics (Spherical Lenses):** Full interactive support for **both Convex (Converging) and Concave (Diverging) lenses**. Real-time calculation of image distance ($d_i$) and linear magnification ($m$) using the thin lens equation:
        $$\frac{1}{f} = \frac{1}{d_o} + \frac{1}{d_i}$$
        Dynamic ray tracing of three principal rays (parallel, chief, and focal) with correct inverted/upright arrows and dashed virtual backtracking guides.
    *   **Electromagnetism (Solenoid):** Adjust turns, current, and core permeability to see dynamic magnetic flux vector lines.
    *   **Pendulum Swing Period:** Adjust gravity and length to visualize simple harmonic motion, comparing theoretical small-angle periods with actual elliptic integrals.
    *   **Ideal Gas Kinetic Theory:** Vary temperature, volume, and particles to view molecular collisions and real-time wall-pressure calculation.
*   **AI-Guided Experiment Start:** Single-click **Start Experiment** button introduces objectives, physical principles, relevant equations, and suggests parameters to modify.
*   **Lab Report Assistant:** Single-click **Write Lab Report** button generates a complete structured lab report (Title, Objectives, Theory, Setup, Observations, Data Analysis, and Conclusion) using active parameters. Includes a persistent lab report writing guide artifact.
*   **Offline LaTeX & Markdown Rendering:** Fully bundled local `katex` package and custom markdown tokenizer inside Svelte rendering, guaranteeing flawless formula rendering and rich formatting (bold, italic, list items, code snippets) with zero network dependency.
*   **Resource Constraints Optimized:** Pure CPU execution pinned to 4 threads, limited context window of 3072 tokens, and FP16 KV cache to run comfortably on standard 8GB RAM laptops without OOM panics.

---

## 🛠️ Technology Stack

*   **Backend:** Rust + Tauri v2, using the safe `llama-cpp-2` C++ bindings for on-device inference.
*   **Frontend:** Svelte 5 + TypeScript + Vite, using Svelte runes (`$state`, `$derived`, `$effect`) for high-performance reactive SVG/Canvas drawing.
*   **Styles:** Modern Vanilla CSS styled with custom dark-mode aesthetics and animations.

---

## 📋 System Requirements & Prerequisites

To compile and run this application locally, your development machine needs:

1.  **Rust Toolchain:** Install Rust via [rustup](https://rustup.rs/).
2.  **C/C++ Build Tools:**
    *   **Windows:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++ desktop development tools installed, including `MSVC` and `CMake`.
    *   **Linux (Ubuntu/Fedora/etc.):** `cmake`, C/C++ compiler (`gcc`/`g++` or `clang`), and Tauri system dependencies.
3.  **Node.js:** Node.js v18+ and `npm`.

### Linux System Prerequisites
For Linux distributions, Tauri v2 and the `llama.cpp` compiler require several system libraries (Webkit2GTK, GTK3, OpenSSL, CMake, etc.).

A convenience setup script `setup_linux.sh` is provided in the root directory. You can run it to automatically detect your distribution (Ubuntu/Debian-based or Fedora/RHEL-based) and install all required system packages:
```bash
# From the root folder:
bash setup_linux.sh
```

Alternatively, you can install them manually:
*   **Ubuntu/Debian/Mint/Pop!_OS:**
    ```bash
    sudo apt-get update
    sudo apt-get install -y build-essential curl wget pkg-config libssl-dev libgtk-3-dev webkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libayatana-appindicator3-dev librsvg2-dev cmake
    ```
*   **Fedora/Nobara:**
    ```bash
    sudo dnf groupinstall "Development Tools"
    sudo dnf install -y curl wget pkgconf-pkg-config openssl-devel gtk3-devel webkit2gtk4.1-devel libsoup3-devel libayatana-appindicator-devel librsvg2-devel cmake
    ```

---

## 💻 Setup, Compilation, and Running

### 1. Download the LLM Weights
From the **root folder** of the project repository (which contains `download_model.sh`), run the shell script to download the GGUF weights:
```bash
bash download_model.sh
```
This script will download the `qwen2.5-1.5b-instruct-q4_k_m.gguf` file (~2.9 GB) from Hugging Face and place it in the `model/` directory.

### 2. Install Frontend Dependencies
Navigate into the `adtc-stem-tutor` folder and install NPM packages:
```bash
cd adtc-stem-tutor
npm install
```

### 3. Run in Development Mode
To run the Svelte development server and compile the Rust Tauri backend concurrently:
```bash
npm run tauri dev
```
Upon compile completion, the native desktop window will launch. Interacting with the simulations and inputting prompts will stream tokens in real-time from the local LLM.

### 4. Build Optimized Production Executable
To compile a minimized, release-optimized standalone installer:
```bash
npm run tauri build
```
The compiled binaries will be outputted to `src-tauri/target/release/`.

---

## 🔬 Project Directory Structure

```
stemtutor/
├── download_model.sh      ← Root script to download weights
├── metadata.json          ← Submission information and test prompts
├── REPORT.md              ← Technical report (problem, benchmarks)
├── README.md              ← General project overview and setup documentation
├── model/
│   └── qwen2.5-1.5b-instruct-q4_k_m.gguf  ← Downloaded GGUF weights (Ignored)
└── adtc-stem-tutor/       ← Tauri app workspace
    ├── package.json
    ├── svelte.config.js
    ├── vite.config.js
    ├── src/               ← Frontend (Svelte 5 pages & simulations)
    └── src-tauri/         ← Backend (Rust Tauri & llama-cpp-2)
```

---

## 📄 License
This application is licensed under the [GNU GPL v3 License](../LICENSE).
