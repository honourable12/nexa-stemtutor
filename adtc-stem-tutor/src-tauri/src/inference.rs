use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use serde::Serialize;
use serde_json::Value;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::model::AddBos;

use crate::model::{get_model, get_backend, build_context_params, N_CTX, N_BATCH};
use crate::rag::{get_curriculum, retrieve_top_k, get_embedding};

pub static GEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize)]
pub struct TokenPayload {
    pub token: String,
}

pub fn parse_chatml_prompt(prompt: &str) -> (String, String) {
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

pub fn parse_state_from_sys_prompt(sys_content: &str) -> String {
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

pub fn prefill_tokens(
    _model: &LlamaModel,
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

pub fn run_inference_loop(
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
pub async fn stream_stem_tutor_inference(
    app: AppHandle,
    student_prompt: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
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
        let mut ctx = model.new_context(backend, ctx_params)
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
