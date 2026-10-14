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
    pub lang: &'static str,
}

static CURRICULUM: OnceLock<Vec<CurriculumSnippet>> = OnceLock::new();

/// English curriculum (10 snippets, 2 per simulation module).
pub fn get_curriculum_texts_en() -> Vec<(&'static str, &'static str)> {
    vec![
        // Simple Pendulum
        ("The period T of a simple pendulum is given by T = 2 * pi * sqrt(L / g), where L is the length of the pendulum string and g is the acceleration due to gravity. The mass of the pendulum bob and the angle of displacement (for small angles) do not affect the period.", "en"),
        ("Damping in a pendulum represents energy loss due to air resistance or friction. The amplitude of oscillation decreases exponentially over time as E(t) = E_0 * e^(-b*t/m). A larger bob mass has more momentum and decays slower.", "en"),

        // Spherical Lenses
        ("The thin lens equation relates focal length f, object distance d_o, and image distance d_i via 1/f = 1/d_o + 1/d_i. A convex (converging) lens has positive focal length f > 0, while a concave (diverging) lens has negative focal length f < 0.", "en"),
        ("The magnification m of a lens is defined as m = -d_i / d_o = h_i / h_o. A positive magnification m > 0 indicates a virtual, upright image, whereas a negative magnification m < 0 indicates a real, inverted image.", "en"),

        // Solenoid
        ("The magnetic field B inside a long solenoid is given by B = mu_0 * mu_r * N * I / L, where mu_0 is the permeability of free space, mu_r is the relative permeability of the core material, N is the number of turns, I is the current, and L is the length of the solenoid.", "en"),
        ("Inserting a ferromagnetic core (like a soft iron core with high relative permeability mu_r) inside a solenoid increases the magnetic flux density B drastically because the core domains align with the field, amplifying the electromagnet's strength.", "en"),

        // Thermodynamics
        ("The Ideal Gas Law is PV = N * k * T (or PV = n * R * T), where P is pressure, V is volume, N is the number of gas particles, T is absolute temperature in Kelvin, and k is the Boltzmann constant (1.380649e-23 J/K).", "en"),
        ("Kinetic theory of gases states that pressure arises from collision of gas molecules with container walls. Higher temperature increases particle speed (v_rms proportional to sqrt(T)) and collision rate/momentum transfer, increasing pressure.", "en"),

        // Chemistry
        ("In a strong acid-strong base titration (e.g., HCl and NaOH), the equivalence point occurs when the moles of acid equal the moles of base, resulting in a neutral pH of exactly 7.0 at 25 degrees Celsius.", "en"),
        ("Chemical indicators are weak acids or bases that change color at a specific pH range. For example, Phenolphthalein is colorless in acidic solutions (pH < 8.2) and turns bright pink in basic environments (pH > 10.0).", "en"),
    ]
}

/// French curriculum (10 snippets). v0.2: hand-translated, scientific terms
/// preserved. Native-speaker review pass recommended before shipping.
pub fn get_curriculum_texts_fr() -> Vec<(&'static str, &'static str)> {
    vec![
        // Pendule simple
        ("La periode T d'un pendule simple est donnee par T = 2 * pi * sqrt(L / g), ou L est la longueur du fil du pendule et g est l'acceleration due a la gravite. La masse du bob et l'angle de declinaison (pour de petits angles) n'affectent pas la periode.", "fr"),
        ("L'amortissement dans un pendule represente la perte d'energie due a la resistance de l'air ou aux frottements. L'amplitude d'oscillation decroit de maniere exponentielle : E(t) = E_0 * e^(-b*t/m). Une masse plus grande a plus de quantite de mouvement et decroit plus lentement.", "fr"),

        // Lentilles spheriques
        ("L'equation des lentilles minces relie la distance focale f, la distance objet d_o et la distance image d_i via 1/f = 1/d_o + 1/d_i. Une lentille convexe (convergente) a une distance focale positive f > 0, tandis qu'une lentille concave (divergente) a une distance focale negative f < 0.", "fr"),
        ("Le grandissement m d'une lentille est defini par m = -d_i / d_o = h_i / h_o. Un grandissement positif m > 0 indique une image virtuelle et droite, tandis qu'un grandissement negatif m < 0 indique une image reelle et inversee.", "fr"),

        // Solenoide
        ("Le champ magnetique B a l'interieur d'un long solenoide est donne par B = mu_0 * mu_r * N * I / L, ou mu_0 est la permeabilite du vide, mu_r est la permeabilite relative du materiau du noyau, N est le nombre de spires, I est le courant et L est la longueur du solenoide.", "fr"),
        ("L'insertion d'un noyau ferromagnetique (comme un noyau de fer doux a haute permeabilite relative mu_r) a l'interieur d'un solenoide augmente considerablement la densite de flux magnetique B, car les domaines du noyau s'alignent avec le champ, amplifiant la force de l'electroaimant.", "fr"),

        // Thermodynamique
        ("La loi des gaz parfaits est PV = N * k * T (ou PV = n * R * T), ou P est la pression, V est le volume, N est le nombre de particules de gaz, T est la temperature absolue en Kelvin, et k est la constante de Boltzmann (1.380649e-23 J/K).", "fr"),
        ("La theorie cinetique des gaz stipule que la pression resulte des collisions des molecules de gaz avec les parois du recipient. Une temperature plus elevee augmente la vitesse des particules (v_rms proportionnel a sqrt(T)) et le taux de collision et de transfert de quantite de mouvement, augmentant ainsi la pression.", "fr"),

        // Chimie
        ("Dans un titrage acide fort - base forte (par exemple HCl et NaOH), le point d'equivalence est atteint lorsque les moles d'acide egalent les moles de base, ce qui donne un pH neutre d'exactement 7,0 a 25 degres Celsius.", "fr"),
        ("Les indicateurs chimiques sont des acides ou des bases faibles qui changent de couleur dans une plage de pH specifique. Par exemple, la phenolphtaleine est incolore dans les solutions acides (pH < 8,2) et devient rose vif dans les milieux basiques (pH > 10,0).", "fr"),
    ]
}

/// Swahili curriculum (10 snippets). v0.2: hand-translated, scientific terms
/// preserved. Native-speaker review pass recommended before shipping.
pub fn get_curriculum_texts_sw() -> Vec<(&'static str, &'static str)> {
    vec![
        // Siku ya Pendulum
        ("Kipindi T cha pendulum rahisi kinapewa na T = 2 * pi * sqrt(L / g), ambapo L ni urefu wa kamba ya pendulum na g ni kasi ya ukuzaji wa graviti. Masi ya bob na pembe ya uhamisho (kwa pembe ndogo) haiongezeki kipindi.", "sw"),
        ("Kupungua kwa pendulum kunaakisi hasara ya nishati inayosababishwa na upinzani wa hewa au msuguano. Mwelekeo wa oscillation hupungua kwa kiasi cha kufuata: E(t) = E_0 * e^(-b*t/m). Masi kubwa ya bob ina hatari kubwa ya mwendo na hupungua polepole.", "sw"),

        // Lenzi za Spherical
        ("Mlinganisho wa lenzi nyembamba unahusisha urefu wa focal f, umbali wa kitu d_o, na umbali wa picha d_i kupitia 1/f = 1/d_o + 1/d_i. Lenzi convex (inayokusanya) ina urefu wa focal chanya f > 0, huku lenzi concave (inayotawanya) ina urefu wa focal hasi f < 0.", "sw"),
        ("Ukuzaji m wa lenzi unafafanuliwa kama m = -d_i / d_o = h_i / h_o. Ukuzaji chanya m > 0 unaashiria picha ya kawaida na iliyoonyeshwa wima, huku ukuzaji hasi m < 0 unaashiria picha halisi na iliyoonyeshwa kwa kichwa chini.", "sw"),

        // Solenoid
        ("Uwanja wa sumaku B ndani ya solenoid ndefu unapewa na B = mu_0 * mu_r * N * I / L, ambapo mu_0 ni permeability ya nafasi huru, mu_r ni permeability ya jamaa ya nyenzo ya kiini, N ni idadi ya zamu, I ni sasa, na L ni urefu wa solenoid.", "sw"),
        ("Kuingiza kiini cha ferromagnetic (kama vile kiini cha chuma laini chenye permeability ya juu ya jamaa mu_r) ndani ya solenoid huongeza kwa kiasi kikubwa msongezo wa sumaku B, kwa sababu maeneo ya kiini hulandana na uwanja, huku ukiongeza nguvu ya electro-magnet.", "sw"),

        // Sayansi ya joto
        ("Sheria ya gesi bora ni PV = N * k * T (au PV = n * R * T), ambapo P ni shinikizo, V ni kiasi, N ni idadi ya chembechembe za gesi, T ni joto halisi kwa Kelvin, na k ni constant ya Boltzmann (1.380649e-23 J/K).", "sw"),
        ("Nadharia ya kinetic ya gesi inasema kwamba shinikizo linatokana na mgongano wa molekuli za gesi na kuta za chombo. Joto la juu huongeza kasi ya chembechembe (v_rms inalingana na sqrt(T)) na kiwango cha mgongano na uhamisho wa hatari, huku ukiongeza shinikizo.", "sw"),

        // Kemia
        ("Katika titration ya asidi imara - besi imara (k.m., HCl na NaOH), nukta sawa hutokea wakati mole za asidi zinalingana na mole za besi, huku kukipa pH ya kawaida ya hasa 7.0 kwa 25 degrees Celsius.", "sw"),
        ("Alama za kemikali ni asidi au besi dhaifu zinazobadilisha rangi katika masafa maalum ya pH. Kwa mfano, Phenolphthalein haina rangi katika suluhisho za asidi (pH < 8.2) na hubadilika kuwa pink nyeusi katika mazingira ya besi (pH > 10.0).", "sw"),
    ]
}

/// v0.2: returns the union of EN, FR, and SW snippets. Embeddings are
/// computed once at startup (OnceLock), so the 3x cost is paid at boot.
pub fn get_curriculum_texts() -> Vec<(&'static str, &'static str)> {
    let mut all = get_curriculum_texts_en();
    all.extend(get_curriculum_texts_fr());
    all.extend(get_curriculum_texts_sw());
    all
}

pub fn get_embedding(model: &LlamaModel, backend: &LlamaBackend, text: &str) -> Result<Vec<f32>, String {
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
    let tokens = &tokens[..tokens_len];

    let mut batch = LlamaBatch::new(tokens_len, 1);
    for (i, token) in tokens.iter().enumerate() {
        batch.add(*token, i as i32, &[0], false)
            .map_err(|e| format!("Embedding batch add failed: {:?}", e))?;
    }

    ctx.clear_kv_cache();
    ctx.decode(&mut batch)
        .map_err(|e| format!("Embedding decode failed: {:?}", e))?;

    let embedding = ctx.embeddings_seq(0)
        .ok_or_else(|| "Embedding retrieval returned None".to_string())?
        .to_vec();

    Ok(embedding)
}

pub fn cosine_similarity(a: &Vec<f32>, b: &Vec<f32>) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// v0.2: now accepts a `lang` filter. Only snippets with `snippet.lang == lang`
/// (case-insensitive) are scored. If no snippet matches the requested
/// language, falls back to English so the model still has grounding.
pub fn retrieve_top_k(
    query_emb: &Vec<f32>,
    curriculum: &[CurriculumSnippet],
    k: usize,
    lang: &str,
) -> Vec<CurriculumSnippet> {
    let lang_lower = lang.to_lowercase();
    let mut filtered: Vec<&CurriculumSnippet> = curriculum
        .iter()
        .filter(|s| s.lang.eq_ignore_ascii_case(&lang_lower))
        .collect();

    // Fallback to English if the requested language has no snippets
    if filtered.is_empty() {
        eprintln!("Warning: no snippets found for lang='{}', falling back to English", lang);
        filtered = curriculum
            .iter()
            .filter(|s| s.lang == "en")
            .collect();
    }

    let mut scored: Vec<(&CurriculumSnippet, f32)> = filtered
        .iter()
        .map(|s| (s, cosine_similarity(query_emb, &s.embedding)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.iter().take(k).map(|(s, _)| (*s).clone()).collect()
}

pub fn get_curriculum(model: &LlamaModel, backend: &LlamaBackend) -> &'static Vec<CurriculumSnippet> {
    CURRICULUM.get_or_init(|| {
        let texts = get_curriculum_texts();
        let mut snippets = Vec::with_capacity(texts.len());
        for (text, lang) in texts {
            let embedding = get_embedding(model, backend, text)
                .expect("Failed to embed curriculum snippet");
            snippets.push(CurriculumSnippet {
                text: text.to_string(),
                embedding,
                lang,
            });
        }
        eprintln!("Info: Embedded {} curriculum snippets (EN+FR+SW)", snippets.len());
        snippets
    })
}
