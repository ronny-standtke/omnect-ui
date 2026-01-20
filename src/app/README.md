# Crux Core for omnect-ui

This package contains the Crux Core for the omnect-ui application. It implements the business logic and state management in Rust, which can be compiled to WebAssembly for use in the Vue frontend.

## Architecture

The Crux Core follows the Model-View-Update pattern:

- **Model** - The complete application state (auth, device info, network status, etc.)
- **ViewModel** - Data needed by the UI to render
- **Events** - Actions that can occur in the application
- **Effects** - Side effects (HTTP requests, WebSocket, rendering)

## Key Files

- `src/lib.rs` - App struct, Effect enum, and re-exports
- `src/model.rs` - Model and ViewModel structs
- `src/events.rs` - Event enum definitions
- `src/types/` - Domain-based type definitions
  - `auth.rs` - Authentication types (AuthToken, password requests)
  - `device.rs` - Device information types (SystemInfo, HealthcheckInfo)
  - `network.rs` - Network configuration types and state
  - `factory_reset.rs` - Factory reset types
  - `update.rs` - Update validation types
  - `common.rs` - Common shared types
- `src/http_helpers.rs` - HTTP response handling helper functions
- `src/macros.rs` - HTTP request macros (`auth_post!`, `unauth_post!`, `auth_post_basic!`, `http_get!`, `http_get_silent!`, `handle_response!`)
- `src/update/` - Domain-based event handlers
  - `mod.rs` - Main dispatcher
  - `auth.rs` - Authentication handlers (login, logout, password)
  - `device/` - Device action handlers
    - `mod.rs` - Device event dispatcher
    - `operations.rs` - Device operations (reboot, factory reset, updates)
    - `reconnection.rs` - Device reconnection handlers
    - `network/` - Network configuration handlers
      - `mod.rs` - Module re-exports
      - `config.rs` - Network config request/response
      - `form.rs` - Form state management
      - `verification.rs` - IP check and rollback logic
  - `websocket.rs` - WebSocket/Centrifugo handlers
  - `ui.rs` - UI action handlers (clear error/success)
- `src/commands/centrifugo.rs` - Custom WebSocket commands

## Building

### For Testing

```bash
cargo test -p omnect-ui-core
```

### For WASM (Web)

First, install wasm-pack:

```bash
cargo install wasm-pack
```

Then build:

```bash
cd src/app
wasm-pack build --target web --out-dir ../ui/src/core/pkg
```

This will generate the WASM module in `src/ui/src/core/pkg/`.

### Generate TypeScript Types

Make sure pnpm is in your PATH, then:

```bash
cargo build -p shared_types
```

This generates TypeScript types in `src/shared_types/generated/typescript/`.

## Integration with Vue

The Vue shell uses the `useCore()` composable (in `src/ui/src/composables/useCore.ts`) to interact with the Crux Core:

```typescript
const { viewModel, sendEvent, login, logout } = useCore()

// Send an event
await login('password')

// Access the view model
const isLoading = computed(() => viewModel.is_loading)
const errorMessage = computed(() => viewModel.error_message)
```

## Event Flow

1. User action in Vue component
2. Vue calls `sendEvent()` or convenience method
3. Event is serialized and sent to WASM core
4. Core updates Model and returns Effects
5. Effects are processed (HTTP requests, render updates, etc.)
6. ViewModel is updated and Vue re-renders

## Testing

The core includes unit tests for business logic:

```bash
cargo test -p omnect-ui-core
```

Run with clippy:

```bash
cargo clippy -p omnect-ui-core -- -D warnings
```

## Current Status

**Additional Tasks:**

- [ ] Performance bundle size optimization

### Technical Debt

- [ ] Refactor `Model.auth_token` to not be serialized to the view model directly. The current approach of removing `#[serde(skip_serializing)]` in `src/app/src/model.rs` is a workaround for `shared_types` deserialization misalignment. A long-term solution should involve either making TypeGen respect `skip_serializing` or separating view-specific model fields.
