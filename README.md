# CM

CM is a Creative Memories photo management tool with both CLI and GUI workflows.

The next major direction is captured in:

- [CM v2 vision](docs/cm-v2-vision.md)
- [CM v2 decision graph](docs/cm-v2-decision-graph.md)
- [CM v2 implementation slices](docs/cm-v2-implementation-slices.md)

## Remaining Work

### Layout

Tab layouts should be saveable in a menu in the file menu.
Layout > Preset 1 | Preset 2 | Save New Preset

### File renaming

- [ ] Regular expressions to rename
- [ ] 50 character (dynamic) length limit
    - [ ] List of rules (substrings to remove) to help shorten length, e.g., remove "pack" iff len(name) > 50

### Image crop-to-content

- [ ] Remove white padding around image

### Image resizing

- [ ] Target dimensions
- [ ] Target filesize
- [ ] Reencode to better file format? Try them all and pick best? can't be webp

### Metadata fetching

- [ ] Search CM site to find the price for the given SKU if exists


### New stuff

Dump json from https://www.creativememories.ca/new.html
