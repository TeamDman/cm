# CM v2 Implementation Slices

These slices are ordered to preserve the current app while creating a real path toward
the decision-builder model.

## Slice 1: Name The Decisions

Create a typed representation for user picks without changing the visible UI much.

Possible starting types:

```text
DecisionId
DecisionState<T>
DecisionStatus
DecisionDependency
DecisionGraph
Plan
PlanStep
PlanExpectation
```

The first implementation can mirror fields already present in `AppState`.

Status: started in `src/gui/plan.rs` with `DecisionSummary`,
`DecisionStatus`, `PlanEntry`, and `CmPlan`.

## Slice 2: Serialize The Plan

Add a plan export that writes the current inferred plan to an inspectable text or JSON
file. This gives agents and humans a stable artifact to review.

The export should include:

- selected input paths
- derived image paths
- output path options
- image processing settings
- rename settings
- planned output paths
- validation failures

Status: started with a V2 plan tile export to `APP_HOME/last-plan.txt` for humans
and `APP_HOME/last-plan.json` for structured agent/automation inspection.
The export action is available from both the Plan tile and the Studio Guide Run step.

## Slice 3: Add A Plan Tile

Add a V2-only tile that displays the current plan and validation state.

Useful sections:

- decisions made
- waiting decisions
- planned transformations
- planned outputs
- conflicts and errors

Status: started in `src/gui/tiles/plan.rs`.

## Slice 3.5: Add A Studio Guide

Add a V2-only guide tile that gives Mom a direct sequence through the workflow while
preserving the surrounding poweruser panes.

Useful sections:

- pick photos
- review discovered images
- choose output shape
- tune image processing
- tune names
- preview and run

Status: started in `src/gui/tiles/studio_guide.rs`.

Current behavior: the guide now exposes actionable controls for adding input folders
or files, removing individual inputs, clearing/refreshing inputs, selecting any
discovered image, inspecting selected-image metadata, choosing output shape, setting a
shared output folder, tuning crop/compression, configuring rename and max-name choices,
editing find/replace rename rules, enabling auto-search during processing, and running
all or selected images. The Processing step also mirrors V1's selected-image feedback
by showing input size, estimated output size, dimension changes, and the preview
pan/zoom sync toggle. The Run step records and displays the latest processing result,
including processed counts and error details, after Process All or Process Selected
finishes.

The guide also has persistent wizard step state, a clickable step rail, and Back/Next
navigation so Mom sees one focused decision surface at a time while poweruser panes
remain available around it.

Product Search is now a separate main-menu mode instead of a pane inside the V1 or V2
studios. It also skips the studio Layout menu and studio layout autosave so opening
the standalone utility cannot overwrite a V1 or V2 layout.

Automated layout tests now assert that V2 includes every core V1 studio pane plus
`StudioGuide` and `Plan`, while Product Search remains a standalone mode.

## Slice 4: Gate Controls By Dependencies

Move from "all controls are visible" toward "controls reveal themselves when their
prerequisites are met."

Start small:

- output options depend on an output directory
- crop threshold controls depend on crop-to-content
- max name length value depends on max-name enforcement
- rename rule details depend on renaming being enabled

Status: started by disabling crop threshold/preview controls unless crop-to-content
is enabled, disabling rename rule details unless rename rules are enabled, and
making max-name enforcement an explicit persisted decision. When max-name enforcement
is off, the output preview stops warning about long names and rename rules marked
`only when name too long` stay inactive. The crop threshold preview also has an
explicit bounding-box visibility decision; turning it off removes the red preview box
and disables the thickness control.

The reduce-file-size decision is now named consistently in the V1 settings tile, V2
Studio Guide, and plan. Existing users still start with reduction disabled, while
turning it on seeds the decision-graph default of 50 MB.

Output folder picks now validate the graph expectation that an output directory must
either be an existing directory or a not-yet-created path. The Studio Guide shows
whether the shared output folder exists, needs creation, or is invalid, and splits
Create and Open into separate actions.

## Slice 5: Preview Each Transformation

Make the pipeline more inspectable by showing before and after views for each major
operation.

Candidates:

- rename before/after
- crop before/after
- compression estimated size before/after
- output path before/after

Status: started by adding named per-entry transformations to `CmPlan` and showing
them in the Plan tile plus the Studio Guide run preview. Current transformations name
read, rename, image processing, metadata/search behavior, output path reservation,
and write. Output path collisions now count as visible plan events: duplicate outputs,
existing-file conflicts, and input-overwrite conflicts show the desired path and the
reserved safe path before execution.

## Slice 6: Persist Recent Picks

Persist recent pick values as suggestions. This gives the app memory without hiding
the user's current decisions.

Candidates:

- recent input paths
- recent output directories
- recent crop thresholds
- recent max file sizes
- recent rename rule sets

Status: started by persisting recent shared output folders in
`recent_output_dirs.txt` under `APP_HOME` and exposing the most recent picks in the
Studio Guide output-shape step. The output-shape step also suggests output folders
derived from the current input selection, including the shared-base output folder and
per-input `-output` folders. The pick-photos step now persists recent input paths in
`recent_input_paths.txt` and exposes the most recent existing paths as quick picks.

## Slice 7: Decide What V1 Means Long Term

V1 can remain the current direct workflow. V2 can become the explicit plan workflow.
Eventually, V1 may simply be a preset layout or a simplified view over the same plan
engine.

## Open Questions

- Should plans be stored under `APP_HOME`, the output directory, or both?
- Should the plan format be JSON, a custom text grammar, or both?
- Should V2 have its own `AppState` fields, or should the existing fields become
  decision values?
- Should each tile own its decision rendering, or should decision rendering be
  centralized?
- What is the smallest useful plan export that helps Mom and helps agents?
