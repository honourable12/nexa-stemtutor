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

const N_THREADS: i32 = 2;
const N_CTX: u32 = 3072;
const N_BATCH: u32 = 1024;

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

    eprintln!("Info: Loading model from path: {}", model_path.display());
    let start = std::time::Instant::now();

    let model_params = LlamaModelParams::default()
        .with_n_gpu_layers(0)
        .with_use_mmap(true);

    let model = LlamaModel::load_from_file(backend, &model_path, &model_params)
        .map_err(|e| {
            let err_msg = format!("Failed to load model weights from {}: {:?}", model_path.display(), e);
            eprintln!("Error: {}", err_msg);
            err_msg
        })?;

    eprintln!("Info: Model loaded successfully in {:?}", start.elapsed());

    let _ = MODEL.set(model);
    Ok(MODEL.get().unwrap())
}

static GEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize)]
struct TokenPayload {
    token: String,
}

fn locate_model_file() -> Result<PathBuf, String> {
    let target_filename = "Qwen3.5-2B-UD-Q4_K_XL.gguf";

    // 1. Try relative paths from current working directory walking upwards
    if let Ok(cwd) = std::env::current_dir() {
        let mut curr = Some(cwd.as_path());
        while let Some(path) = curr {
            let p_model = path.join("model").join(target_filename);
            if p_model.exists() {
                eprintln!("Info: Found model via CWD search at: {}", p_model.display());
                return Ok(p_model);
            }
            let p_direct = path.join(target_filename);
            if p_direct.exists() {
                eprintln!("Info: Found model via CWD search at: {}", p_direct.display());
                return Ok(p_direct);
            }
            curr = path.parent();
        }
    }

    // 2. Try relative paths from current executable walking upwards
    if let Ok(exe_path) = std::env::current_exe() {
        let mut curr = exe_path.parent();
        while let Some(path) = curr {
            let p_model = path.join("model").join(target_filename);
            if p_model.exists() {
                eprintln!("Info: Found model via EXE search at: {}", p_model.display());
                return Ok(p_model);
            }
            let p_direct = path.join(target_filename);
            if p_direct.exists() {
                eprintln!("Info: Found model via EXE search at: {}", p_direct.display());
                return Ok(p_direct);
            }
            curr = path.parent();
        }
    }

    // 3. Fallback to hardcoded relative search paths
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
            eprintln!("Info: Found model via fallback relative path: {}", p.display());
            return Ok(p.to_path_buf());
        }
    }

    let err_msg = format!(
        "Model file '{}' not found. Looked in CWD parents, EXE parents, and fallback paths. Please verify your download_model.sh script successfully fetched the assets.",
        target_filename
    );
    eprintln!("Error: {}", err_msg);
    Err(err_msg)
}

fn build_context_params() -> LlamaContextParams {
    LlamaContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(N_CTX))
        .with_n_batch(N_BATCH)
        .with_n_ubatch(N_BATCH)
        .with_n_threads(N_THREADS)
        .with_n_threads_batch(N_THREADS)
        // Set KV cache back to standard F16 for fast CPU prefill
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
        let start_time = std::time::Instant::now();
        eprintln!("Info: stream_stem_tutor_inference invoked");

        let _guard = GEN_LOCK.lock().unwrap();

        let model = get_model().map_err(|e| {
            eprintln!("Error loading model: {}", e);
            e
        })?;
        let backend = get_backend();

        let ctx_params = build_context_params();
        let mut ctx: LlamaContext = model.new_context(backend, ctx_params)
            .map_err(|e| {
                let err = format!("Context pool allocation failed: {:?}", e);
                eprintln!("Error: {}", err);
                err
            })?;
        eprintln!("Info: Context allocated in {:?}", start_time.elapsed());

        let formatted_prompt = if student_prompt.starts_with("<|im_start|>") {
            student_prompt
        } else {
            let system_prompt = "You are a Localized STEM Virtual Lab Tutor. Respond directly without thinking tokens or internal reasoning blocks. Explain concepts step-by-step. Detail all mathematical derivations, physical laws, and chemical reactions clearly. Support multilingual outputs (e.g., Swahili, French) flawlessly.";
            
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<think>\n</think>\n",
                system_prompt,
                student_prompt
            )
        };

        let tokens = model.str_to_token(&formatted_prompt, AddBos::Always)
            .map_err(|e| {
                let err = format!("Tokenization engine fault: {:?}", e);
                eprintln!("Error: {}", err);
                err
            })?;

        if tokens.len() as u32 > N_CTX {
            let err = format!(
                "Input string payload exceeds strict {} context ceiling.",
                N_CTX
            );
            eprintln!("Error: {}", err);
            return Err(err);
        }

        let prefill_start = std::time::Instant::now();
        let mut batch = LlamaBatch::new(N_BATCH as usize, 1);
        let mut i = 0;
        while i < tokens.len() {
            let chunk_size = std::cmp::min(tokens.len() - i, N_BATCH as usize);
            batch.clear();
            for j in 0..chunk_size {
                let token_idx = i + j;
                let logits = token_idx == tokens.len() - 1;
                batch.add(tokens[token_idx], token_idx as i32, &[0], logits)
                    .map_err(|e| {
                        let err = format!("Batch allocation error: {:?}", e);
                        eprintln!("Error: {}", err);
                        err
                    })?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| {
                    let err = format!("Prefill decoding pipeline crashed: {:?}", e);
                    eprintln!("Error: {}", err);
                    err
                })?;
            i += chunk_size;
        }
        let prefill_duration = prefill_start.elapsed();
        eprintln!("Info: Prefilled {} tokens in {:?}", tokens.len(), prefill_duration);

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::greedy(),
        ]);

        let mut n_cur = tokens.len() as i32;
        let mut utf8_buffer: Vec<u8> = Vec::new();

        let mut emit_buffer = String::new();

        let gen_start = std::time::Instant::now();
        let mut gen_count = 0;

        while n_cur < N_CTX as i32 {
            let batch_idx = (batch.n_tokens() - 1) as i32;
            let token = sampler.sample(&ctx, batch_idx);

            if token == model.token_eos() {
                break;
            }

            let bytes = model.token_to_piece_bytes(token, 128, false, None)
                .map_err(|e| {
                    let err = format!("Detokenizer cycle error: {:?}", e);
                    eprintln!("Error: {}", err);
                    err
                })?;

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

            if !emit_buffer.is_empty() {
                app.emit("adtc-token-stream", TokenPayload { token: std::mem::take(&mut emit_buffer) })
                    .map_err(|e| {
                        let err = format!("IPC stream emit failed: {:?}", e);
                        eprintln!("Error: {}", err);
                        err
                    })?;
            }

            sampler.accept(token);

            batch.clear();
            batch.add(token, n_cur, &[0], true)
                .map_err(|e| {
                    let err = format!("Batch rebuild fault: {:?}", e);
                    eprintln!("Error: {}", err);
                    err
                })?;

            ctx.decode(&mut batch)
                .map_err(|e| {
                    let err = format!("Token decoding failure: {:?}", e);
                    eprintln!("Error: {}", err);
                    err
                })?;

            n_cur += 1;
            gen_count += 1;
        }

        if !utf8_buffer.is_empty() {
            emit_buffer.push_str(&String::from_utf8_lossy(&utf8_buffer));
        }
        if !emit_buffer.is_empty() {
            let _ = app.emit("adtc-token-stream", TokenPayload { token: emit_buffer });
        }

        let gen_duration = gen_start.elapsed();
        let tps = if gen_duration.as_secs_f32() > 0.0 {
            gen_count as f32 / gen_duration.as_secs_f32()
        } else {
            0.0
        };
        eprintln!(
            "Info: Generated {} tokens in {:?} ({:.2} tokens/sec)",
            gen_count, gen_duration, tps
        );

        Ok(())
    }).await.map_err(|e| format!("Task execution runtime fault: {:?}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("Info: Initializing Tauri application, pre-loading model...");
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

        let mut batch = LlamaBatch::new(N_BATCH as usize, 1);
        let mut i = 0;
        while i < tokens.len() {
            let chunk_size = std::cmp::min(tokens.len() - i, N_BATCH as usize);
            batch.clear();
            for j in 0..chunk_size {
                let token_idx = i + j;
                let logits = token_idx == tokens.len() - 1;
                batch.add(tokens[token_idx], token_idx as i32, &[0], logits).unwrap();
            }
            ctx.decode(&mut batch).unwrap();
            i += chunk_size;
        }

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