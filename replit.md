# Rust Playground

## Overview
An offline Rust code editor and runner with a web interface. This project is designed to run in Docker containers on your local machine.

## Project Structure
```
.
├── .devcontainer/           # VS Code dev container configuration
│   └── devcontainer.json
├── frontend/                # Web interface files
│   ├── index.html
│   ├── style.css
│   └── script.js
├── server/                  # Rust backend server
│   ├── Cargo.toml
│   └── src/main.rs
├── Dockerfile               # Alpine Linux based container
├── docker-compose.yml       # Easy container orchestration
└── README.md
```

## How to Use (Local Machine)

### Docker
```bash
docker build -t rust-playground .
docker run -p 8080:8080 rust-playground
```

### Docker Compose
```bash
docker-compose up --build
```

### VS Code Dev Container
1. Open folder in VS Code
2. Install "Dev Containers" extension
3. Use "Reopen in Container" command
4. Access at http://localhost:8080

## Technical Details
- **Base Image**: Alpine Linux (minimal ~5MB)
- **Backend**: Actix-web (Rust)
- **Frontend**: Vanilla HTML/CSS/JS
- **Port**: 8080

## Recent Changes
- 2026-01-12: Initial project creation with Dockerfile, devcontainer config, and web interface
