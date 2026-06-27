## Git / Commit Style
- Commit message subject line must use linux style: `subsystem: summary phrase` (all lower case).
- **ALWAYS** increment `[workspace.package].version` in `Cargo.toml` before building/shipping a `client-services` binary intended for testing or release when the old version string would make OTEL/support triage ambiguous. `client-services` surfaces that version through OTEL `service.version`, remote-log scope version, HTTP user agent, and Windows file metadata, so leaving it unchanged makes old and new binaries look the same in telemetry.

# Rust Guidelines
- Target Rust 1.91 version using 2024 edition on stable toolchains.
- Follow SOLID and other patterns from the Rust Design Patterns book.

## Tokio-native callback ports (do not skip)
- **ALWAYS** model callbacks as explicit message passing (`tokio::sync::*`) and/or a small async `trait`; do not keep ad-hoc "register callback fn" style APIs in Rust unless an external ABI requires it.
- Automatically choose the primitive based on semantics:
  - Exactly-once completion/reply: `tokio::sync::oneshot`
  - Many items / work queue with backpressure: bounded `tokio::sync::mpsc`
  - Pub-sub events with multiple subscribers: `tokio::sync::broadcast`
  - "Latest value" state feed: `tokio::sync::watch`
  - Wakeup/signal without payload: `tokio::sync::Notify`
- Request/response pattern: send a typed `Command` over `mpsc` that includes a `oneshot::Sender<Result<T>>` for the reply.
- If a clear interface boundary exists, define a consumer-layer `trait` and inject it (Dependency Inversion); prefer native `async fn` in traits if MSRV supports it, otherwise use `async_trait`.
- **NEVER** use `std::sync::mpsc`, blocking waits, or `thread::spawn` for these ports; use `tokio::spawn`/`spawn_blocking`, and carry a `tokio::runtime::Handle` when bridging from non-async/FFI threads.


## Tracing macro guards (do not add)
`tracing` macros already short-circuit when disabled, so extra guards are redundant and can change semantics.

- **NEVER** wrap `tracing::{trace,debug,info,warn,error}!` or `tracing::event!` with `tracing::enabled!` / `tracing::level_enabled!` checks.
- **ONLY** use `tracing::enabled!` when guarding non-log side effects, and keep those side effects outside the log call.

## Constructors over struct literals (do not skip)
When porting OO-style code, keep data encapsulated and construct types via `new`/`try_new`/builders instead of public field init.

- **ALWAYS** prefer `MyType::new(...)` / `MyType::try_new(...)` (or a builder) over `MyType { ... }` in non-test code.
- **NEVER** make fields `pub` just to satisfy construction; keep fields private/`pub(crate)` and expose invariants via constructors + getters.
- If initialization is complex or has many parameters, introduce a builder (or a typed options struct) rather than a long `new(...)` signature.

## Rust SOLID + File Hygiene (do not skip)
- **ALWAYS** keep SRP at the file/module level: prefer **1 primary `struct`/`enum`/`trait` per file** (with its `impl`s). If a new responsibility is introduced, create a new module/file.
- **ALWAYS** before implementing a non-trivial change, write a short "responsibility map": which types/modules you'll add, the file path for each, and what each owns (then implement to that map).
- **NEVER** grow "god files": if a Rust file exceeds **~300 LOC** or contains **>2 public types**, split it into smaller modules/files.
- **ALWAYS** keep functions single-purpose: if a function mixes **I/O** (DB/fs/net), **parsing/validation**, and **domain logic**, split it into helpers and/or new types (also split if > **~50 LOC** or > **3** nested scopes).
- **ALWAYS** apply Dependency Inversion: define small `trait`s in the consumer layer (domain) and inject implementations from infra/adapters (don't call DB/fs/net directly from domain logic).
- **ALWAYS** keep `mod.rs` minimal: module declarations + re-exports only; put real logic in sibling files.
- **ALWAYS** if any rule above is intentionally violated, explicitly justify it in the final response.

## Rust Anti-Patterns (do not use)
- **NEVER** "clone to satisfy the borrow checker": do not add `clone()`/`to_owned()` just to make the compiler happy. Refactor ownership/borrows; if a clone is truly semantically required, explicitly justify it (what is cloned + why it's correct/acceptable).
- **NEVER** add `#![deny(warnings)]` in code. Fix warnings or enforce in tooling/CI instead; do not make local development/builds brittle by turning all warnings into hard errors in the crate source.
- **NEVER** use "Deref polymorphism": do not implement `Deref` to get implicit method forwarding / pseudo-inheritance. Only implement `Deref` for true smart-pointer semantics; otherwise prefer explicit APIs (`as_*`/`into_*`), `AsRef`, `Borrow`, or newtype methods.

## Rust OO Bias + DRY (do not skip)
- **ALWAYS** prefer "objects" (Rust `struct`/`enum` + `impl`) over free-function soup: if behavior belongs to a domain concept, make it a method on that type (or a `trait` implemented by that type).
- **NEVER** create large "utils" modules of unrelated functions. If you feel compelled to add many helpers, introduce a cohesive type and move the helpers into methods.
- **ALWAYS** avoid duplicate logic: before adding a new helper/function, search the workspace (`rg` + rust-analyzer workspace symbols) for an existing equivalent; reuse or refactor to a shared implementation instead of copying.
- **ALWAYS** if you must keep two similar functions, document the semantic difference (different invariants, error handling, perf characteristics, or caller contract) in the final response.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- When writing tests, prefer comparing the equality of entire objects over fields one by one.
- Prefer SeaORM entities/modules/structs in their own files to maintain clarity.
- Run `cargo fmt` and `cargo check` before delivering changes.
