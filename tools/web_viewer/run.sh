#!/bin/bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Activate the virtual environment
source "$PROJECT_ROOT/tools/.venv/bin/activate"

# Start the uvicorn server
echo "Starting Ratel Drone Telemetry Web Server..."
echo "Open your browser at: http://localhost:8080"
python -m uvicorn server:app --app-dir "$SCRIPT_DIR" --host 0.0.0.0 --port 8080
