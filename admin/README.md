# Mycelix Admin Panel

A web-based admin panel for the Mycelix Living Protocol WebSocket server.

## Features

- **Dashboard**: Overview of server status, cycle state, and key metrics
- **Cycle Control**: View and control the metabolism cycle (test mode only)
- **Connections**: Monitor active WebSocket connections
- **Metrics**: Detailed phase metrics with charts
- **History**: Phase transition history
- **Settings**: View server configuration

## Development

### Prerequisites

- Node.js 18+
- npm 9+

### Setup

```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

The development server runs on `http://localhost:3000` and proxies API requests to the admin server at `http://localhost:8891`.

### Building

```bash
# Build for production
npm run build

# Or use the build script
./build.sh
```

The built files will be in the `dist/` directory.

## Starting the Server with Admin Panel

```bash
# Start the WebSocket server with admin panel enabled
cargo run -p ws-server -- --enable-admin

# With custom admin port
cargo run -p ws-server -- --enable-admin --admin-port 9000

# With authentication
cargo run -p ws-server -- --enable-admin --admin-password mysecretpassword

# In test mode (enables cycle advancement)
cargo run -p ws-server -- --enable-admin --simulated-time
```

## API Endpoints

The admin server provides the following REST API endpoints under `/admin/api/`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/admin/api/state` | GET | Current cycle state |
| `/admin/api/connections` | GET | Active connections list |
| `/admin/api/server/metrics` | GET | Server metrics |
| `/admin/api/metrics` | GET | Current phase metrics |
| `/admin/api/metrics/:phase` | GET | Metrics for specific phase |
| `/admin/api/history` | GET | Phase transition history |
| `/admin/api/config` | GET | Server configuration |
| `/admin/api/cycle/advance` | POST | Advance to next phase (test mode only) |

## Authentication

When `--admin-password` is set, the admin API requires HTTP Basic Authentication:

- Username: `admin`
- Password: The value passed to `--admin-password`

Example curl request:
```bash
curl -u admin:mypassword http://localhost:8891/admin/api/state
```

## Project Structure

```
admin/
├── src/
│   ├── api/           # API client
│   │   └── client.ts  # Admin API client with types
│   ├── components/    # Reusable UI components
│   │   ├── Card.tsx   # Card containers
│   │   ├── Table.tsx  # Data tables
│   │   ├── Chart.tsx  # Recharts wrappers
│   │   ├── Badge.tsx  # Status badges
│   │   └── Button.tsx # Styled buttons
│   ├── hooks/         # React hooks
│   │   └── useApi.ts  # Data fetching hooks
│   ├── pages/         # Page components
│   │   ├── Dashboard.tsx
│   │   ├── CycleControl.tsx
│   │   ├── Connections.tsx
│   │   ├── Metrics.tsx
│   │   ├── History.tsx
│   │   └── Settings.tsx
│   ├── App.tsx        # Main layout with routing
│   ├── main.tsx       # Entry point
│   └── index.css      # Tailwind imports
├── package.json
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
├── build.sh           # Build script
└── README.md
```

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool
- **TailwindCSS** - Styling
- **Recharts** - Charts
- **React Router** - Navigation

## Embedding in Rust Binary

For production deployments, the admin panel can be embedded in the Rust binary using [rust-embed](https://github.com/pyrossh/rust-embed):

1. Build the admin panel: `./build.sh`
2. Add rust-embed to Cargo.toml
3. Embed the `dist/` directory
4. Serve files from the embedded assets

Example rust-embed setup:

```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "admin/dist/"]
struct AdminAssets;

// Then serve files from AdminAssets::get("index.html")
```

## License

AGPL-3.0-or-later
