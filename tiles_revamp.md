# Tiles Revamp Plan

This document tracks the migration from `egui-snarl` to `egui_tiles` for the CM GUI.

## Design Overview

We're switching from a node graph (snarl) to a tiled window layout (tiles) because our domain model is simpler than what a node graph provides. The tiles approach gives us:

- Dedicated panels for each feature
- Better use of screen real estate
- Easier to understand for end users
- More flexible layout (split, tabs, drag to rearrange)

## Tile Types

| Tile | Description | Status |
|------|-------------|--------|
| **Input Paths** | List of input directories with X buttons, clear all, drag-and-drop | ✅ |
| **Input Images Tree** | Hierarchical tree view of discovered image files | ✅ |
| **Rename Rules** | List of all rename rules (each rule inline, not separate tiles) | ✅ |
| **Max Name Length** | Global setting for max file name length | ✅ |
| **Output Preview** | Tree view with orange=renamed, red=too long | ✅ |
| **Input Image Preview** | Preview selected input image | ⬜ |
| **Output Image Preview** | Preview what the output image will look like | ⬜ |
| **Logs** | Tracing logs (off by default) | ✅ |

## Model Changes

### RenameRule (simplified)
```rust
pub struct RenameRule {
    pub id: Uuid,
    pub find: String,
    pub replace: String,
    pub case_sensitive: bool,          // NEW (inverted from old CaseInsensitive)
    pub only_when_name_too_long: bool,  // NEW (replaces WhenExpr + Modifier)
}
```

### Files to Delete
- [x] `src/rename_rules/when_expr.rs`
- [x] `src/rename_rules/modifier.rs`

### Files to Modify
- [x] `src/rename_rules/mod.rs` - Remove WhenExpr/Modifier exports
- [x] `src/rename_rules/rename_rule.rs` - Simplify struct

## File Structure (new gui/)

```
src/gui/
├── mod.rs                 # Main entry, run_gui, CmApp, eframe::App impl
├── behavior.rs            # CmPane enum, Behavior impl
├── state.rs               # Shared app state
├── tree_view.rs           # Reusable tree view helpers
├── tiles/
│   ├── mod.rs
│   ├── input_paths.rs     # Input paths tile
│   ├── input_images.rs    # Input images tree tile
│   ├── rename_rules.rs    # Rename rules tile (all rules in one tile)
│   ├── max_name_length.rs # Max name length setting tile
│   ├── output_preview.rs  # Output preview tree tile
│   ├── image_preview.rs   # Image preview tile (input + output variants)
│   └── logs.rs            # Logs tile
```

## Click Behavior

For tree views, when clicking a file:
- **Open New Tile**: Opens a new image preview tile
- **Replace Last Tile**: Replaces the last opened preview tile of the same type

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClickBehavior {
    OpenNewTile,
    ReplaceLastTile,
}
```

## Implementation Steps

### Phase 1: Model Refactoring
1. [x] Simplify `RenameRule` struct
2. [x] Delete `when_expr.rs` and `modifier.rs`
3. [x] Update `rename_rules/mod.rs`
4. [x] Update file format parsing/writing
5. [x] Update CLI command for rename rules

### Phase 2: GUI Scaffolding
1. [x] Create new `gui/` file structure
2. [x] Define `CmPane` enum for tile types
3. [x] Implement basic `Behavior<CmPane>` trait
4. [x] Create minimal CmApp with default tile layout

### Phase 3: Individual Tiles
1. [x] Input Paths tile
2. [x] Input Images Tree tile
3. [x] Rename Rules tile
4. [x] Max Name Length tile
5. [x] Output Preview tile
6. [ ] Input Image Preview tile (scaffolded, needs image loading)
7. [ ] Output Image Preview tile (scaffolded, needs image loading)
8. [x] Logs tile

### Phase 4: Interactions
1. [ ] Click behavior dropdown
2. [ ] File click -> open/replace preview tile
3. [x] Drag and drop files

### Phase 5: Cleanup
1. [ ] Remove egui-snarl from Cargo.toml
2. [ ] Test all functionality
3. [ ] Final polish

## Menu Bar

Keep the existing menu bar with:
- Refresh button
- Logs toggle
- About button
- Theme switch
- Quit button (right-aligned)

## Progress

- **Started**: 2026-01-05
- **Current Phase**: Phase 3 Complete (core tiles)
- **Last Updated**: 2026-01-05

## Notes

- The GUI is now functional with egui_tiles
- Drag-and-drop works for adding input directories
- Tiles can be rearranged by dragging
- Logs tile toggles on/off with the menu button
- Output preview shows renamed files in orange, too-long files in red
- Image preview tiles are scaffolded but need actual image loading implementation
