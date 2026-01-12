# Rust Playground

An offline Rust code editor and runner with a web interface. Runs completely offline using Docker.

## Features

- Write and execute Rust code in your browser
- Syntax highlighting and tab support
- Real-time compilation and execution
- Error messages with line numbers
- Runs completely offline (no internet required after build)
- Minimal Alpine Linux base image

## Quick Start

### Using Docker

```bash
# Build the image
docker build -t rust-playground .

# Run the container
docker run -p 8080:8080 rust-playground
```

Then open http://localhost:8080 in your browser.

### Using Docker Compose

```bash
docker-compose up --build
```

### Using VS Code Dev Container

1. Open this folder in VS Code
2. Install the "Dev Containers" extension
3. Press F1 and select "Dev Containers: Reopen in Container"
4. The playground will be available at http://localhost:8080

## Project Structure

```
.
├── .devcontainer/
│   └── devcontainer.json    # VS Code dev container config
├── frontend/
│   ├── index.html           # Web interface
│   ├── style.css            # Styling
│   └── script.js            # Client-side logic
├── server/
│   ├── Cargo.toml           # Rust dependencies
│   └── src/
│       └── main.rs          # Server code
├── Dockerfile               # Container definition
├── docker-compose.yml       # Docker Compose config
└── README.md
```

## Keyboard Shortcuts

- **Ctrl/Cmd + Enter**: Run code
- **Tab**: Insert 4 spaces

## API

### POST /api/run

Execute Rust code.

**Request:**
```json
{
  "code": "fn main() { println!(\"Hello!\"); }"
}
```

**Response:**
```json
{
  "success": true,
  "output": "Hello!\n",
  "error": ""
}
```
