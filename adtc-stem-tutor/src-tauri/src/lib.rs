use llama_cpp_2::context::params::{KvCacheType, LlamaContextParams};
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::llama_batch::LlamaBatch;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
static MODEL: OnceLock<LlamaModel> = OnceLock::new();

const N_THREADS: i32 = 4;
const N_CTX: u32 = 3072;

fn get_backend() -> &'static LlamaBackend {
    BACKEND.get_or_init(|| {
        LlamaBackend::init().expect("Failed to initialize llama-cpp backend")
    })
}

fn get_model() -> Result<&'static LlamaModel, String> {
    if let Some(m) = MODEL.get() {
        return Ok(m);
    }
    let model_path = locate_model_file()?;
    let backend = get_backend();

    let model_params = LlamaModelParams::default()
        .with_n_gpu_layers(0);

    let model = LlamaModel::load_from_file(backend, &model_path, &model_params)
        .map_err(|e| format!("Failed to load model weights: {:?}", e))?;

    let _ = MODEL.set(model);
    Ok(MODEL.get().unwrap())
}

static GEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize)]
struct TokenPayload {
    token: String,
}

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


fn build_context_params() -> LlamaContextParams {
    LlamaContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(N_CTX))
        .with_n_batch(N_CTX)
        .with_n_ubatch(N_CTX)
        .with_n_threads(N_THREADS)
        .with_n_threads_batch(N_THREADS)
        .with_type_k(KvCacheType::Q8_0)
        .with_type_v(KvCacheType::Q8_0)
        .with_offload_kqv(false)
}

#[tauri::command]
async fn stream_stem_tutor_inference(
    app: AppHandle,
    student_prompt: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let _guard = GEN_LOCK.lock().unwrap();

        let model = get_model()?;
        let backend = get_backend();

        let ctx_params = build_context_params();
        let mut ctx: LlamaContext = model.new_context(backend, ctx_params)
            .map_err(|e| format!("Context pool allocation failed: {:?}", e))?;

        let formatted_prompt = if student_prompt.starts_with("<|im_start|>") {
            student_prompt
        } else {
            let system_prompt = "You are a Localized STEM Virtual Lab Tutor. Explain concepts step-by-step. Detail all mathematical derivations, physical laws, and chemical reactions clearly. If asked to translate, support multilingual outputs (e.g., Swahili, French) flawlessly.";
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                system_prompt,
                student_prompt
            )
        };

        let tokens = model.str_to_token(&formatted_prompt, AddBos::Always)
            .map_err(|e| format!("Tokenization engine fault: {:?}", e))?;

        if tokens.len() as u32 > N_CTX {
            return Err(format!(
                "Input string payload exceeds strict {} context ceiling.",
                N_CTX
            ));
        }

        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, token) in tokens.iter().enumerate() {
            let logits = i == tokens.len() - 1;
            batch.add(*token, i as i32, &[0], logits)
                .map_err(|e| format!("Batch allocation error: {:?}", e))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| format!("Prefill decoding pipeline crashed: {:?}", e))?;

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::greedy(),
        ]);

        let mut n_cur = tokens.len() as i32;
        let mut utf8_buffer: Vec<u8> = Vec::new();

        let mut emit_buffer = String::new();
        const EMIT_FLUSH_CHARS: usize = 4;

        while n_cur < N_CTX as i32 {
            let batch_idx = (batch.n_tokens() - 1) as i32;
            let token = sampler.sample(&ctx, batch_idx);

            if token == model.token_eos() {
                break;
            }

            let bytes = model.token_to_piece_bytes(token, 128, true, None)
                .map_err(|e| format!("Detokenizer cycle error: {:?}", e))?;

            utf8_buffer.extend_from_slice(&bytes);

            match std::str::from_utf8(&utf8_buffer) {
                Ok(valid_str) => {
                    emit_buffer.push_str(valid_str);
                    utf8_buffer.clear();
                }
                Err(utf8_err) => {
                    let valid_up_to = utf8_err.valid_up_to();
                    if let Some(error_len) = utf8_err.error_len() {
                        let valid_part = &utf8_buffer[..valid_up_to];
                        emit_buffer.push_str(&String::from_utf8_lossy(valid_part));
                        utf8_buffer = utf8_buffer[valid_up_to + error_len..].to_vec();
                    } else if valid_up_to > 0 {
                        let valid_part = &utf8_buffer[..valid_up_to];
                        emit_buffer.push_str(std::str::from_utf8(valid_part).unwrap());
                        utf8_buffer = utf8_buffer[valid_up_to..].to_vec();
                    }
                }
            }

            if emit_buffer.chars().count() >= EMIT_FLUSH_CHARS {
                app.emit("adtc-token-stream", TokenPayload { token: std::mem::take(&mut emit_buffer) })
                    .map_err(|e| format!("IPC stream emit failed: {:?}", e))?;
            }

            sampler.accept(token);

            batch.clear();
            batch.add(token, n_cur, &[0], true)
                .map_err(|e| format!("Batch rebuild fault: {:?}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Token decoding failure: {:?}", e))?;

            n_cur += 1;
        }

        if !utf8_buffer.is_empty() {
            emit_buffer.push_str(&String::from_utf8_lossy(&utf8_buffer));
        }
        if !emit_buffer.is_empty() {
            let _ = app.emit("adtc-token-stream", TokenPayload { token: emit_buffer });
        }

        Ok(())
    }).await.map_err(|e| format!("Task execution runtime fault: {:?}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = get_model() {
        eprintln!("Warning: failed to pre-load model at startup: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![stream_stem_tutor_inference])
        .run(tauri::generate_context!())
        .expect("Tauri runtime initialization failure");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_model_tokens() {
        let model = get_model().unwrap();

        let bos = model.token_bos();
        let eos = model.token_eos();
        println!("BOS token: {:?}", bos);
        println!("EOS token: {:?}", eos);

        if let Ok(bytes) = model.token_to_piece_bytes(bos, 128, true, None) {
            println!("BOS token detokenized: {:?}", String::from_utf8_lossy(&bytes));
        }
        if let Ok(bytes) = model.token_to_piece_bytes(eos, 128, true, None) {
            println!("EOS token detokenized: {:?}", String::from_utf8_lossy(&bytes));
        }

        // Check token IDs 0 to 20
        for id in 0..20 {
            let token = llama_cpp_2::token::LlamaToken(id);
            if let Ok(bytes) = model.token_to_piece_bytes(token, 128, true, None) {
                println!("Token {}: {:?}", id, String::from_utf8_lossy(&bytes));
            }
        }
    }

    #[test]
    #[ignore]
    fn test_generation() {
        let model = get_model().unwrap();
        let backend = get_backend();

        let ctx_params = build_context_params();
        let mut ctx = model.new_context(backend, ctx_params).unwrap();

        let system_prompt = "You are a Localized STEM Virtual Lab Tutor. Explain concepts step-by-step. Detail all mathematical derivations, physical laws, and chemical reactions clearly. If asked to translate, support multilingual outputs (e.g., Swahili, French) flawlessly.";
        let student_prompt = "Explain the magnetic field lines generated by a solenoid step-by-step. Detail the effect of number of turns, current strength, and relative permeability of a soft iron core.";
        let formatted_prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system_prompt,
            student_prompt
        );

        let tokens = model.str_to_token(&formatted_prompt, AddBos::Always).unwrap();

        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, token) in tokens.iter().enumerate() {
            let logits = i == tokens.len() - 1;
            batch.add(*token, i as i32, &[0], logits).unwrap();
        }

        ctx.decode(&mut batch).unwrap();

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::greedy(),
        ]);

        let mut n_cur = tokens.len() as i32;
        let mut utf8_buffer: Vec<u8> = Vec::new();
        let mut output_str = String::new();

        while n_cur < N_CTX as i32 {
            let batch_idx = (batch.n_tokens() - 1) as i32;
            let token = sampler.sample(&ctx, batch_idx);

            if token == model.token_eos() {
                break;
            }

            let bytes = model.token_to_piece_bytes(token, 128, true, None).unwrap();
            utf8_buffer.extend_from_slice(&bytes);

            match std::str::from_utf8(&utf8_buffer) {
                Ok(valid_str) => {
                    output_str.push_str(valid_str);
                    utf8_buffer.clear();
                }
                Err(utf8_err) => {
                    let valid_up_to = utf8_err.valid_up_to();
                    if let Some(error_len) = utf8_err.error_len() {
                        let valid_part = &utf8_buffer[..valid_up_to];
                        output_str.push_str(&String::from_utf8_lossy(valid_part));
                        utf8_buffer = utf8_buffer[valid_up_to + error_len..].to_vec();
                    } else {
                        if valid_up_to > 0 {
                            let valid_part = &utf8_buffer[..valid_up_to];
                            output_str.push_str(std::str::from_utf8(valid_part).unwrap());
                            utf8_buffer = utf8_buffer[valid_up_to..].to_vec();
                        }
                    }
                }
            }

            sampler.accept(token);

            batch.clear();
            batch.add(token, n_cur, &[0], true).unwrap();

            ctx.decode(&mut batch).unwrap();

            n_cur += 1;
        }

        if !utf8_buffer.is_empty() {
            output_str.push_str(&String::from_utf8_lossy(&utf8_buffer));
        }

        println!("--- Generated Output ---\n{}", output_str);
        assert!(!output_str.is_empty());
        assert!(output_str.contains("solenoid") || output_str.contains("Solenoid"));
    }
}