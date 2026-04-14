# port

A simple port and process manager.

## Usage

- `port 3000`: Show what's using port 3000 with process name and PID.
- `port kill 3000`: Kill the process using port 3000.
- `port list`: List all listening ports.
- `port free 3000-3010`: Show the first available port in the range.
- `port watch 8080`: Attempt to tail the process output on port 8080.

## Installation

```bash
./install.sh
```
