use std::sync::OnceLock;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::context::params::{LlamaContextParams, LlamaPoolingType};
use llama_cpp_2::model::AddBos;
use llama_cpp_2::llama_batch::LlamaBatch;
use crate::model::{locate_model_file, detect_thread_split};

#[derive(Clone)]
pub struct CurriculumSnippet {
    pub text: String,
    pub embedding: Vec<f32>,
}

static CURRICULUM: OnceLock<Vec<CurriculumSnippet>> = OnceLock::new();

pub fn get_curriculum_texts() -> Vec<&'static str> {
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

pub fn get_embedding(model: &LlamaModel, backend: &LlamaBackend, text: &str) -> Result<Vec<f32>, String> {
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

pub fn load_or_generate_embeddings(model: &LlamaModel, backend: &LlamaBackend) -> Result<Vec<CurriculumSnippet>, String> {
    let model_path = locate_model_file()?;
    let model_dir = model_path.parent().ok_or("No parent dir for model path")?;
    let bin_path = model_dir.join("embeddings.bin");

    if bin_path.exists() {
        eprintln!("Info: Loading static embeddings from {}", bin_path.display());
        let file = File::open(&bin_path)
            .map_err(|e| format!("Failed to open embeddings.bin: {:?}", e))?;
        let mut reader = BufReader::new(file);
        
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
        let file = File::create(&bin_path)
            .map_err(|e| format!("Failed to create embeddings.bin: {:?}", e))?;
        let mut writer = BufWriter::new(file);
        
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

pub fn get_curriculum(model: &LlamaModel, backend: &LlamaBackend) -> &'static [CurriculumSnippet] {
    CURRICULUM.get_or_init(|| {
        load_or_generate_embeddings(model, backend)
            .expect("Failed to load or generate curriculum embeddings")
    })
}

pub fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
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

pub fn retrieve_top_k(
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
