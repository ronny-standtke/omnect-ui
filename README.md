# omnect UI

Product page: <www.omnect.io>

This module implements a web frontend and backend to provide omnect specific features in a local environment, where the device might not be connected to the azure cloud. In that case the device cannot be remotely controlled by [omnect-portal](https://cp.omnect.conplement.cloud/) and omnect UI might be the alternative.

## Architecture

omnect UI follows a full-stack Single Page Application (SPA) architecture:

- **Backend**: Rust-based web service (Actix-web) providing API endpoints and WebSocket support via Centrifugo
- **Crux Core**: Platform-agnostic business logic compiled to WebAssembly
- **Frontend**: Vue 3 TypeScript SPA serving as the shell for the Crux Core
- **Shared Types**: TypeScript bindings auto-generated from Rust types

## Install omnect UI

Since omnect secure OS is designed as generic OS, all specific or optional applications must be provided as docker images via azure iotedge deployment:

- deployment of omnect UI docker image via omnect-portal to a device in field
- device must be online (at least once) in order to receive the deployment and to set initial password
- after a factory reset omnect UI must be deployed again what requires a connection to azure cloud

## Access omnect UI

omnect UI can be reached at <https://DeviceIp:1977>

Login with the configured password<br>
![login](docu/login.png)<br>
Watch device status<br>
![login](docu/main.png)<br>
Reset device and choose options to keep<br>
![factory-reset](docu/factory-reset.png)<br>
Update your device<br>
![update](docu/update.png)

## Development

### Prerequisites

- Rust toolchain (1.89+)
- Bun or pnpm for frontend development
- Docker with buildx support
- `toml` CLI tool (for version extraction)

### Project Structure

```
omnect-ui/
├── src/
│   ├── backend/          # Rust backend (Actix-web)
│   ├── app/              # Crux Core (business logic)
│   ├── shared_types/     # TypeGen for TypeScript bindings
│   └── ui/               # Vue 3 frontend
├── tools/                # Development tools
│   ├── centrifugo        # WebSocket server binary (gitignored)
│   └── setup-centrifugo.sh  # Download script for Centrifugo
├── Dockerfile            # Multi-stage Docker build
└── build-and-deploy-image.sh  # Build and deployment script
```

### Building

#### Local Development

```bash
# Setup Centrifugo (first time only)
./tools/setup-centrifugo.sh

# Backend
cargo build -p omnect-ui

# Frontend (from src/ui/)
export PATH="$HOME/.local/share/pnpm:$PATH"
pnpm install
pnpm run dev

# Generate TypeScript types
cargo build -p shared_types
```

#### Docker Image Build

Use the `build-and-deploy-image.sh` script for building and optionally deploying a Docker image to device **(there must be an existing deployment and the restart policy of omnect-ui must be set to `never`)**.

```bash
# Build ARM64 image (default)
./build-and-deploy-image.sh

# Build for different architecture
./build-and-deploy-image.sh --arch amd64

# Build with custom tag
./build-and-deploy-image.sh --tag v1.2.0

# Build and push to registry
./build-and-deploy-image.sh --push

# Build and deploy to device
./build-and-deploy-image.sh --deploy

# Full example with all options
./build-and-deploy-image.sh --arch arm64 --tag my-feature --push --deploy --host 192.168.1.100 --port 1977
```

Run `./build-and-deploy-image.sh --help` for all available options.

### Testing

```bash
# Run backend tests
cargo test --features mock

# Run Crux Core tests
cargo test -p omnect-ui-core

# Lint
cargo clippy --all-targets --features mock
```

### VSCode Integration

The project includes VSCode launch configurations with pre-launch tasks that:

- Kill any running Centrifugo processes
- Set up test password file for local development
- a running local instance of omnect-device-service is required

## License

Licensed under either of

- Apache License, Version 2.0, (./LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (./LICENSE-MIT or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

---

copyright (c) 2024 conplement AG<br>
Content published under the Apache License Version 2.0 or MIT license, are marked as such. They may be used in accordance with the stated license conditions.
