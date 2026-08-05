use llama_cpp_2::context::params::{KvCacheType, LlamaContextParams, LlamaPoolingType};
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::llama_batch::LlamaBatch;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

mod tools;

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
static MODEL: OnceLock<LlamaModel> = OnceLock::new();

const N_CTX: u32 = 3072;
const N_BATCH: u32 = 512;

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
    let target_filename = "qwen2.5-1.5b-instruct-q4_k_m.gguf";

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

pub fn detect_thread_split() -> (i32, i32) {
    let physical_cores = num_cpus::get_physical() as i32;

    let decode_threads = match physical_cores {
        1..=2 => physical_cores,
        3..=8 => physical_cores - 1,
        _ => 8,
    };

    let prefill_threads = match physical_cores {
        1..=2 => physical_cores,
        _ => (physical_cores - 1).min(12),
    };

    (decode_threads, prefill_threads)
}

fn build_context_params() -> LlamaContextParams {
    let (n_decode, n_prefill) = detect_thread_split();
    
    LlamaContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(N_CTX))
        .with_n_batch(N_BATCH)
        .with_n_ubatch(N_BATCH)
        .with_n_threads(n_decode)
        .with_n_threads_batch(n_prefill)
        .with_type_k(KvCacheType::Q8_0)
        .with_type_v(KvCacheType::Q8_0)
        .with_offload_kqv(false)
}

// RAG / LOCAL VECTOR RETRIEVAL SYSTEM

#[derive(Clone)]
struct CurriculumSnippet {
    text: String,
    embedding: Vec<f32>,
}

static CURRICULUM: OnceLock<Vec<CurriculumSnippet>> = OnceLock::new();

fn get_curriculum_texts() -> Vec<&'static str> {
    vec![
        // Simple Pendulum
        "The period T of a simple pendulum is given by T = 2 * pi * sqrt(L / g), where L is the length of the pendulum string and g is the acceleration due to gravity. The mass of the pendulum bob and the angle of displacement (for small angles) do not affect the period.",
        "Damping in a pendulum represents energy loss due to air resistance or friction. The amplitude of oscillation decreases exponentially over time as E(t) = E_0 * e^(-b*t/m). A larger bob mass has more momentum and decays slower.",
        
        // Spherical Lenses
        "The thin lens equation relates focal length f, object distance d_o, and image distance d_i via 1/f = 1/d_o + 1/d_i. A convex (converging) lens has positive focal length f > 0, while a concave (diverging) lens has negative focal length f < 0.",
        "The magnification m of a lens is defined as m = -d_i / d_o = h_i / h_o. A positive magnification m > 0 indicates a virtual, upright image, whereas a negative magnification m < 0 indicates a real, inverted image.",
        
        // Solenoid
        "The magnetic field B inside a long solenoid is given by B = mu_0 * mu_r * N * I / L, where mu_0 is the permeability of free space, mu_r is the relative permeability of the core material, N is the number of turns, I is the current, and L is the length of the solenoid.",
        "Inserting a ferromagnetic core (like a soft iron core with high relative permeability mu_r) inside a solenoid increases the magnetic flux density B drastically because the core domains align with the field, amplifying the electromagnet's strength.",
        
        // Thermodynamics
        "The Ideal Gas Law is PV = N * k * T (or PV = n * R * T), where P is pressure, V is volume, N is the number of gas particles, T is absolute temperature in Kelvin, and k is the Boltzmann constant (1.380649e-23 J/K).",
        "Kinetic theory of gases states that pressure arises from collision of gas molecules with container walls. Higher temperature increases particle speed (v_rms proportional to sqrt(T)) and collision rate/momentum transfer, increasing pressure.",
        
        // Chemistry
        "In a strong acid-strong base titration (e.g., HCl and NaOH), the equivalence point occurs when the moles of acid equal the moles of base, resulting in a neutral pH of exactly 7.0 at 25 degrees Celsius.",
        "Chemical indicators are weak acids or bases that change color at a specific pH range. For example, Phenolphthalein is colorless in acidic solutions (pH < 8.2) and turns bright pink in basic environments (pH > 10.0)."
    ]
}

fn get_embedding(model: &LlamaModel, backend: &LlamaBackend, text: &str) -> Result<Vec<f32>, String> {
    let mut params = LlamaContextParams::default()
        .with_embeddings(true)
        .with_pooling_type(LlamaPoolingType::Mean)
        .with_n_ctx(std::num::NonZeroU32::new(1024))
        .with_n_batch(512)
        .with_n_ubatch(512);

    let (n_decode, n_prefill) = detect_thread_split();
    params = params.with_n_threads(n_decode).with_n_threads_batch(n_prefill);

    let mut ctx = model.new_context(backend, params)
        .map_err(|e| format!("Failed to allocate embedding context: {:?}", e))?;

    let tokens = model.str_to_token(text, AddBos::Always)
        .map_err(|e| format!("Tokenization failed: {:?}", e))?;

    let tokens_len = tokens.len().min(1024);
    let tokens_slice = &tokens[..tokens_len];

    let mut batch = LlamaBatch::new(tokens_slice.len(), 1);
    for (pos, &token) in tokens_slice.iter().enumerate() {
        batch.add(token, pos as i32, &[0], true)
            .map_err(|e| format!("Failed to add token to batch: {:?}", e))?;
    }

    ctx.decode(&mut batch)
        .map_err(|e| format!("Failed to decode batch for embedding: {:?}", e))?;

    let embedding = ctx.embeddings_seq_ith(0)
        .map_err(|e| format!("Failed to retrieve embeddings: {:?}", e))?;

    Ok(embedding.to_vec())
}

fn load_or_generate_embeddings(model: &LlamaModel, backend: &LlamaBackend) -> Result<Vec<CurriculumSnippet>, String> {
    let model_path = locate_model_file()?;
    let model_dir = model_path.parent().ok_or("No parent dir for model path")?;
    let bin_path = model_dir.join("embeddings.bin");

    if bin_path.exists() {
        eprintln!("Info: Loading static embeddings from {}", bin_path.display());
        let file = std::fs::File::open(&bin_path)
            .map_err(|e| format!("Failed to open embeddings.bin: {:?}", e))?;
        let mut reader = std::io::BufReader::new(file);
        
        use std::io::Read;
        let mut num_snippets_bytes = [0u8; 4];
        reader.read_exact(&mut num_snippets_bytes)
            .map_err(|e| format!("Failed to read num_snippets: {:?}", e))?;
        let num_snippets = u32::from_le_bytes(num_snippets_bytes) as usize;
        
        let mut snippets = Vec::with_capacity(num_snippets);
        for _ in 0..num_snippets {
            let mut text_len_bytes = [0u8; 4];
            reader.read_exact(&mut text_len_bytes)
                .map_err(|e| format!("Failed to read text length: {:?}", e))?;
            let text_len = u32::from_le_bytes(text_len_bytes) as usize;
            
            let mut text_bytes = vec![0u8; text_len];
            reader.read_exact(&mut text_bytes)
                .map_err(|e| format!("Failed to read text bytes: {:?}", e))?;
            let text = String::from_utf8(text_bytes)
                .map_err(|e| format!("Invalid UTF-8 in text: {:?}", e))?;
                
            let mut emb_len_bytes = [0u8; 4];
            reader.read_exact(&mut emb_len_bytes)
                .map_err(|e| format!("Failed to read embedding length: {:?}", e))?;
            let emb_len = u32::from_le_bytes(emb_len_bytes) as usize;
            
            let mut embedding = vec![0.0f32; emb_len];
            let mut float_bytes = vec![0u8; emb_len * 4];
            reader.read_exact(&mut float_bytes)
                .map_err(|e| format!("Failed to read embedding floats: {:?}", e))?;
            
            for i in 0..emb_len {
                let start = i * 4;
                let mut float_bits = [0u8; 4];
                float_bits.copy_from_slice(&float_bytes[start..start+4]);
                embedding[i] = f32::from_le_bytes(float_bits);
            }
            
            snippets.push(CurriculumSnippet { text, embedding });
        }
        
        Ok(snippets)
    } else {
        eprintln!("Info: embeddings.bin not found at {}. Generating from scratch...", bin_path.display());
        let texts = get_curriculum_texts();
        let mut snippets = Vec::new();
        
        for text in texts {
            let embedding = get_embedding(model, backend, text)?;
            snippets.push(CurriculumSnippet {
                text: text.to_string(),
                embedding,
            });
        }
        
        // Write to file
        let file = std::fs::File::create(&bin_path)
            .map_err(|e| format!("Failed to create embeddings.bin: {:?}", e))?;
        let mut writer = std::io::BufWriter::new(file);
        
        use std::io::Write;
        let num_snippets = snippets.len() as u32;
        writer.write_all(&num_snippets.to_le_bytes())
            .map_err(|e| format!("Failed to write num_snippets: {:?}", e))?;
            
        for snippet in &snippets {
            let text_bytes = snippet.text.as_bytes();
            let text_len = text_bytes.len() as u32;
            writer.write_all(&text_len.to_le_bytes())
                .map_err(|e| format!("Failed to write text length: {:?}", e))?;
            writer.write_all(text_bytes)
                .map_err(|e| format!("Failed to write text bytes: {:?}", e))?;
                
            let emb_len = snippet.embedding.len() as u32;
            writer.write_all(&emb_len.to_le_bytes())
                .map_err(|e| format!("Failed to write embedding length: {:?}", e))?;
                
            for &float in &snippet.embedding {
                writer.write_all(&float.to_le_bytes())
                    .map_err(|e| format!("Failed to write embedding float: {:?}", e))?;
            }
        }
        writer.flush().map_err(|e| format!("Failed to flush embeddings writer: {:?}", e))?;
        eprintln!("Info: embeddings.bin successfully generated and saved.");
        
        Ok(snippets)
    }
}

fn get_curriculum(model: &LlamaModel, backend: &LlamaBackend) -> &'static [CurriculumSnippet] {
    CURRICULUM.get_or_init(|| {
        load_or_generate_embeddings(model, backend)
            .expect("Failed to load or generate curriculum embeddings")
    })
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..v1.len().min(v2.len()) {
        dot_product += v1[i] * v2[i];
        norm_a += v1[i] * v1[i];
        norm_b += v2[i] * v2[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a.sqrt() * norm_b.sqrt())
    }
}

fn retrieve_top_k(
    query_embedding: &[f32],
    snippets: &[CurriculumSnippet],
    k: usize,
) -> Vec<String> {
    let mut scored_snippets: Vec<(f32, &String)> = snippets.iter()
        .map(|s| {
            let sim = cosine_similarity(query_embedding, &s.embedding);
            (sim, &s.text)
        })
        .collect();
        
    scored_snippets.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    
    scored_snippets.into_iter()
        .take(k)
        .map(|(_, text)| text.clone())
        .collect()
}

fn parse_chatml_prompt(prompt: &str) -> (String, String) {
    let sys_start = "<|im_start|>system\n";
    let sys_end = "<|im_end|>\n<|im_start|>user\n";
    let user_end = "<|im_end|>\n<|im_start|>assistant\n";

    if let Some(sys_start_idx) = prompt.find(sys_start) {
        if let Some(sys_end_idx) = prompt.find(sys_end) {
            let sys_content = &prompt[sys_start_idx + sys_start.len()..sys_end_idx];
            if let Some(user_end_idx) = prompt.find(user_end) {
                let user_content = &prompt[sys_end_idx + sys_end.len()..user_end_idx];
                return (sys_content.to_string(), user_content.to_string());
            }
        }
    }
    ("".to_string(), prompt.to_string())
}

fn parse_state_from_sys_prompt(sys_content: &str) -> String {
    if sys_content.contains("Simple Pendulum") {
        let l = extract_value(sys_content, "length (L) = ", " meters").unwrap_or_else(|| "1.00".to_string());
        let angle = extract_value(sys_content, "initial swing angle (theta) = ", " degrees").unwrap_or_else(|| "30".to_string());
        let g = extract_value(sys_content, "gravity (g) = ", " m/s^2").unwrap_or_else(|| "9.81".to_string());
        format!("Module=Mechanics, L={}m, Angle={}deg, g={}m/s^2", l, angle, g)
    } else if sys_content.contains("Optics") || sys_content.contains("Thin Lens") {
        let d_o = extract_value(sys_content, "object distance (d_o) = ", " cm").unwrap_or_else(|| "45".to_string());
        let f = extract_value(sys_content, "focal length (f) = ", " cm").unwrap_or_else(|| "20".to_string());
        format!("Module=Optics, d_o={}cm, f={}cm", d_o, f)
    } else if sys_content.contains("Thermodynamics") || sys_content.contains("Ideal Gas") {
        let t = extract_value(sys_content, "gas temperature (T) = ", " Kelvin").unwrap_or_else(|| "300".to_string());
        let v = extract_value(sys_content, "container volume factor (V) = ", ",").unwrap_or_else(|| "1.00".to_string());
        let n = extract_value(sys_content, "particle count (N) = ", ".").unwrap_or_else(|| "60".to_string());
        format!("Module=Thermodynamics, T={}K, V={}, N={}", t, v, n)
    } else if sys_content.contains("Electromagnetism") || sys_content.contains("Solenoid") {
        let turns = extract_value(sys_content, "number of turns (N) = ", ",").unwrap_or_else(|| "20".to_string());
        let current = extract_value(sys_content, "electric current (I) = ", " Amperes").unwrap_or_else(|| "2.0".to_string());
        let mu_r = extract_value(sys_content, "relative permeability of core (mu_r) = ", ",").unwrap_or_else(|| "500".to_string());
        format!("Module=Electromagnetism, N={}, I={}A, mu_r={}", turns, current, mu_r)
    } else if sys_content.contains("Chemistry") || sys_content.contains("Titration") {
        let titrant = extract_value(sys_content, "titrant = ", ",").unwrap_or_else(|| "NaOH".to_string());
        let analyte = extract_value(sys_content, "analyte = ", ",").unwrap_or_else(|| "HCl".to_string());
        let t_conc = extract_value(sys_content, "titrant concentration = ", " M").unwrap_or_else(|| "0.10".to_string());
        let a_vol = extract_value(sys_content, "analyte volume = ", " mL").unwrap_or_else(|| "25.0".to_string());
        let ph = extract_value(sys_content, "current pH = ", ",").unwrap_or_else(|| "7.00".to_string());
        format!("Module=Chemistry, Titrant={}, Analyte={}, TitrantConc={}M, AnalyteVol={}mL, pH={}", titrant, analyte, t_conc, a_vol, ph)
    } else {
        "Module=General".to_string()
    }
}

fn extract_value(text: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = text.find(prefix)? + prefix.len();
    let end = text[start..].find(suffix)?;
    Some(text[start..start + end].trim().to_string())
}

fn prefill_tokens(
    model: &LlamaModel,
    ctx: &mut LlamaContext,
    batch: &mut LlamaBatch,
    tokens: &[llama_cpp_2::token::LlamaToken],
) -> Result<(), String> {
    let mut i = 0;
    while i < tokens.len() {
        let chunk_size = std::cmp::min(tokens.len() - i, N_BATCH as usize);
        batch.clear();
        for j in 0..chunk_size {
            let token_idx = i + j;
            let logits = token_idx == tokens.len() - 1;
            batch.add(tokens[token_idx], token_idx as i32, &[0], logits)
                .map_err(|e| format!("Batch allocation error: {:?}", e))?;
        }
        ctx.decode(batch)
            .map_err(|e| format!("Prefill decoding pipeline crashed: {:?}", e))?;
        i += chunk_size;
    }
    Ok(())
}

fn run_inference_loop(
    model: &LlamaModel,
    ctx: &mut LlamaContext,
    batch: &mut LlamaBatch,
    app: &AppHandle,
    _is_generating: &mut bool,
    n_cur: &mut i32,
    stream_to_ui: bool,
    check_prefix: bool,
) -> Result<(String, bool), String> {
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::greedy(),
    ]);

    let mut utf8_buffer: Vec<u8> = Vec::new();
    let mut generated_text = String::new();
    let mut prefix_buffer = String::new();
    let mut checking_prefix = check_prefix;
    let mut is_tool_call = false;

    while *n_cur < N_CTX as i32 {
        let batch_idx = (batch.n_tokens() - 1) as i32;
        let token = sampler.sample(ctx, batch_idx);

        if token == model.token_eos() {
            break;
        }

        let bytes = model.token_to_piece_bytes(token, 128, false, None)
            .map_err(|e| format!("Detokenizer cycle error: {:?}", e))?;

        utf8_buffer.extend_from_slice(&bytes);

        let mut decoded_piece = String::new();
        match std::str::from_utf8(&utf8_buffer) {
            Ok(valid_str) => {
                decoded_piece.push_str(valid_str);
                utf8_buffer.clear();
            }
            Err(utf8_err) => {
                let valid_up_to = utf8_err.valid_up_to();
                if let Some(error_len) = utf8_err.error_len() {
                    let valid_part = &utf8_buffer[..valid_up_to];
                    decoded_piece.push_str(&String::from_utf8_lossy(valid_part));
                    utf8_buffer = utf8_buffer[valid_up_to + error_len..].to_vec();
                } else if valid_up_to > 0 {
                    let valid_part = &utf8_buffer[..valid_up_to];
                    decoded_piece.push_str(std::str::from_utf8(valid_part).unwrap());
                    utf8_buffer = utf8_buffer[valid_up_to..].to_vec();
                }
            }
        }

        if !decoded_piece.is_empty() {
            generated_text.push_str(&decoded_piece);
            
            if checking_prefix {
                prefix_buffer.push_str(&decoded_piece);
                if prefix_buffer.len() >= 11 {
                    checking_prefix = false;
                    if prefix_buffer.starts_with("<tool_call>") {
                        is_tool_call = true;
                    } else {
                        if stream_to_ui {
                            app.emit("adtc-token-stream", TokenPayload { token: std::mem::take(&mut prefix_buffer) })
                                .map_err(|e| format!("IPC stream emit failed: {:?}", e))?;
                        }
                    }
                } else {
                    if !b"<tool_call>".starts_with(prefix_buffer.as_bytes()) {
                        checking_prefix = false;
                        if stream_to_ui {
                            app.emit("adtc-token-stream", TokenPayload { token: std::mem::take(&mut prefix_buffer) })
                                .map_err(|e| format!("IPC stream emit failed: {:?}", e))?;
                        }
                    }
                }
            } else {
                if !is_tool_call && stream_to_ui {
                    app.emit("adtc-token-stream", TokenPayload { token: decoded_piece })
                        .map_err(|e| format!("IPC stream emit failed: {:?}", e))?;
                }
            }
        }

        sampler.accept(token);

        batch.clear();
        batch.add(token, *n_cur, &[0], true)
            .map_err(|e| format!("Batch rebuild fault: {:?}", e))?;

        ctx.decode(batch)
            .map_err(|e| format!("Token decoding failure: {:?}", e))?;

        *n_cur += 1;
    }

    if !utf8_buffer.is_empty() {
        let remaining = String::from_utf8_lossy(&utf8_buffer).into_owned();
        generated_text.push_str(&remaining);
        if checking_prefix {
            if !prefix_buffer.starts_with("<tool_call>") && stream_to_ui {
                let _ = app.emit("adtc-token-stream", TokenPayload { token: prefix_buffer + &remaining });
            }
        } else if !is_tool_call && stream_to_ui {
            let _ = app.emit("adtc-token-stream", TokenPayload { token: remaining });
        }
    } else if checking_prefix {
        if !prefix_buffer.starts_with("<tool_call>") && stream_to_ui {
            let _ = app.emit("adtc-token-stream", TokenPayload { token: prefix_buffer });
        }
    }

    Ok((generated_text, is_tool_call))
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

        // 1. Parse prompt and extract user prompt / active state
        let (sys_content, user_content) = parse_chatml_prompt(&student_prompt);
        let active_state = parse_state_from_sys_prompt(&sys_content);
        eprintln!("Info: Parsed Active State: {}", active_state);

        // 2. Perform grounding RAG (retrieve context)
        let query_emb = get_embedding(model, backend, &user_content)
            .map_err(|e| {
                let err = format!("Failed to generate query embedding: {}", e);
                eprintln!("Error: {}", err);
                err
            })?;
            
        let curriculum = get_curriculum(model, backend);
        let retrieved_snippets = retrieve_top_k(&query_emb, curriculum, 2);
        eprintln!("Info: Retrieved grounding snippets");

        // 3. Build prompt with tools, active simulation state, and retrieved snippets
        let language_instruction = if sys_content.contains("en français") || sys_content.contains("Vous êtes") {
            "You are Nexa, a Localized Interactive STEM Virtual Lab Tutor. Répondez directement et uniquement en français. Pas de blocs de pensée ou de balises <think>. Détaillez clairement les dérivations mathématiques étape par étape."
        } else if sys_content.contains("Kiswahili") || sys_content.contains("Wewe ni Mwalimu") {
            "You are Nexa, a Localized Interactive STEM Virtual Lab Tutor. Jibu moja kwa moja na kwa Kiswahili pekee. Usitoe mawazo au lebo za <think>. Eleza hatua kwa hatua makadirio yote ya hesabu."
        } else {
            "You are Nexa, a Localized Interactive STEM Virtual Lab Tutor. Direct answer only in English. Do not output thinking blocks or <think> tags. Detail mathematical derivations step-by-step cleanly."
        };
        
        let snippet_1 = retrieved_snippets.get(0).cloned().unwrap_or_default();
        let snippet_2 = retrieved_snippets.get(1).cloned().unwrap_or_default();
        
        let first_pass_prompt = format!(
            "<|im_start|>system\n{}\n[Active Simulation State: {}]\n\n# Tools\nWhen a numeric computation or exact calculation is requested, invoke a function instead of guessing.\n<tools>\n{}\n</tools>\n\nTo call a function, respond strictly in this format:\n<tool_call>{{\"name\": \"...\", \"arguments\": {{...}}}}</tool_call>\n\n# Grounding Reference Material\n- {}\n- {}\n<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            language_instruction,
            active_state,
            crate::tools::TOOL_DEFINITIONS,
            snippet_1,
            snippet_2,
            user_content
        );

        // 4. Set up context and decode prompt
        let ctx_params = build_context_params();
        let mut ctx: LlamaContext = model.new_context(backend, ctx_params)
            .map_err(|e| format!("Context pool allocation failed: {:?}", e))?;
            
        let tokens = model.str_to_token(&first_pass_prompt, AddBos::Always)
            .map_err(|e| format!("Tokenization engine fault: {:?}", e))?;

        if tokens.len() as u32 > N_CTX {
            return Err(format!("Input string payload exceeds strict {} context ceiling.", N_CTX));
        }

        let mut batch = LlamaBatch::new(N_BATCH as usize, 1);
        prefill_tokens(model, &mut ctx, &mut batch, &tokens)?;

        let mut n_cur = tokens.len() as i32;
        let mut is_generating = true;

        // 5. First pass inference
        let (first_pass_result, is_tool_call) = run_inference_loop(
            model,
            &mut ctx,
            &mut batch,
            &app,
            &mut is_generating,
            &mut n_cur,
            true, // stream to UI (if it's not a tool call, this will stream it)
            true, // check prefix for <tool_call>
        )?;

        // 6. Handle tool call if detected
        if is_tool_call {
            eprintln!("Info: Tool call detected in first pass: {}", first_pass_result);
            if let Some(start_idx) = first_pass_result.find("<tool_call>") {
                if let Some(end_idx) = first_pass_result.find("</tool_call>") {
                    let json_str = &first_pass_result[start_idx + 11..end_idx];
                    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(json_str) {
                        let name = map.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let args = map.get("arguments").unwrap_or(&Value::Null);

                        // Execute the tool in Rust
                        let tool_result = crate::tools::execute_tool(name, args);
                        eprintln!("Info: Executed tool {}, result: {}", name, tool_result);

                        // Build second pass prompt
                        let second_pass_prompt = format!(
                            "{}{}<|im_end|>\n<|im_start|>system\n{}<|im_end|>\n<|im_start|>assistant\n",
                            first_pass_prompt,
                            first_pass_result,
                            tool_result.to_string()
                        );

                        // Re-tokenize and prefill for the second pass context
                        let second_pass_tokens = model.str_to_token(&second_pass_prompt, AddBos::Always)
                            .map_err(|e| format!("Second pass tokenization failed: {:?}", e))?;

                        if second_pass_tokens.len() as u32 > N_CTX {
                            return Err(format!("Second pass prompt exceeds strict {} context ceiling.", N_CTX));
                        }

                        // Clear context's kv cache and prefill the new tokens
                        ctx.clear_kv_cache();

                        let mut second_batch = LlamaBatch::new(N_BATCH as usize, 1);
                        prefill_tokens(model, &mut ctx, &mut second_batch, &second_pass_tokens)?;

                        let mut second_n_cur = second_pass_tokens.len() as i32;

                        // Second pass: stream directly to UI (no prefix check needed)
                        run_inference_loop(
                            model,
                            &mut ctx,
                            &mut second_batch,
                            &app,
                            &mut is_generating,
                            &mut second_n_cur,
                            true, // stream to UI
                            false, // do not check prefix (this is the final response)
                        )?;
                    }
                }
            }
        }

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