# Vendored Windows Reactor Upstream Notes

This `vendor/` area contains the Windows Reactor runtime that `cm` currently owns locally:

- `vendor/windows-reactor`
- `vendor/windows-reactor-setup`

## Upstream Source

- Upstream repository: `https://github.com/microsoft/windows-rs`
- Upstream base revision recorded in the vendored crates:
  - `826ac184fc77e3875b2af1b616889413f327a6e5`

## Important Provenance Detail

The current vendored contents were copied from the local working tree at:

- `D:\Repos\rust\windows-rs\crates\libs\reactor`
- `D:\Repos\rust\windows-rs\crates\libs\reactor-setup`

That means this vendor snapshot is based on upstream revision `826ac184fc77e3875b2af1b616889413f327a6e5`, but it is not guaranteed to match that upstream commit exactly. Local modifications from the `windows-rs` working tree were intentionally brought over with the copy.

## Why We Vendor This

We are vendoring Reactor instead of maintaining a fork right now because:

- `cm` needs rapid iteration on Windows-specific behavior.
- Drag and drop required host-level Reactor changes, not just app-level code.
- The Reactor feature is still very new, so upstream may continue to evolve quickly.
- Copying the runtime into `cm` lowers friction for experimentation and product-driven changes.

## Expected Local Ownership Areas

The places most likely to diverge from upstream are:

- `vendor/windows-reactor/src/winui/host.rs`
  - window lifecycle
  - file drop support
  - title bar and backdrop behavior
  - Win32 / COM interop
- `vendor/windows-reactor/src/core/*`
  - render scheduling
  - reconciler behavior
  - hooks / async state behavior
- `vendor/windows-reactor-setup/*`
  - self-contained bootstrap and packaging behavior

## Resync Guidance

If we want newer upstream Reactor changes later:

1. Compare the current vendored tree against the current upstream `windows-rs` Reactor sources.
2. Reconcile host-level local changes first, especially in `src/winui/host.rs`.
3. Re-run `.\check-all.ps1` in `cm` after every meaningful merge step.
4. Update this file with the new base revision and any notable provenance details.

## Notes For Future Maintainers

- Treat this as a tracked upstream snapshot, not a throwaway copy.
- Keep app-specific behavior in `cm` when possible.
- Keep generic host/runtime improvements in the vendored Reactor layer.
- If a local change seems broadly useful, consider upstreaming it later once the behavior stabilizes.
