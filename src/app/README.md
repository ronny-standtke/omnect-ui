# Crux Core for omnect-ui

This package contains the Crux Core for the omnect-ui application. It implements the business logic and state management in Rust, which can be compiled to WebAssembly for use in the Vue frontend.

## Architecture

The Crux Core follows the Model-View-Update pattern:

- **Model** - The complete application state (auth, device info, network status, etc.)
- **ViewModel** - Data needed by the UI to render
- **Events** - Actions that can occur in the application
- **Capabilities** - Side effects (HTTP requests, WebSocket, rendering)

## Key Files

- `src/lib.rs` - App struct, Capabilities, and re-exports
- `src/model.rs` - Model and ViewModel structs
- `src/events.rs` - Event enum definitions
- `src/types.rs` - Shared data types
- `src/update/` - Domain-based event handlers
  - `mod.rs` - Main dispatcher and view function
  - `auth.rs` - Authentication handlers (login, logout, password)
  - `device.rs` - Device action handlers (reboot, factory reset, network, updates)
  - `websocket.rs` - WebSocket/Centrifugo handlers
  - `ui.rs` - UI action handlers (clear error/success)
- `src/capabilities/centrifugo.rs` - Custom WebSocket capability (deprecated API, kept for Effect enum generation)
- `src/capabilities/centrifugo_command.rs` - Command-based WebSocket capability (new API)

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

## Capabilities

### Render

Updates the ViewModel to trigger UI re-rendering.

### HTTP

Makes REST API calls to the backend. The shell handles the actual HTTP request and sends the response back to the core.

### Centrifugo

Manages WebSocket subscriptions for real-time updates. The shell handles the actual WebSocket connection.

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

### Completed Infrastructure

- [x] Complete WASM integration with wasm-pack
- [x] Implement full effect processing in Vue shell
- [x] Migrate all state management from Vue stores to Crux Core
- [x] Migrate Centrifugo capability to Command API (non-deprecated)
- [x] Migrate HTTP capability to Command API (non-deprecated)
- [x] Split monolithic lib.rs into domain-based modules
- [x] Suppress deprecated warnings with module-level `#![allow(deprecated)]`
- [x] Introduce shared_types crate for types shared between backend API and Crux Core
- [x] Create proof-of-concept component (DeviceInfoCore.vue)

### Vue Component Migration (Future PRs)

The Core infrastructure is complete, but most Vue components still use direct API calls (`useFetch`, `useCentrifuge`). These need to be migrated to use the Core:

**Components to Migrate:**

1. [ ] `DeviceActions.vue` - Reboot and factory reset actions
   - Replace `useFetch` POST calls with Core events
   - Replace `useCentrifuge` factory reset subscription with Core ViewModel
2. [x] `DeviceInfo.vue` - Replaced with `DeviceInfoCore.vue`
   - ~~Update import in `DeviceOverview.vue`~~
   - ~~Remove old `DeviceInfo.vue` file~~
3. [ ] `DeviceNetworks.vue` - Network list and status
   - Replace `useCentrifuge` subscription with Core ViewModel
4. [ ] `NetworkSettings.vue` - Network configuration
   - Replace `useFetch` POST calls with Core events
5. [ ] `UpdateFileUpload.vue` - Firmware update upload
   - Replace `useFetch` multipart upload with Core event
6. [ ] `UserMenu.vue` - User authentication actions
   - Replace `useFetch` logout with Core event

**Additional Tasks:**

- [ ] Remove `useCentrifuge` composable once all components migrated
- [ ] Add comprehensive integration tests for all migrated components
- [ ] Add more unit tests for Core edge cases
- [ ] Performance testing and bundle size optimization

### Technical Debt

- [ ] Remove deprecated capabilities once crux_core provides alternative Effect generation mechanism
