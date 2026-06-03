# CM v2 Decision Graph

This document names the user picks in CM v2 and the dependencies between them. It is
not the final UI. It is a durable description of the decisions that the UI should help
the user make.

## Pick Vocabulary

A pick is a user decision. A pick can be:

- absent
- cancelled
- invalid
- valid
- defaulted
- explicitly chosen

Every pick should have a typed value, validation expectations, and enough display
metadata for the GUI to explain its current state.

## Top-Level Flow

```text
pick tool choice
    Tool v1
    Tool v2
    Product Search

if Tool v1:
    run current workflow

if Tool v2:
    build a plan from user picks
    preview the plan
    execute the plan when the user submits it

if Product Search:
    run standalone product search workflow
```

## Core Picks

```text
pick input paths as set of paths
suggest recent input paths
allow browse
allow drag and drop
expect each input path exists

pick input image paths as set of paths
derive suggestions from input paths and directory descendants
expect each input image path exists and is an image

pick output dir path as path
suggest recent output dirs
suggest sibling output dirs derived from input paths
allow browse
allow drag and drop
expect output dir path is a dir or does not exist
allow create output dir
allow open output dir
```

## Output Path Picks

These depend on a valid output directory.

```text
pick should flatten output hierarchy as bool default true
pick should save all inputs to same folder as bool default false
pick shared output dir as optional path
```

The output path plan should prove that output paths do not collide with input paths.
When collisions or duplicate output paths are found, the user should see the conflict
before execution.

## Crop Picks

These depend on crop-to-content being enabled.

```text
pick should crop images to content as bool default true

if should crop images to content:
    pick crop background detection threshold as integer where 0 <= value <= 255
    pick threshold preview mode
    pick should show bounding box as bool default true
```

Preview implications:

- input image preview remains the source view
- threshold preview shows detected content vs background
- output preview shows the cropped result

## Compression Picks

```text
pick should reduce file size as bool default true

if should reduce file size:
    pick preferred max file size as file size
    pick jpeg quality as integer where 1 <= value <= 100
    pick should update image metadata if dimensions change as bool default true
```

The plan should show estimated output size when available.

## Rename Picks

```text
pick should rename images as bool default true

if should rename images:
    pick file name find and replace rules as list
    pick should hyphenate camelCase as bool default false
    pick should enforce max file name length as bool default true

if should enforce max file name length:
    pick preferred max file name length as integer greater than 0
```

Each rename rule has:

- find string or regex
- replace string
- enabled flag
- case sensitive flag
- only when name too long flag

Preview implications:

- each rename operation should have before and after examples
- the combined rename pipeline should show cumulative effects
- rules that do not affect any current image should be visually quiet
- when max-name enforcement is disabled, long-name warnings and
  only-when-too-long rules should be inactive

## Plan Shape

```text
plan:
    inputs:
        input file path
        input file contents
    transformations:
        crop
        compress
        rename
        metadata update
    outputs:
        output file path
        output file contents
    expectations:
        input exists
        output parent exists or can be created
        output path does not overwrite input path
        output path collisions are handled
```

The plan should be serializable so it can be inspected, resumed, and eventually tested
without requiring the GUI to be open.

## UI Revealing Rule

The UI should reveal controls according to the dependency graph:

- show a decision when its prerequisites are satisfied
- show why a decision is waiting when its prerequisites are absent
- keep reset-to-default available when a value differs from its default
- keep previews close to the decision that changes them
