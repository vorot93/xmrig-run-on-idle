# xmrig-run-on-idle
A Rust utility that monitors GNOME desktop idle time, automatically pauses [XMRig](https://xmrig.com) mining when the user is active and resumes mining when the system becomes idle beyond a configurable threshold.

## Installation
```bash
cd xmrig-run-on-idle
cargo build --release
```

The binary will be available at `target/release/xmrig-run-on-idle`.

## Configuration
### XMRig Setup
Ensure XMRig is running with JSON-RPC enabled. Add these options to your XMRig configuration:

```json
{
    "http": {
        "enabled": true,
        "host": "localhost",
        "port": 18080,
        "access-token": "YourSecureTokenHere",
        "restricted": false
    }
}
```

Or run XMRig with the following options:

`--http-port=18080 --http-access-token=YourSecureTokenHere --http-no-restricted`

## Usage
| Argument | Description | Required | Default |
|----------|-------------|----------|---------|
| `--url` | URL of XMRig's HTTP server | Yes | - |
| `--bearer` | Bearer token for authentication | Yes | - |
| `--threshold-ms` | Idle threshold in milliseconds | Yes | - |
| `--interval-ms` | Polling interval in milliseconds | No | 250 |

### Example
To pause mining when user has been idle for less than 5 minutes and check every second:

```bash
xmrig-run-on-idle \
    --url http://localhost:18080 \
    --bearer YourSecureTokenHere \
    --threshold-ms 300000 \
    --interval-ms 1000
```

Prepend `RUST_LOG=debug` environment variable to see polling details.

## systemd Service
To run as a systemd user service, create `~/.config/systemd/user/xmrig-idle.service`:

```ini
[Unit]
Description=XMRig Idle Controller
After=graphical-session.target

[Service]
Type=simple
ExecStart=/path/to/xmrig-run-on-idle --url http://127.0.0.1:18080 --bearer YOUR_TOKEN --threshold-ms 300000
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
```

Enable and start:
```bash
systemctl --user daemon-reload
systemctl --user enable xmrig-idle.service
systemctl --user start xmrig-idle.service
```

## License
This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-only). See the [LICENSE](LICENSE) file for details.
