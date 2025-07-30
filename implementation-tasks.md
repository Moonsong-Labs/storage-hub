# Implementation Tasks - Phased Approach

## ⚠️ IMPORTANT COMPILATION NOTE

When compiling or checking this project, you **MUST** run commands from the workspace root WITHOUT specifying any specific package:

✅ **CORRECT**: `cargo check`  
❌ **INCORRECT**: `cargo check -p sh-backend-lib`  

---

## Phase 1: Foundation Setup
**No dependencies - all tasks can be done in parallel**

### Task 1.1: Update Dependencies
- File: `mock-endpoints/backend/lib/Cargo.toml`
- Add new dependencies for JWT, multipart, etc.
- Assignee: Developer A

### Task 1.2: Create Models Module Structure
- Create `mock-endpoints/backend/lib/src/models/mod.rs`
- Update `mock-endpoints/backend/lib/src/lib.rs` to include models module
- Assignee: Developer B

### Task 1.3: Create Validation Utilities
- Create `mock-endpoints/backend/lib/src/api/validation.rs`
- Update `mock-endpoints/backend/lib/src/api/mod.rs` to export validation
- Assignee: Developer C

---

## Phase 2: Model Definitions
**Depends on Phase 1.2 - all model tasks can be done in parallel**

### Task 2.1: Authentication Models
- Create `mock-endpoints/backend/lib/src/models/auth.rs`
- Define NonceRequest, NonceResponse, VerifyRequest, VerifyResponse, etc.
- Assignee: Developer A

### Task 2.2: MSP Info Models
- Create `mock-endpoints/backend/lib/src/models/msp_info.rs`
- Define InfoResponse, StatsResponse, Capacity, ValueProp, MspHealthResponse
- Assignee: Developer B

### Task 2.3: Bucket Models
- Create `mock-endpoints/backend/lib/src/models/buckets.rs`
- Define Bucket, FileTree structures
- Assignee: Developer C

### Task 2.4: File Operation Models
- Create `mock-endpoints/backend/lib/src/models/files.rs`
- Define FileInfo, DistributeResponse
- Assignee: Developer D

### Task 2.5: Payment Models
- Create `mock-endpoints/backend/lib/src/models/payment.rs`
- Define PaymentStream structure
- Assignee: Developer E

---

## Phase 3: Service Layer
**Depends on Phase 2 - services can be done in parallel**

### Task 3.1: Auth Service
- Create `mock-endpoints/backend/lib/src/services/auth.rs`
- Implement generate_nonce, verify_signature, refresh_token, get_profile, logout
- Assignee: Developer A

### Task 3.2: MSP Service
- Create `mock-endpoints/backend/lib/src/services/msp.rs`
- Implement all MSP-related methods (get_info, get_stats, get_value_props, etc.)
- Assignee: Developer B

### Task 3.3: Update Services Module
- Update `mock-endpoints/backend/lib/src/services/mod.rs`
- Add auth and msp modules, update Services struct
- Assignee: Developer C

---

## Phase 4: Handler Implementation
**Depends on Phase 3 - single task due to file size**

### Task 4.1: Create MSP Handlers
- Create `mock-endpoints/backend/lib/src/api/msp_handlers.rs`
- Implement all HTTP handlers for MSP endpoints
- Note: This is a large file but can't be easily split
- Assignee: Developer A & B (pair programming recommended)

---

## Phase 5: Integration
**Depends on Phase 4 - tasks must be done sequentially**

### Task 5.1: Update Routes
- Update `mock-endpoints/backend/lib/src/api/routes.rs`
- Add all MSP routes in correct order (file routes order matters!)
- Assignee: Developer A

### Task 5.2: Update Error Handling
- Update `mock-endpoints/backend/lib/src/error.rs`
- Add new error variants and update IntoResponse implementation
- Assignee: Developer B

---

## Phase 6: Testing & Verification
**Depends on Phase 5 - all tests can run in parallel**

### Task 6.1: Compilation Check
- Run `cargo check` from workspace root
- Fix any compilation errors
- Assignee: Developer A

### Task 6.2: Public Endpoint Testing
- Test /info, /stats, /value-props, /health endpoints
- Verify responses match expected format
- Assignee: Developer B

### Task 6.3: Auth Flow Testing
- Test complete auth flow: nonce → verify → profile
- Test token refresh and logout
- Assignee: Developer C

### Task 6.4: Authenticated Endpoint Testing
- Test bucket operations with auth headers
- Test file operations (download, upload, distribute)
- Test payment stream endpoint
- Assignee: Developer D

---

## Task Dependencies Graph

```
Phase 1 (Parallel)
├── Task 1.1: Dependencies
├── Task 1.2: Models Module ─┐
└── Task 1.3: Validation     │
                             │
Phase 2 (Parallel) ◄─────────┘
├── Task 2.1: Auth Models
├── Task 2.2: MSP Models
├── Task 2.3: Bucket Models
├── Task 2.4: File Models
└── Task 2.5: Payment Models
              │
Phase 3 (Parallel) ◄─────────┘
├── Task 3.1: Auth Service
├── Task 3.2: MSP Service
└── Task 3.3: Update Services
              │
Phase 4 ◄────────────────────┘
└── Task 4.1: MSP Handlers
              │
Phase 5 (Sequential) ◄───────┘
├── Task 5.1: Routes
└── Task 5.2: Error Handling
              │
Phase 6 (Parallel) ◄─────────┘
├── Task 6.1: Compilation
├── Task 6.2: Public Tests
├── Task 6.3: Auth Tests
└── Task 6.4: Protected Tests
```

---

## Time Estimates

- **Phase 1**: 30 minutes (parallel)
- **Phase 2**: 45 minutes (parallel)
- **Phase 3**: 1 hour (parallel)
- **Phase 4**: 1.5 hours
- **Phase 5**: 30 minutes (sequential)
- **Phase 6**: 45 minutes (parallel)

**Total time with 5 developers**: ~4 hours
**Total time with 1 developer**: ~4.5 hours

---

## Critical Path

The critical path is: Phase 1.2 → Phase 2 (any) → Phase 3 (any) → Phase 4 → Phase 5 → Phase 6

To optimize:
1. Ensure Task 1.2 (Models Module) is prioritized in Phase 1
2. Start Phase 4 (Handlers) as soon as any Phase 3 task completes
3. Have developers ready to start Phase 5 immediately after Phase 4