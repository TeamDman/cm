# CM v2 Vision

CM is currently a tool for helping Mom process Creative Memories inventory images. The
next version should keep that practical job intact while making the application more
inspectable, explainable, and adaptable.

The core idea is that the user is not just filling out a form. The user is building a
plan. Each choice narrows or expands the plan, changes what can be previewed, and may
unlock later choices.

## Product Shape

The application starts with a main menu:

- Run tool v1
- Run tool v2
- Product Search

Tool v1 preserves the current workflow. Tool v2 explores a more explicit decision
builder where the app reveals controls according to the decisions already made.
Product Search is a standalone utility mode outside both studios.

## Design Principles

- Keep images as ordinary files on disk when a canvas or plan references them.
- Prefer inspectable text artifacts for intent, plans, decisions, and reusable rules.
- Treat user choices as typed values, not incidental widget state.
- Keep stdout available for user-selected output shapes in CLI mode.
- Use stderr for structured logs and diagnostics.
- Push privilege and global state upward toward the application entrypoint.
- Prefer parameterized logic over tests that mutate global process state.
- Make old behavior available while giving the new approach room to become better.

## Mental Model

A CM run is a builder pattern.

The user picks values:

- input paths
- input image paths
- output directory
- crop behavior
- compression behavior
- metadata behavior
- rename behavior
- file name length behavior
- selected preview image

The program turns those picks into a plan:

- input file path
- input file contents
- output file path
- output file contents
- validation expectations
- execution strategy

The user can inspect the plan before executing it.

## What V2 Should Add

V2 should make the implicit dependency graph visible. If the user has not picked an
output directory, output-only decisions should remain hidden, disabled, or grouped as
waiting on that pick. If the user enables crop-to-content, threshold controls and
preview affordances become relevant. If the user enables rename rules, before/after
name previews become relevant.

This can grow through the existing tile system:

- one tile per decision
- one tile per preview
- one tile for the complete plan
- one tile for execution progress and errors

The existing `egui_tiles` layout persistence is a good foundation for letting the user
arrange these views without forcing one final layout too early.

## Agent Inspectability

The app should accumulate project knowledge in files that agents can read without
replaying a conversation. Useful artifacts include:

- product direction
- decision graph
- user preference notes
- implementation commentary
- known limitations
- links to example repos
- raw source notes when preserving the original phrasing matters

The README should stay digestible. Deeper reasoning belongs in `docs/`.
