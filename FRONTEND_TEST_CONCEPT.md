# Testing Concept: Omnect-UI (Strategy: Core-First)

## Strategy

Leverage the Crux architecture's testability by design. The Core contains all business logic as pure, deterministic functions - making it the highest-ROI test target. The Shell is intentionally thin (renders ViewModel, executes effects) and needs minimal testing.

**Approach:** Test the Core exhaustively (cheap, fast, deterministic), keep E2E minimal for regression safety.

**Phase 1 Status:** ⏳ **In Progress** (17 tests in this branch)

- PR #77: Authentication Tests (17 tests) - ✅ Complete (this branch)
- PR #78: Device Tests - 🔄 Pending
- PR #79: Network Tests - 🔄 Pending
- PR #80: Reconnection Tests - 🔄 Pending

## Implementation Plan

### Phase 1: Core State Transitions (Unit Tests)

*Goal: Secure business logic and state machines with fast, deterministic tests.*

#### PR 1.1: Authentication Tests ✅
- [x] Test login flow (loading state, success, failure)
- [x] Test logout and session cleanup
- [x] Test token management state
- [x] Test password change flow

#### PR 1.2: Device Tests 🔄

- [ ] Test system info updates (WebSocket events)
- [ ] Test online status transitions
- [ ] Test factory reset state machine
- [ ] Test reboot flows
- [ ] Test firmware upload state transitions

#### PR 1.3: Network Tests 🔄

- [ ] Test network configuration updates
- [ ] Test IP change detection and rollback state
- [ ] Test DHCP/static switching logic
- [ ] Test network form state management

#### PR 1.4: Update/Reconnection Tests 🔄

- [ ] Test reconnection state machine for all operations (reboot, factory reset, update)
- [ ] Test reconnection timeout handling with operation-specific durations
- [ ] Test update completion detection based on validation status
- [ ] Test healthcheck response handling during reconnection
- [ ] Test network IP reachability detection

**Note:** Original PR 1.5 (WebSocket Tests) was merged into PR 1.2 as WebSocket event handling tests are naturally colocated with device state updates.

### Phase 2: Core Effect Emissions

**Status:** 🚫 **Skipped - Not Recommended**

After implementing Phase 1, we've determined that effect emission testing provides minimal value:

**Why Skip Effect Testing:**

1. **Implementation Detail Testing**: Effects are how the Core communicates with the Shell, not what it does. Testing effect structure couples tests to implementation details.

2. **Macros Handle Correctness**: The codebase uses well-tested macros (`auth_post!`, `http_get!`, `http_get_silent!`) that generate effects consistently. These macros are the single source of truth for effect creation.

3. **Auto-Generated Types**: The `Effect` enum is auto-generated via `#[derive(crux_core::macros::Effect)]`. Testing against generated types is brittle and adds maintenance burden.

4. **Response Testing is Sufficient**: Phase 1 already tests response handling (e.g., `LoginResponse`, `SetNetworkConfigResponse`), which validates the complete request/response cycle behavior from the user's perspective.

5. **Integration Coverage**: E2E tests (Phase 3) will validate actual HTTP requests reach the backend correctly.

**What We Test Instead:**
- ✅ State transitions (Phase 1) - validates business logic
- ✅ Response handling (Phase 1) - validates correct reactions to success/error
- ✅ Critical paths (Phase 3) - validates actual network communication

**Original Phase 2 Tasks** (archived for reference):
- ~~Test login emits correct POST request~~
- ~~Test authenticated requests include bearer token~~
- ~~Test network config changes emit correct payloads~~
- ~~Test Centrifugo connection/subscription effects~~

### Phase 3: E2E Regression Tests (Selective)

*Goal: Guard critical user journeys against regression. Keep minimal.*

#### PR 3.1: E2E Infrastructure
- [ ] Set up Playwright with minimal config
- [ ] Create test fixtures for mock backend responses
- [ ] Document local test execution

#### PR 3.2: Critical Path Tests
- [ ] Test: Login → View device info → Logout
- [ ] Test: Authentication redirect (unauthenticated access)
- [ ] Test: Network settings change with rollback UI

## Test Patterns

### State Transition Test
```rust
#[test]
fn test_login_sets_loading() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    app.update(Event::Login { password: "test".into() }, &mut model);

    assert!(model.is_loading);
    assert!(model.error_message.is_none());
}
```

### Effect Emission Test (Not Recommended - See Phase 2)
```rust
// ❌ NOT RECOMMENDED: Testing implementation details
// Effects are auto-generated and handled by macros
// This test is brittle and provides minimal value

#[test]
fn test_login_emits_http_request() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let effects = app.update(Event::Login { password: "test".into() }, &mut model);

    // This tests HOW the Core communicates, not WHAT it does
    // Better to test state transitions and response handling instead
}
```

### Response Handling Test (✅ Recommended Pattern)
```rust
// ✅ RECOMMENDED: Test response handling and state changes
// This validates WHAT the Core does from the user's perspective

#[test]
fn test_login_success_sets_authenticated() {
    let app = AppTester::<App>::default();
    let mut model = Model {
        is_loading: true,
        ..Default::default()
    };

    let _ = app.update(
        Event::Auth(AuthEvent::LoginResponse(Ok(AuthToken {
            token: "test_token_123".into(),
        }))),
        &mut model,
    );

    assert!(model.is_authenticated);
    assert!(!model.is_loading);
    assert_eq!(model.auth_token, Some("test_token_123".into()));
}

#[test]
fn test_login_failure_sets_error() {
    let app = AppTester::<App>::default();
    let mut model = Model {
        is_loading: true,
        ..Default::default()
    };

    let _ = app.update(
        Event::Auth(AuthEvent::LoginResponse(Err("Invalid password".into()))),
        &mut model,
    );

    assert!(!model.is_authenticated);
    assert!(!model.is_loading);
    assert!(model.error_message.is_some());
}
```

### Colocated Test Pattern (✅ Used in Phase 1)
```rust
// Tests are colocated with the code they test using #[cfg(test)] mod tests
// Example: src/app/src/update/auth.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{AuthEvent, Event};
    use crate::model::Model;
    use crate::types::AuthToken;
    use crate::App;
    use crux_core::testing::AppTester;

    mod login {
        use super::*;

        #[test]
        fn success_sets_authenticated_and_stores_token() {
            let app = AppTester::<App>::default();
            let mut model = Model {
                is_loading: true,
                ..Default::default()
            };

            let _ = app.update(
                Event::Auth(AuthEvent::LoginResponse(Ok(AuthToken {
                    token: "test_token_123".into(),
                }))),
                &mut model,
            );

            assert!(model.is_authenticated);
            assert!(!model.is_loading);
            assert_eq!(model.auth_token, Some("test_token_123".into()));
        }
    }
}
```

## Tools

| Scope | Tool | Purpose |
|:------|:-----|:--------|
| **Core Logic** | `cargo test` + `crux_core::testing` | State transitions, effect emissions |
| **E2E** | Playwright | Critical user journey regression |

## ROI Summary

| Phase | Speed | Stability | Coverage | Priority | Status |
|:------|:------|:----------|:---------|:---------|:-------|
| Core State Tests | Fast (ms) | Deterministic | High | **High** | ⏳ **In Progress (17/92 tests)** |
| ~~Core Effect Tests~~ | ~~Fast (ms)~~ | ~~Deterministic~~ | ~~High~~ | **Skipped** | 🚫 **Not recommended** |
| E2E Tests | Slow (s) | Flaky-prone | Low | Low | ⏳ **Planned (Phase 3)** |

## Lessons Learned

### What Worked Well
1. **Colocated Tests**: Keeping tests next to the code they test (`#[cfg(test)] mod tests`) improves maintainability
2. **Domain Organization**: Organizing tests by domain (auth, device, network) mirrors code structure
3. **Response-Focused Testing**: Testing response handling validates behavior without coupling to implementation
4. **State Machine Validation**: Comprehensive state transition testing catches edge cases early

### What to Avoid
1. **Effect Emission Testing**: Testing auto-generated effect structures is brittle and low-value
2. **Testing Macros**: Well-tested macros (`auth_post!`) don't need per-use validation
3. **Testing Request Events**: Events that trigger HTTP requests don't have immediate state changes to test

### Key Patterns
- Use `let _ = app.update(...)` to ignore unused `Update<Effect, Event>` results
- Test response events (e.g., `LoginResponse`) not request events (e.g., `Login`)
- Organize tests in nested modules matching code structure
- Use helper functions to create test data (e.g., `create_healthcheck()`)
- Test state transitions, not implementation details
