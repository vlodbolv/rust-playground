use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Instant;
use tokio::process::Command;
use uuid::Uuid;

struct AppState {
    crates: Mutex<Vec<CrateInfo>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct CrateInfo {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_features: Option<bool>,
}

#[derive(Deserialize)]
struct RunRequest {
    code: String,
    crates: Option<Vec<String>>,
}

#[derive(Serialize, Clone)]
struct FunctionTime {
    name: String,
    time_ms: f64,
}

#[derive(Serialize)]
struct RunResponse {
    success: bool,
    output: String,
    error: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    images: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compile_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    function_times: Vec<FunctionTime>,
}

fn collect_images(dir: &str) -> Vec<String> {
    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" || ext_str == "gif" || ext_str == "svg" {
                    if let Ok(data) = std::fs::read(&path) {
                        use base64::{Engine as _, engine::general_purpose::STANDARD};
                        let mime = match ext_str.as_str() {
                            "png" => "image/png",
                            "jpg" | "jpeg" => "image/jpeg",
                            "gif" => "image/gif",
                            "svg" => "image/svg+xml",
                            _ => "application/octet-stream",
                        };
                        let b64 = STANDARD.encode(&data);
                        images.push(format!("data:{};base64,{}", mime, b64));
                    }
                }
            }
        }
    }
    images
}

fn instrument_code_for_profiling(code: &str) -> String {
    let fn_regex = Regex::new(r"(?m)^(\s*)(pub\s+)?(async\s+)?fn\s+(\w+)\s*\(([^)]*)\)(\s*->\s*[^{]+)?\s*\{").unwrap();
    
    let mut result = String::new();
    let mut last_end = 0;
    let mut functions_found: Vec<(String, usize, usize)> = Vec::new();
    
    // First pass: find all function locations
    for cap in fn_regex.captures_iter(code) {
        let full_match = cap.get(0).unwrap();
        let fn_name = cap.get(4).unwrap().as_str().to_string();
        let start = full_match.start();
        let end = full_match.end();
        functions_found.push((fn_name, start, end));
    }
    
    if functions_found.is_empty() {
        return code.to_string();
    }
    
    // Add profiler import at the top
    result.push_str("use std::time::Instant as __ProfilerInstant;\n\n");
    
    // Process each function
    for (fn_name, fn_start, brace_end) in &functions_found {
        // Add code before this function
        result.push_str(&code[last_end..*fn_start]);
        
        // Add the function signature including the opening brace
        result.push_str(&code[*fn_start..*brace_end]);
        
        // Find the matching closing brace
        if let Some(close_idx) = find_matching_brace(&code[*brace_end..]) {
            let body_content = &code[*brace_end..*brace_end + close_idx];
            
            // Get indentation from the function
            let indent = if fn_name == "main" { "    " } else { "    " };
            
            // Add timing start
            result.push_str(&format!(
                "\n{}let __profiler_start = __ProfilerInstant::now();",
                indent
            ));
            
            // Check if the function has a return type or returns something
            let has_return_value = code[*fn_start..*brace_end].contains("->");
            
            // Wrap the body
            if has_return_value {
                // For functions with return values, wrap the result
                result.push_str(&format!("\n{}let __profiler_result = {{", indent));
                result.push_str(body_content);
                result.push_str(&format!("\n{}}};\n", indent));
                result.push_str(&format!(
                    "{}eprintln!(\"__PROFILER__:{}:{{:.6}}\", __profiler_start.elapsed().as_secs_f64() * 1000.0);\n",
                    indent, fn_name
                ));
                result.push_str(&format!("{}__profiler_result\n", indent));
            } else {
                // For void functions, just add timing at the end
                result.push_str(body_content);
                result.push_str(&format!(
                    "\n{}eprintln!(\"__PROFILER__:{}:{{:.6}}\", __profiler_start.elapsed().as_secs_f64() * 1000.0);",
                    indent, fn_name
                ));
            }
            
            result.push_str("\n}");
            last_end = *brace_end + close_idx + 1;
        } else {
            // Could not find matching brace, skip instrumentation for this function
            last_end = *brace_end;
        }
    }
    
    // Add remaining code
    result.push_str(&code[last_end..]);
    result
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 1; // We start after the opening brace
    let mut in_string = false;
    let mut in_char = false;
    let mut escape_next = false;
    let chars: Vec<char> = s.chars().collect();
    
    for (i, &c) in chars.iter().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        if c == '\\' && (in_string || in_char) {
            escape_next = true;
            continue;
        }
        
        if c == '"' && !in_char {
            in_string = !in_string;
            continue;
        }
        
        if c == '\'' && !in_string {
            // Check if it's a char literal or a lifetime
            if i + 2 < chars.len() && chars[i + 2] == '\'' {
                in_char = !in_char;
            }
            continue;
        }
        
        if !in_string && !in_char {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn parse_profiler_output(stderr: &str) -> (String, Vec<FunctionTime>) {
    let mut function_times: Vec<FunctionTime> = Vec::new();
    let mut clean_stderr = String::new();
    
    for line in stderr.lines() {
        if line.starts_with("__PROFILER__:") {
            let parts: Vec<&str> = line.trim_start_matches("__PROFILER__:").split(':').collect();
            if parts.len() == 2 {
                if let Ok(time) = parts[1].parse::<f64>() {
                    let name = parts[0].to_string();
                    if let Some(existing) = function_times.iter_mut().find(|ft| ft.name == name) {
                        existing.time_ms += time;
                    } else {
                        function_times.push(FunctionTime { name, time_ms: time });
                    }
                }
            }
        } else {
            if !clean_stderr.is_empty() {
                clean_stderr.push('\n');
            }
            clean_stderr.push_str(line);
        }
    }
    
    (clean_stderr, function_times)
}

#[derive(Serialize)]
struct StatusResponse {
    online: bool,
}

#[derive(Serialize)]
struct CratesResponse {
    crates: Vec<CrateInfo>,
}

#[derive(Deserialize)]
struct AddCrateRequest {
    name: String,
    version: Option<String>,
    features: Option<Vec<String>>,
    default_features: Option<bool>,
}

#[derive(Deserialize)]
struct RemoveCrateRequest {
    name: String,
}

async fn check_status() -> HttpResponse {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};
    
    // Try to connect to crates.io via TCP (port 443 for HTTPS)
    let online = timeout(
        Duration::from_secs(3),
        TcpStream::connect("crates.io:443")
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);

    HttpResponse::Ok().json(StatusResponse { online })
}

fn get_standard_crates() -> Vec<CrateInfo> {
    vec![
        CrateInfo { name: "std".to_string(), version: "builtin".to_string(), features: None, default_features: None },
        CrateInfo { name: "core".to_string(), version: "builtin".to_string(), features: None, default_features: None },
        CrateInfo { name: "alloc".to_string(), version: "builtin".to_string(), features: None, default_features: None },
        CrateInfo { name: "collections".to_string(), version: "builtin".to_string(), features: None, default_features: None },
        CrateInfo { name: "proc_macro".to_string(), version: "builtin".to_string(), features: None, default_features: None },
    ]
}

async fn list_crates(data: web::Data<AppState>) -> HttpResponse {
    let user_crates = data.crates.lock().unwrap().clone();
    let mut all_crates = get_standard_crates();
    all_crates.extend(user_crates);
    HttpResponse::Ok().json(CratesResponse { crates: all_crates })
}

async fn add_crate(req: web::Json<AddCrateRequest>, data: web::Data<AppState>) -> HttpResponse {
    let online = Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "--connect-timeout", "3", "https://crates.io/api/v1/crates?per_page=1"])
        .stdout(Stdio::piped())
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "200")
        .unwrap_or(false);

    if !online {
        return HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "success": false,
            "error": "Cannot add crates while offline"
        }));
    }

    let version = req.version.clone().unwrap_or_else(|| "*".to_string());
    
    let url = format!("https://crates.io/api/v1/crates/{}", req.name);
    let check = Command::new("curl")
        .args(["-s", &url])
        .stdout(Stdio::piped())
        .output()
        .await;

    match check {
        Ok(output) => {
            let response = String::from_utf8_lossy(&output.stdout);
            if response.contains("\"errors\"") {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": format!("Crate '{}' not found on crates.io", req.name)
                }));
            }

            let actual_version = if version == "*" {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                    parsed["crate"]["max_stable_version"]
                        .as_str()
                        .unwrap_or("latest")
                        .to_string()
                } else {
                    "latest".to_string()
                }
            } else {
                version
            };

            let mut crates = data.crates.lock().unwrap();
            if !crates.iter().any(|c| c.name == req.name) {
                crates.push(CrateInfo {
                    name: req.name.clone(),
                    version: actual_version.clone(),
                    features: req.features.clone(),
                    default_features: req.default_features,
                });
            }

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "name": req.name,
                "version": actual_version,
                "features": req.features,
                "default_features": req.default_features
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to verify crate: {}", e)
        })),
    }
}

async fn remove_crate(req: web::Json<RemoveCrateRequest>, data: web::Data<AppState>) -> HttpResponse {
    let standard_crates = get_standard_crates();
    let is_builtin = standard_crates.iter().any(|c| c.name == req.name);
    if is_builtin {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": format!("Cannot remove built-in crate '{}'", req.name)
        }));
    }

    let mut crates = data.crates.lock().unwrap();
    let initial_len = crates.len();
    crates.retain(|c| c.name != req.name);
    
    if crates.len() < initial_len {
        HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": format!("Removed crate '{}'", req.name)
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": format!("Crate '{}' not found", req.name)
        }))
    }
}

#[derive(Deserialize)]
struct FormatRequest {
    code: String,
}

#[derive(Serialize)]
struct FormatResponse {
    success: bool,
    formatted: String,
    error: String,
}

async fn format_code(req: web::Json<FormatRequest>) -> HttpResponse {
    let id = Uuid::new_v4().to_string();
    let temp_file = format!("/tmp/rustfmt-{}.rs", id);

    if let Err(e) = std::fs::write(&temp_file, &req.code) {
        return HttpResponse::InternalServerError().json(FormatResponse {
            success: false,
            formatted: String::new(),
            error: format!("Failed to write temp file: {}", e),
        });
    }

    let result = Command::new("rustfmt")
        .arg(&temp_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match result {
        Ok(output) => {
            if output.status.success() {
                match std::fs::read_to_string(&temp_file) {
                    Ok(formatted) => {
                        let _ = std::fs::remove_file(&temp_file);
                        HttpResponse::Ok().json(FormatResponse {
                            success: true,
                            formatted,
                            error: String::new(),
                        })
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&temp_file);
                        HttpResponse::InternalServerError().json(FormatResponse {
                            success: false,
                            formatted: String::new(),
                            error: format!("Failed to read formatted file: {}", e),
                        })
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let _ = std::fs::remove_file(&temp_file);
                HttpResponse::Ok().json(FormatResponse {
                    success: false,
                    formatted: String::new(),
                    error: stderr,
                })
            }
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temp_file);
            HttpResponse::InternalServerError().json(FormatResponse {
                success: false,
                formatted: String::new(),
                error: format!("rustfmt failed: {}", e),
            })
        }
    }
}

async fn run_code(req: web::Json<RunRequest>, data: web::Data<AppState>) -> HttpResponse {
    let total_start = Instant::now();
    let id = Uuid::new_v4().to_string();
    let temp_dir = format!("/tmp/rust-playground-{}", id);
    let src_dir = format!("{}/src", temp_dir);

    if let Err(e) = std::fs::create_dir_all(&src_dir) {
        return HttpResponse::InternalServerError().json(RunResponse {
            success: false,
            output: String::new(),
            error: format!("Failed to create temp directory: {}", e),
            images: Vec::new(),
            compile_time_ms: None,
            run_time_ms: None,
            total_time_ms: None,
            function_times: Vec::new(),
        });
    }

    let user_crates = data.crates.lock().unwrap().clone();
    let standard_crates = get_standard_crates();
    let non_builtin_standard: Vec<CrateInfo> = standard_crates
        .into_iter()
        .filter(|c| c.version != "builtin")
        .collect();
    
    let mut all_available: Vec<CrateInfo> = non_builtin_standard;
    all_available.extend(user_crates);
    
    let used_crates: Vec<CrateInfo> = if let Some(ref requested) = req.crates {
        all_available.into_iter().filter(|c| requested.contains(&c.name)).collect()
    } else {
        vec![]
    };

    if !used_crates.is_empty() {
        // Sort crates to ensure consistent Cargo.toml for same dependencies
        let mut sorted_crates = used_crates.clone();
        sorted_crates.sort_by(|a, b| a.name.cmp(&b.name));
        
        let mut cargo_toml = String::from("[package]\nname = \"playground\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[profile.release]\nincremental = true\n\n[dependencies]\n");
        for c in &sorted_crates {
            let has_features = c.features.as_ref().map(|f| !f.is_empty()).unwrap_or(false);
            let has_default_features = c.default_features.is_some();
            
            if has_features || has_default_features {
                cargo_toml.push_str(&format!("{} = {{ version = \"{}\"", c.name, c.version));
                if let Some(false) = c.default_features {
                    cargo_toml.push_str(", default-features = false");
                }
                if let Some(ref features) = c.features {
                    if !features.is_empty() {
                        let features_str: Vec<String> = features.iter().map(|f| format!("\"{}\"", f)).collect();
                        cargo_toml.push_str(&format!(", features = [{}]", features_str.join(", ")));
                    }
                }
                cargo_toml.push_str(" }\n");
            } else {
                cargo_toml.push_str(&format!("{} = \"{}\"\n", c.name, c.version));
            }
        }
        
        if let Err(e) = std::fs::write(format!("{}/Cargo.toml", temp_dir), cargo_toml) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to write Cargo.toml: {}", e),
                images: Vec::new(),
                compile_time_ms: None,
                run_time_ms: None,
                total_time_ms: Some(total_start.elapsed().as_millis() as u64),
                function_times: Vec::new(),
            });
        }

        // Instrument the code for automatic profiling
        let instrumented_code = instrument_code_for_profiling(&req.code);
        
        if let Err(e) = std::fs::write(format!("{}/main.rs", src_dir), &instrumented_code) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to write code: {}", e),
                images: Vec::new(),
                compile_time_ms: None,
                run_time_ms: None,
                total_time_ms: Some(total_start.elapsed().as_millis() as u64),
                function_times: Vec::new(),
            });
        }

        let compile_start = Instant::now();
        let build_result = Command::new("cargo")
            .args(["run", "--release"])
            .current_dir(&temp_dir)
            .env("CARGO_TARGET_DIR", "/tmp/rust-playground-cache")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let compile_time = compile_start.elapsed().as_millis() as u64;

        let images = collect_images(&temp_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);
        let total_time = total_start.elapsed().as_millis() as u64;

        match build_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let (clean_stderr, function_times) = parse_profiler_output(&stderr);
                if output.status.success() {
                    HttpResponse::Ok().json(RunResponse {
                        success: true,
                        output: stdout,
                        error: String::new(),
                        images,
                        compile_time_ms: Some(compile_time),
                        run_time_ms: None,
                        total_time_ms: Some(total_time),
                        function_times,
                    })
                } else {
                    HttpResponse::Ok().json(RunResponse {
                        success: false,
                        output: stdout,
                        error: clean_stderr,
                        images: Vec::new(),
                        compile_time_ms: Some(compile_time),
                        run_time_ms: None,
                        total_time_ms: Some(total_time),
                        function_times: Vec::new(),
                    })
                }
            }
            Err(e) => HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Build failed: {}", e),
                images: Vec::new(),
                compile_time_ms: None,
                run_time_ms: None,
                total_time_ms: Some(total_time),
                function_times: Vec::new(),
            }),
        }
    } else {
        let main_file = format!("{}/main.rs", temp_dir);
        let output_file = format!("{}/main", temp_dir);

        // Instrument the code for automatic profiling
        let instrumented_code = instrument_code_for_profiling(&req.code);
        
        if let Err(e) = std::fs::write(&main_file, &instrumented_code) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to write code: {}", e),
                images: Vec::new(),
                compile_time_ms: None,
                run_time_ms: None,
                total_time_ms: Some(total_start.elapsed().as_millis() as u64),
                function_times: Vec::new(),
            });
        }

        let compile_start = Instant::now();
        let compile_result = Command::new("rustc")
            .args([&main_file, "-o", &output_file])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let compile_time = compile_start.elapsed().as_millis() as u64;

        match compile_result {
            Ok(output) => {
                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr).to_string();
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return HttpResponse::Ok().json(RunResponse {
                        success: false,
                        output: String::new(),
                        error,
                        images: Vec::new(),
                        compile_time_ms: Some(compile_time),
                        run_time_ms: None,
                        total_time_ms: Some(total_start.elapsed().as_millis() as u64),
                        function_times: Vec::new(),
                    });
                }
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return HttpResponse::InternalServerError().json(RunResponse {
                    success: false,
                    output: String::new(),
                    error: format!("Compilation failed: {}", e),
                    images: Vec::new(),
                    compile_time_ms: None,
                    run_time_ms: None,
                    total_time_ms: Some(total_start.elapsed().as_millis() as u64),
                    function_times: Vec::new(),
                });
            }
        }

        let run_start = Instant::now();
        let run_result = Command::new(&output_file)
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let run_time = run_start.elapsed().as_millis() as u64;

        let images = collect_images(&temp_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);
        let total_time = total_start.elapsed().as_millis() as u64;

        match run_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let (clean_stderr, function_times) = parse_profiler_output(&stderr);
                HttpResponse::Ok().json(RunResponse {
                    success: output.status.success(),
                    output: stdout,
                    error: clean_stderr,
                    images,
                    compile_time_ms: Some(compile_time),
                    run_time_ms: Some(run_time),
                    total_time_ms: Some(total_time),
                    function_times,
                })
            }
            Err(e) => HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Execution failed: {}", e),
                images: Vec::new(),
                compile_time_ms: Some(compile_time),
                run_time_ms: None,
                total_time_ms: Some(total_time),
                function_times: Vec::new(),
            }),
        }
    }
}

async fn prewarm_cache() {
    println!("Pre-warming cargo cache...");
    let cache_dir = "/tmp/rust-playground-cache";
    let warmup_dir = "/tmp/rust-playground-warmup";
    
    // Create warmup project with EXACT same format as run_code uses
    let _ = std::fs::create_dir_all(format!("{}/src", warmup_dir));
    
    let cargo_toml = r#"[package]
name = "playground"
version = "0.1.0"
edition = "2021"

[profile.release]
incremental = true
"#;
    
    let main_rs = r#"fn main() { println!("Hello"); }"#;
    
    let _ = std::fs::write(format!("{}/Cargo.toml", warmup_dir), cargo_toml);
    let _ = std::fs::write(format!("{}/src/main.rs", warmup_dir), main_rs);
    
    // Build to warm the cache
    let result = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(warmup_dir)
        .env("CARGO_TARGET_DIR", cache_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;
    
    match result {
        Ok(output) => {
            if output.status.success() {
                println!("Cache pre-warmed successfully!");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("Cache warmup build failed: {}", stderr);
            }
        }
        Err(e) => println!("Cache warmup error: {}", e),
    }
    
    // Don't remove warmup directory - keep Cargo.lock for faster subsequent builds
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "5000".to_string());
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "/app/frontend".to_string());
    
    let app_state = web::Data::new(AppState {
        crates: Mutex::new(Vec::new()),
    });

    // Pre-warm the cache in background
    tokio::spawn(async {
        prewarm_cache().await;
    });

    println!("Rust Playground starting on http://0.0.0.0:{}", port);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/api/run", web::post().to(run_code))
            .route("/api/format", web::post().to(format_code))
            .route("/api/status", web::get().to(check_status))
            .route("/api/crates", web::get().to(list_crates))
            .route("/api/crates/add", web::post().to(add_crate))
            .route("/api/crates/remove", web::post().to(remove_crate))
            .service(Files::new("/", &frontend_dir).index_file("index.html"))
    })
    .bind(format!("0.0.0.0:{}", port))?;
    
    println!("Server ready and accepting connections on port {}", port);
    
    server.run().await
}
