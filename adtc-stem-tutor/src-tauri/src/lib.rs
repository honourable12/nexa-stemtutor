use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::llama_batch::LlamaBatch;
use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

// Thread-safe singleton for LlamaBackend to prevent double-initialization errors (BackendAlreadyInitialized)
static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

pub fn get_backend() -> &'static LlamaBackend {
    BACKEND.get_or_init(|| {
        LlamaBackend::init().expect("Failed to initialize llama-cpp backend")
    })
}

#[derive(Clone, Serialize)]
struct TokenPayload {
    token: String,
}

// Tauri command to stream STEM tutor inference step-by-step
#[tauri::command]
async fn stream_stem_tutor_inference(
    app: AppHandle,
    student_prompt: String,
) -> Result<(), String> {
    // We execute the inference in a separate blocking thread because LlamaContext is `!Send` and `!Sync`.
    // Running it entirely within tokio::task::spawn_blocking ensures it stays on a single thread.
    tokio::task::spawn_blocking(move || {
        let backend = get_backend();

        // 1. Load model with zero GPU layers (Integrated graphics only)
        let model_path = Path::new("model/phi-3.5-mini-instruct-q4_k_m.gguf");
        if !model_path.exists() {
            return Err(format!(
                "Model file not found at {:?}. Please download `phi-3.5-mini-instruct-q4_k_m.gguf` (2.43 GB) and place it in the `model/` folder.",
                model_path.canonicalize().unwrap_or(model_path.to_path_buf())
            ));
        }

        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(0); // Pinned to integrated graphics
        
        let model = LlamaModel::load_from_file(backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load model: {:?}", e))?;

        // 2. Configure Context parameters
        // strictly limit context window to 3072 tokens to prevent breaking 8GB RAM limit
        // strictly pin thread execution to 4 threads to maximize performance without context switching overhead
        // Key-Value cache type defaults to FP16 in llama.cpp, optimizing memory footprint.
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(3072))
            .with_n_threads(4)
            .with_n_threads_batch(4);

        let mut ctx = model.new_context(backend, ctx_params)
            .map_err(|e| format!("Failed to create context: {:?}", e))?;

        // 3. Format strict system prompt template matching Phi-3.5-Instruct format
        let system_prompt = "You are a Localized STEM Virtual Lab Tutor, an expert systems tutor for math and scientific reasoning under the Africa Deep Tech Challenge 2026. Explain step-by-step, detail all mathematical derivations, physical laws, and chemical reactions clearly, without any external dependencies. Keep explanations self-contained.";
        let formatted_prompt = format!(
            "<|system|>\n{}<|end|>\n<|user|>\n{}<|end|>\n<|assistant|>\n",
            system_prompt,
            student_prompt
        );

        // 4. Tokenize the formatted prompt
        let tokens = model.str_to_token(&formatted_prompt, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize prompt: {:?}", e))?;

        if tokens.len() > 3072 {
            return Err("Input prompt exceeds the maximum context window of 3072 tokens.".to_string());
        }

        // 5. Initialize the batch and prefill the model
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, token) in tokens.iter().enumerate() {
            let logits = i == tokens.len() - 1; // only calculate logits for the last token in prefill
            batch.add(*token, i as i32, &[0], logits)
                .map_err(|e| format!("Failed to add token to batch: {:?}", e))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode prompt: {:?}", e))?;

        // 6. Set up the greedy sampler for deterministic STEM explanations
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::greedy(),
        ]);

        let mut n_cur = tokens.len() as i32;

        // 7. Generation loop (Zero-network IPC streams tokens back via Tauri window events)
        while n_cur < 3072 {
            let batch_idx = (batch.n_tokens() - 1) as i32;
            let token = sampler.sample(&ctx, batch_idx);

            // Check for Stop tokens (BOS / EOS)
            if token == model.token_eos() || token == model.token_bos() {
                break;
            }

            // Detokenize the generated token to raw bytes and convert to UTF-8 lossily
            let bytes = model.token_to_piece_bytes(token, 128, true, None)
                .map_err(|e| format!("Failed to detokenize token: {:?}", e))?;
            let token_str = String::from_utf8_lossy(&bytes).into_owned();

            // Stream token to frontend via Tauri event system
            app.emit("adtc-token-stream", TokenPayload { token: token_str })
                .map_err(|e| format!("Failed to emit token: {:?}", e))?;

            // Update the sampler state with the accepted token
            sampler.accept(token);

            // Prepare batch with the new token for the next decoding step
            batch.clear();
            batch.add(token, n_cur, &[0], true)
                .map_err(|e| format!("Failed to add token to batch: {:?}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Failed to decode token: {:?}", e))?;

            n_cur += 1;
        }

        Ok(())
    }).await.map_err(|e| format!("Blocking task join error: {:?}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![stream_stem_tutor_inference])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
