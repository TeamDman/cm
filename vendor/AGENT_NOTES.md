# Agent Notes For Vendored Windows Reactor

This note is for agents or maintainers arriving with little context. It summarizes how the vendored Reactor runtime works in `cm`, what to be careful about, and where to look first.

## What This Is

This is a Rust-driven retained UI runtime for WinUI 3. It is not immediate-mode drawing. Instead, Rust code builds an `Element` tree, the reconciler diffs it against the previous tree, and the backend mutates real WinUI controls.

The main pipeline is:

- `App`
- `ReactorHost`
- `RenderHost`
- `Reconciler`
- `WinUIBackend`
- real WinUI 3 controls

Start reading here:

- [vendor/windows-reactor/src/app.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\app.rs)
- [vendor/windows-reactor/src/winui/host.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\winui\host.rs)
- [vendor/windows-reactor/src/core/render_host.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\render_host.rs)
- [vendor/windows-reactor/src/core/reconciler.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\reconciler.rs)
- [vendor/windows-reactor/src/winui/backend/mod.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\winui\backend\mod.rs)

## Mental Model

Design with this as if it were a small React-style runtime targeting WinUI 3:

- components render `Element` trees
- hooks store component-local state
- rerenders are scheduled and coalesced
- reconciliation decides what real controls to create, reuse, update, or destroy

Do not think of it as "draw pixels from Rust every frame."

## Main Things To Be Aware Of

### 1. UI thread ownership matters

WinUI and COM objects want to be touched on the UI thread. Off-thread work must marshal back through the UI dispatcher and `UiMarshaller`.

Relevant files:

- [vendor/windows-reactor/src/core/dispatcher.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\dispatcher.rs)
- [vendor/windows-reactor/src/core/render_context.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\render_context.rs)
- [vendor/windows-reactor/src/winui/dispatcher.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\winui\dispatcher.rs)

### 2. Rendering is tree build -> reconcile -> effects

`RenderHost` does not mutate controls directly from component code. It:

- builds a new tree
- reconciles it against the old tree
- flushes effects after the tree update

That means:

- avoid imperative side-channel UI mutations when possible
- expect state updates to be batched/coalesced
- use effects for real side effects, not normal UI data flow

Relevant file:

- [vendor/windows-reactor/src/core/render_host.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\render_host.rs)

### 3. Hook order is a correctness rule

`RenderCx` stores hook state in slot order, so changing hook ordering inside a component can break behavior in the same way React hooks can break.

Relevant file:

- [vendor/windows-reactor/src/core/render_context.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\render_context.rs)

### 4. Equality and identity are performance-critical

Components can skip updates when props and element structure are stable. Unnecessary prop churn or unstable tree structure causes more reconciliation work and more WinUI churn.

Relevant files:

- [vendor/windows-reactor/src/core/component.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\component.rs)
- [vendor/windows-reactor/src/core/reconciler.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\core\reconciler.rs)

### 5. Host-level behavior belongs in the host

Window lifecycle, title bar behavior, backdrop, DPI, drag and drop, and other Win32/COM integration belong in the host layer, not scattered through app components.

Relevant file:

- [vendor/windows-reactor/src/winui/host.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\winui\host.rs)

This is especially important in `cm`, because the file drop behavior required host-level changes.

### 6. The backend owns the real WinUI control graph

If you mutate WinUI controls outside the expected backend/reconciler flow, you can desynchronize Reactor's internal bookkeeping from the actual visual tree.

Relevant file:

- [vendor/windows-reactor/src/winui/backend/mod.rs](G:\Programming\Repos\cm\vendor\windows-reactor\src\winui\backend\mod.rs)

## Current `cm`-Specific Practical Gotchas

- The vendored Reactor copy includes local behavior that may not exist in upstream `windows-rs` yet.
- `vendor/windows-reactor/src/winui/host.rs` is a likely conflict hotspot during upstream resync.
- File drop support currently lives in the vendored host, which means app behavior depends on runtime behavior, not just `src/reactor/app.rs`.
- Some thread-local assumptions exist in the runtime, so it is best to think in terms of one active host per UI thread unless you are deliberately extending the architecture.

## Suggested Layering For Future Changes

Use this rule of thumb:

- app flow, wizard UX, `cm`-specific surfaces -> `src/reactor/*`
- generic window integration, drag and drop, host services -> `vendor/windows-reactor/src/winui/host.rs`
- generic render scheduling / hooks / reconciliation behavior -> `vendor/windows-reactor/src/core/*`
- actual WinUI control creation or property wiring -> `vendor/windows-reactor/src/winui/backend/*`

## If You Need To Change Something

Start with these questions:

1. Is this app-specific behavior, or runtime behavior?
2. Does this need to run on the UI thread?
3. Is this state-driven, or is it truly a side effect?
4. Will this interfere with reconciler identity, hook ordering, or backend bookkeeping?

## Quick Orientation Checklist

- Read [vendor/upstream.md](G:\Programming\Repos\cm\vendor\upstream.md) first.
- Confirm whether the behavior is app-layer or runtime-layer.
- Check `host.rs` before adding Windows-specific logic to components.
- Run `.\check-all.ps1` after changes.
