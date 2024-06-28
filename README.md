# flamelens

`flamelens` is an interactive FlameGraph viewer in the command line.

### Usage

Run `flamelens` with the filename of the profiling data in the form of "folded stacks":

```
flamelens <folded-stacks-filename>
```

Display a live FlameGraph of a running Python program using `py-spy` as the profiler (sudo likely
required):

```
flamelens --pid <pid-of-python-program>
```

### Key bindings
Key | Action
--- | ---
`hjkl` (or `← ↓ ↑→ `) | Navigate cursor for frame selection
`f` | Scroll down
`b` | Scroll up
`G` | Go to bottom
`g` | Go to top
`/<regex>` | Find and highlight frames matching the regex
`#` | Find and highlight frames matching the selected frame
`z` (in Live mode) | Freeze the FlameGraph
`q` (or `Ctrl + c`) | Exit

### Installation

```
cargo install flamelens
```