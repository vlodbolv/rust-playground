use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Deserialize)]
struct RunRequest {
    code: String,
}

#[derive(Serialize)]
struct RunResponse {
    success: bool,
    output: String,
    error: String,
}

async fn run_code(req: web::Json<RunRequest>) -> HttpResponse {
    let id = Uuid::new_v4().to_string();
    let temp_dir = format!("/tmp/rust-playground-{}", id);
    let main_file = format!("{}/main.rs", temp_dir);
    let output_file = format!("{}/main", temp_dir);

    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        return HttpResponse::InternalServerError().json(RunResponse {
            success: false,
            output: String::new(),
            error: format!("Failed to create temp directory: {}", e),
        });
    }

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "/app/frontend".to_string());
    
    println!("Rust Playground starting on http://0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .route("/api/run", web::post().to(run_code))
            .service(Files::new("/", &frontend_dir).index_file("index.html"))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
