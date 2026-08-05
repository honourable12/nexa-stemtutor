use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::context::params::{LlamaContextParams, KvCacheType};

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
static MODEL: OnceLock<LlamaModel> = OnceLock::new();

pub const N_CTX: u32 = 3072;
pub const N_BATCH: u32 = 512;

pub fn get_backend() -> &'static LlamaBackend {
    BACKEND.get_or_init(|| {
        LlamaBackend::init().expect("Failed to initialize llama-cpp backend")
    })
}

pub fn get_model() -> Result<&'static LlamaModel, String> {
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

pub fn locate_model_file() -> Result<PathBuf, String> {
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

pub fn build_context_params() -> LlamaContextParams {
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
