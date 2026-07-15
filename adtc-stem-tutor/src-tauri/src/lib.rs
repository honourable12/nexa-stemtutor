use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::llama_batch::LlamaBatch;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

// Thread-safe singleton for LlamaBackend to prevent double-initialization errors
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

// Helper to systematically find the downloaded GGUF file
fn locate_model_file() -> Result<PathBuf, String> {
    let target_filename = "Qwen3.5-4B-Q4_K_M.gguf";
    let base_paths = [
        format!("model/{}", target_filename),
        format!("../model/{}", target_filename),
        format!("../../model/{}", target_filename),
        format!("src-tauri/model/{}", target_filename),
        format!("../src-tauri/model/{}", target_filename),
    ];

    for path_str in &base_paths {
        let p = Path::new(path_str);
        if p.exists() {
            return Ok(p.to_path_buf());
        }
    }

    Err(format!(
        "Model file '{}' not found. Please verify your download_model.sh script successfully fetched the assets.",
        target_filename
    ))
}

#[tauri::command]
async fn stream_stem_tutor_inference(
    app: AppHandle,
    student_prompt: String,
) -> Result<(), String> {
    let model_path = locate_model_file()?;

    // Execute in separate blocking thread due to LlamaContext raw !Send/!Sync bounds
    tokio::task::spawn_blocking(move || {
        let backend = get_backend();

        // 1. Configure for CPU-only evaluation (essential for integrated sandbox)
        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(0);
        
        let model = LlamaModel::load_from_file(backend, &model_path, &model_params)
            .map_err(|e| format!("Failed to load model weights: {:?}", e))?;

        // 2. Strict profiling configuration bounds (4 vCPUs / 8 GB RAM optimization)
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(3072)) // strict context ceiling
            .with_n_threads(4)                           // pin thread execution
            .with_n_threads_batch(4);

        let mut ctx = model.new_context(backend, ctx_params)
            .map_err(|e| format!("Context pool allocation failed: {:?}", e))?;

        // 3. Format system template matching Qwen 3.5's ChatML structure
        let system_prompt = "You are a Localized STEM Virtual Lab Tutor under the Africa Deep Tech Challenge 2026. Explain concepts step-by-step. Detail all mathematical derivations, physical laws, and chemical reactions clearly. If asked to translate, support multilingual outputs (e.g., Swahili, French) flawlessly.";
        let formatted_prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system_prompt,
            student_prompt
        );

        // 4. Tokenize the input stream
        let tokens = model.str_to_token(&formatted_prompt, AddBos::Always)
            .map_err(|e| format!("Tokenization engine fault: {:?}", e))?;

        if tokens.len() > 3072 {
            return Err("Input string payload exceeds strict 3072 context ceiling.".to_string());
        }

        // 5. Initialize batch and prefill
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, token) in tokens.iter().enumerate() {
            let logits = i == tokens.len() - 1;
            batch.add(*token, i as i32, &[0], logits)
                .map_err(|e| format!("Batch allocation error: {:?}", e))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| format!("Prefill decoding pipeline crashed: {:?}", e))?;

        // 6. Greedy sampler setup for deterministic responses
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::greedy(),
        ]);

        let mut n_cur = tokens.len() as i32;
        
        // Multi-byte buffer array to safely handle sliced unicode UTF-8 characters across boundaries
        let mut utf8_buffer: Vec<u8> = Vec::new();

        // 7. Streamed token generation loop
        while n_cur < 3072 {
            let batch_idx = (batch.n_tokens() - 1) as i32;
            let token = sampler.sample(&ctx, batch_idx);

            if token == model.token_eos() || token == model.token_bos() {
                break;
            }

            // Extract byte representation of token
            let bytes = model.token_to_piece_bytes(token, 128, true, None)
                .map_err(|e| format!("Detokenizer cycle error: {:?}", e))?;
            
            utf8_buffer.extend_from_slice(&bytes);

            // Attempt to decode accumulated bytes safely
            match String::from_utf8(utf8_buffer.clone()) {
                Ok(valid_string) => {
                    app.emit("adtc-token-stream", TokenPayload { token: valid_string })
                        .map_err(|e| format!("IPC stream emit failed: {:?}", e))?;
                    utf8_buffer.clear(); // Flush buffer once successfully decoded
                }
                Err(utf8_err) => {
                    // Check if the error is due to an incomplete sequence at the end of the buffer.
                    // If so, keep the bytes in the buffer and wait for the next token to complete it.
                    if let Some(valid_up_to) = utf8_err.error_len() {
                        // If there are actually invalid bytes, decode what we can lossily or skip them
                        let valid_part = &utf8_buffer[..utf8_err.valid_up_to()];
                        let valid_str = String::from_utf8_lossy(valid_part).into_owned();
                        app.emit("adtc-token-stream", TokenPayload { token: valid_str })
                            .map_err(|e| format!("IPC stream emit failed: {:?}", e))?;
                        
                        // Keep only the remainder of the bytes for the next turn
                        utf8_buffer = utf8_buffer[utf8_err.valid_up_to() + valid_up_to..].to_vec();
                    }
                    // If error_len() is None, we are missing bytes at the end. Leave the buffer as-is to let the next token complete it.
                }
            }

            sampler.accept(token);

            batch.clear();
            batch.add(token, n_cur, &[0], true)
                .map_err(|e| format!("Batch rebuild fault: {:?}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Token decoding failure: {:?}", e))?;

            n_cur += 1;
        }

        // Flush any remaining characters left in the buffer at the end of generation
        if !utf8_buffer.is_empty() {
            let final_str = String::from_utf8_lossy(&utf8_buffer).into_owned();
            let _ = app.emit("adtc-token-stream", TokenPayload { token: final_str });
        }

        Ok(())
    }).await.map_err(|e| format!("Task execution runtime fault: {:?}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![stream_stem_tutor_inference])
        .run(tauri::generate_context!())
        .expect("Tauri runtime initialization failure");
}