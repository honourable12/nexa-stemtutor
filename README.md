# Localized STEM Virtual Lab Tutor (ADTC 2026 Submission)

A lightweight, 100% offline desktop application designed for standard budget laptops. It combines real-time interactive physics simulations (running at 60 FPS) with a localized virtual science tutor powered by an offline Large Language Model (LLM).

This project is a submission for the **Africa Deep Tech Challenge (ADTC) 2026** under the **Laptop LLM track** (specifically the `math_scientific_reasoning` domain).

---

## 🚀 Key Features

*   **100% Offline Inference:** Zero external network dependencies or APIs.
*   **Virtual Science Tutor:** Interactive explanations, derivations, and formulas powered by the quantized `Qwen3.5-2B-UD-Q4_K_XL` model (4B parameters).
*   **Real-time 2D Lab Simulations:**
    *   **Electromagnetism (Solenoid):** Adjust turns, current, and core permeability to see dynamic magnetic flux vector lines.
    *   **Pendulum Swing Period:** Adjust gravity and length to visualize simple harmonic motion, comparing theoretical small-angle periods with actual elliptic integrals.
    *   **Ideal Gas Kinetic Theory:** Vary temperature, volume, and particles to view molecular collisions and real-time wall-pressure calculation.
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
    *   **macOS / Linux:** `cmake` and `clang` / `build-essential`.
3.  **Node.js:** Node.js v18+ and `npm`.
4.  **Tauri Prerequisites:** Follow the [Tauri v2 Setup Guide](https://v2.tauri.app/start/prerequisites/) for your operating system.

---

## 💻 Setup, Compilation, and Running

### 1. Download the LLM Weights
From the **root folder** of the project repository (which contains `download_model.sh`), run the shell script to download the GGUF weights:
```bash
bash download_model.sh
```
This script will download the `Qwen3.5-2B-UD-Q4_K_XL.gguf` file (~2.9 GB) from Hugging Face and place it in the `model/` directory.

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
├── model/
│   └── Qwen3.5-2B-UD-Q4_K_XL.gguf  ← Downloaded GGUF weights (Ignored)
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
