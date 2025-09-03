# Ambient Code Watcher

Real-time code review assistant based on [Codex](https://github.com/anthropics/codex)

> **Note**: See [ORIGINAL_README.md](ORIGINAL_README.md) for the original Codex README.
> **Japanese Version**: See [README.ja.md](README.ja.md)

## Overview

Ambient Watcher is a tool that automatically detects code changes and performs real-time code reviews using local LLM (Ollama). You can view review results and ask interactive questions through the Web UI.

## Features

- **Automatic Code Review** - Automatically reviews Git changes upon detection
- **Interactive Questions** - Ask specific questions directly from Web UI  
- **Flexible Configuration** - Customize review perspectives per project
- **Web UI** - User-friendly browser-based interface
- **Privacy First** - All processing done locally

## Installation

### Prerequisites

- Rust (1.70 or later)
- [Ollama](https://ollama.ai/) (for local LLM execution)
- Git

### Setup

```bash
# Clone repository
git clone https://github.com/hama-jp/ambient_code_watcher.git
cd ambient_code_watcher

# Automatic installation (recommended)
./install.sh

# Or manual build
cd codex-rs && cargo build --release

# Download Ollama model (recommended)
ollama pull llama3.2
```

### Install Script

`install.sh` automatically configures:

- Installs executable to `~/.local/bin/`
- Makes `ambient` command globally available  
- Creates default configuration files
- Provides PATH setup instructions

```bash
# Install
./install.sh
# Build takes a little time

# Uninstall  
./install.sh --uninstall
```

## Usage

### Basic Usage

```bash
# Initial setup in project
ambient init

# Start Ambient Watcher
ambient

# Also opens browser automatically
ambient --open
```

### Web UI

After starting, access Web UI at `http://localhost:38080`.

- Real-time review results display
- Formatted Markdown output

## Configuration

### Project Configuration (`.ambient/config.toml`)

```toml
[[reviews]]
name = "Custom Review"
description = "Project-specific review"
file_patterns = ["src/**/*.rs"]
prompt = """
Please review with the following perspectives:
1. Error handling
2. Performance
3. Security
"""
priority = 300
enabled = true
```

### Server Configuration

Server settings are also managed in `.ambient/config.toml`:

```toml
# File change detection interval (seconds)
check_interval_secs = 60

# Web UI port number
port = 38080

# List of file extensions to analyze
file_extensions = ["rs", "toml", "js", "ts", "py"]
```

## Project Structure

```
codex-rs/
|-- cli/src/
|   |-- ambient.rs              # Main logic
|   |-- ambient_server.rs        # WebSocket server
|   |-- ambient_config.rs        # Global configuration
|   |-- ambient_project_config.rs # Project configuration
|   +-- ambient_ui/              # Web UI files
|-- ambient                      # Launch script
+-- ambient-init                 # Initialization script
```

## Customization

### Adding Review Perspectives

Edit `.ambient/config.toml` to add custom review perspectives:

- `file_patterns`: Target file patterns
- `priority`: Execution priority (higher values prioritized)
- `prompt`: Review prompt

### Exclusion Patterns

Exclude specific files or directories:

```toml
exclude_patterns = [
    "target/**",
    "node_modules/**",
    "*.generated.rs"
]
```

## Stopping the System

### Stop Ambient Watcher

```bash
# Stop Ambient Watcher (Ctrl+C or)
pkill -f "ambient"
```

### Stop Ollama

```bash
# Stop Ollama service
systemctl stop ollama

# Or manually terminate process
pkill ollama
```

## Troubleshooting

### Port Already in Use
Ambient Watcher automatically tries next ports (38081, 38082...).

### Ollama Not Working

```bash
# Check Ollama status
ollama list

# Restart service
ollama serve
```

## Security and Privacy

- All processing done locally
- No code sent to external servers
- Uses DOMPurify for XSS protection
- Project settings saved in `.ambient/`

## License

This project is developed based on [Codex](https://github.com/anthropics/codex).
Follows the license terms of the original Codex project.

## Acknowledgments

- [Anthropic Codex](https://github.com/anthropics/codex) - Base of this project
- [Ollama](https://ollama.ai/) - Local LLM execution environment
- All contributors

## Contributing

Issue reports and Pull Requests are welcome. For major changes, please discuss in an Issue first.
