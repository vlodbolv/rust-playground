use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Mutex;
use tokio::process::Command;
use uuid::Uuid;

struct AppState {
    crates: Mutex<Vec<CrateInfo>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct CrateInfo {
    name: String,
    version: String,
}

#[derive(Deserialize)]
struct RunRequest {
    code: String,
    crates: Option<Vec<String>>,
}

#[derive(Serialize)]
struct RunResponse {
    success: bool,
    output: String,
    error: String,
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
}

#[derive(Deserialize)]
struct RemoveCrateRequest {
    name: String,
}

async fn check_status() -> HttpResponse {
    let online = Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "--connect-timeout", "3", "https://crates.io/api/v1/crates?per_page=1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "200")
        .unwrap_or(false);

    HttpResponse::Ok().json(StatusResponse { online })
}

async fn list_crates(data: web::Data<AppState>) -> HttpResponse {
    let crates = data.crates.lock().unwrap().clone();
    HttpResponse::Ok().json(CratesResponse { crates })
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
                });
            }

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "name": req.name,
                "version": actual_version
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to verify crate: {}", e)
        })),
    }
}

async fn remove_crate(req: web::Json<RemoveCrateRequest>, data: web::Data<AppState>) -> HttpResponse {
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
    let id = Uuid::new_v4().to_string();
    let temp_dir = format!("/tmp/rust-playground-{}", id);
    let src_dir = format!("{}/src", temp_dir);

    if let Err(e) = std::fs::create_dir_all(&src_dir) {
        return HttpResponse::InternalServerError().json(RunResponse {
            success: false,
            output: String::new(),
            error: format!("Failed to create temp directory: {}", e),
        });
    }

    let crates = data.crates.lock().unwrap().clone();
    let used_crates: Vec<&CrateInfo> = if let Some(ref requested) = req.crates {
        crates.iter().filter(|c| requested.contains(&c.name)).collect()
    } else {
        vec![]
    };

    if !used_crates.is_empty() {
        let mut cargo_toml = String::from("[package]\nname = \"playground\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n");
        for c in &used_crates {
            cargo_toml.push_str(&format!("{} = \"{}\"\n", c.name, c.version));
        }
        
        if let Err(e) = std::fs::write(format!("{}/Cargo.toml", temp_dir), cargo_toml) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to write Cargo.toml: {}", e),
            });
        }

        if let Err(e) = std::fs::write(format!("{}/main.rs", src_dir), &req.code) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to write code: {}", e),
            });
        }

        let build_result = Command::new("cargo")
            .args(["run", "--release"])
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let _ = std::fs::remove_dir_all(&temp_dir);

        match build_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if output.status.success() {
                    HttpResponse::Ok().json(RunResponse {
                        success: true,
                        output: stdout,
                        error: String::new(),
                    })
                } else {
                    HttpResponse::Ok().json(RunResponse {
                        success: false,
                        output: stdout,
                        error: stderr,
                    })
                }
            }
            Err(e) => HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Build failed: {}", e),
            }),
        }
    } else {
        let main_file = format!("{}/main.rs", temp_dir);
        let output_file = format!("{}/main", temp_dir);

        if let Err(e) = std::fs::write(&main_file, &req.code) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to write code: {}", e),
            });
        }

        let compile_result = Command::new("rustc")
            .args([&main_file, "-o", &output_file])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match compile_result {
            Ok(output) => {
                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr).to_string();
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return HttpResponse::Ok().json(RunResponse {
                        success: false,
                        output: String::new(),
                        error,
                    });
                }
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return HttpResponse::InternalServerError().json(RunResponse {
                    success: false,
                    output: String::new(),
                    error: format!("Compilation failed: {}", e),
                });
            }
        }

        let run_result = Command::new(&output_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let _ = std::fs::remove_dir_all(&temp_dir);

        match run_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                HttpResponse::Ok().json(RunResponse {
                    success: output.status.success(),
                    output: stdout,
                    error: stderr,
                })
            }
            Err(e) => HttpResponse::InternalServerError().json(RunResponse {
                success: false,
                output: String::new(),
                error: format!("Execution failed: {}", e),
            }),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "/app/frontend".to_string());
    
    let app_state = web::Data::new(AppState {
        crates: Mutex::new(Vec::new()),
    });

    println!("Rust Playground starting on http://0.0.0.0:{}", port);

    HttpServer::new(move || {
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
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
