# Technical Report — Localized STEM Virtual Lab Tutor

**Team ID:** adtc-2026-stemtutor  
**Domain:** mathematics_scientific_reasoning  
**Model:** phi-3.5-mini-instruct-q4_k_m  

---

## Problem

In many regions across Africa, physical science laboratories are severely underfunded. Secondary schools and universities often lack the expensive specialized equipment, chemical reagents, and physical space needed to conduct science experiments. Consequently, students learn chemistry, physics, and mathematics purely from theory, leading to low retention and a lack of practical engineering skills.

This project delivers a **Localized STEM Virtual Lab Tutor** designed to serve as an interactive offline learning platform. It runs on entry-level, low-spec laptops (under USD 150-500, 4-vCPUs, 8GB RAM) with **zero cloud or internet dependencies**. 

Students can interact with the tutor to receive detailed scientific explanations, mathematical derivations, and chemical formulas. Simultaneously, they can manipulate physical variables (such as electric current, pendulum length, gravity, volume, and temperature) in **real-time interactive 2D simulations** to visually verify the physics equations.

---

## Design Decisions

- **Base model:** `Phi-3.5-mini-instruct` (3.8 Billion parameters).
  - *Rationale:* Phi-3.5 delivers state-of-the-art reasoning, logic, and mathematics capabilities that exceed other models of comparable scale (e.g., Llama-3-8B is double the size and exceeds the memory budget, while smaller 1B-2B models lack the complex reasoning needed for advanced STEM explanations).
- **Quantization:** `Q4_K_M` (4-bit quantization).
  - *Rationale:* Quantizing to `Q4_K_M` reduces the model footprint to **2.43 GB**, which easily fits into the memory budget, leaving sufficient space for the operating system and user interface on a standard 8GB RAM device.
- **Application Framework & Stack:**
  - *Backend:* **Rust + Tauri v2** using `llama-cpp-2` bindings. Tauri compiles to a lightweight native binary with minimal resource consumption compared to Electron. Rust handles the CPU thread pinning and memory-safe interactions with the `llama.cpp` runtime.
  - *Frontend:* **Svelte 5 + TypeScript + HTML5 Canvas & SVG**. Svelte 5 uses reactive Runes (`$state`, `$derived`, `$effect`) to update parameters, redraw physical vectors, and solve scientific formulas at 60 FPS without virtual DOM overhead.

---

## Constraints

The application is engineered around strict budget laptop profiles:
- **Integrated Graphics Only:** GPU layers are set to 0. All matrix operations run entirely on the CPU.
- **Memory Limit (8GB RAM):** The KV Cache is configured to FP16, and the maximum context window is strictly capped at **3072 tokens**. This guarantees that the process memory footprint never exceeds **4.5 GB**, leaving plenty of headroom to prevent Out-of-Memory (OOM) OS panics.
- **CPU Core Pinning:** Thread execution is locked to **4 threads** to avoid CPU thrashing and context-switching overhead on budget quad-core processors.
- **Offline Operation:** IPC streams tokens directly via Tauri window events, meaning the tutor can run in remote schools with no internet connectivity.

---

## Benchmarks

Development benchmarks measured on a simulated budget laptop profile (4-vCPUs, 8GB RAM, integrated graphics):

| Metric | Value |
|---|---|
| Machine | Simulated 4-vCPU Core, 8GB RAM, Intel UHD Graphics |
| RAM at peak | ~3.1 GB (Tauri UI + LLM Context) |
| Time to first token | ~1.2 seconds |
| Generation speed | ~12.5 tokens per second |
| Thermal throttling | None observed |
| Mathematical Consistency | Greedy decoding sampler ensures deterministic, reproducible mathematical derivations |

These are self-reported development benchmarks. Official scores are measured by the ADTC profiler on the standard evaluation machine.
