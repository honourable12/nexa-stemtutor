use serde_json::{json, Value};

pub const TOOL_DEFINITIONS: &str = r#"[
  {"name": "solve_pendulum", "description": "Exact period of a pendulum",
   "parameters": {"L": "number (m)", "g": "number (m/s^2)"}},
  {"name": "solve_thin_lens", "description": "Image distance and magnification",
   "parameters": {"d_o": "number (object distance, m)", "f": "number (focal length, m)"}},
  {"name": "solve_ideal_gas", "description": "Solve PV=NkT for missing variable",
   "parameters": {"N": "number|null", "V": "number|null", "T": "number|null", "P": "number|null"}},
  {"name": "solve_titration_ph", "description": "Calculates pH curve at given titrant volume",
   "parameters": {"conc_acid": "number", "vol_acid": "number", "conc_base": "number", "vol_base": "number"}}
]"#;

pub fn execute_tool(name: &str, args: &Value) -> Value {
    match name {
        "solve_pendulum" => {
            let l = args["L"].as_f64().unwrap_or(1.0);
            let g = args["g"].as_f64().unwrap_or(9.81);
            let period = 2.0 * std::f64::consts::PI * (l / g).sqrt();
            json!({ "period_s": period })
        }
        "solve_thin_lens" => {
            let d_o = args["d_o"].as_f64().unwrap();
            let f = args["f"].as_f64().unwrap();
            let d_i = 1.0 / (1.0 / f - 1.0 / d_o);
            json!({ "d_i": d_i, "magnification": -d_i / d_o })
        }
        "solve_ideal_gas" => {
            let p = args.get("P").and_then(|v| v.as_f64());
            let v = args.get("V").and_then(|v| v.as_f64());
            let n = args.get("N").and_then(|v| v.as_f64());
            let t = args.get("T").and_then(|v| v.as_f64());

            // P * V = N * k * T
            // Use Boltzmann constant: k_B = 1.380649e-23
            let k = 1.380649e-23;

            if p.is_none() || p == Some(0.0) {
                if let (Some(v_val), Some(n_val), Some(t_val)) = (v, n, t) {
                    if v_val == 0.0 {
                        json!({ "error": "division_by_zero" })
                    } else {
                        json!({ "P": n_val * k * t_val / v_val })
                    }
                } else {
                    json!({ "error": "missing_parameters" })
                }
            } else if v.is_none() || v == Some(0.0) {
                if let (Some(p_val), Some(n_val), Some(t_val)) = (p, n, t) {
                    if p_val == 0.0 {
                        json!({ "error": "division_by_zero" })
                    } else {
                        json!({ "V": n_val * k * t_val / p_val })
                    }
                } else {
                    json!({ "error": "missing_parameters" })
                }
            } else if n.is_none() || n == Some(0.0) {
                if let (Some(p_val), Some(v_val), Some(t_val)) = (p, v, t) {
                    if t_val == 0.0 {
                        json!({ "error": "division_by_zero" })
                    } else {
                        json!({ "N": (p_val * v_val) / (k * t_val) })
                    }
                } else {
                    json!({ "error": "missing_parameters" })
                }
            } else if t.is_none() || t == Some(0.0) {
                if let (Some(p_val), Some(v_val), Some(n_val)) = (p, v, n) {
                    if n_val == 0.0 {
                        json!({ "error": "division_by_zero" })
                    } else {
                        json!({ "T": (p_val * v_val) / (n_val * k) })
                    }
                } else {
                    json!({ "error": "missing_parameters" })
                }
            } else {
                json!({ "error": "all_parameters_present" })
            }
        }
        "solve_titration_ph" => {
            let c_a = args["conc_acid"].as_f64().unwrap();
            let v_a = args["vol_acid"].as_f64().unwrap();
            let c_b = args["conc_base"].as_f64().unwrap();
            let v_b = args["vol_base"].as_f64().unwrap();
            
            let moles_a = c_a * (v_a / 1000.0);
            let moles_b = c_b * (v_b / 1000.0);
            let total_v = (v_a + v_b) / 1000.0;

            let ph = if moles_b > moles_a {
                14.0 - (-Math_log10((moles_b - moles_a) / total_v))
            } else if (moles_a - moles_b).abs() < 1e-7 {
                7.0
            } else {
                -Math_log10((moles_a - moles_b) / total_v)
            };
            json!({ "ph": ph })
        }
        _ => json!({ "error": "unknown_tool" }),
    }
}

fn Math_log10(val: f64) -> f64 {
    val.log10()
}
