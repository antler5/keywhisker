A layout generation library built on top of keycat.

# Upstream

Find the upstream repository here!

https://github.com/semilin/keywhisker

# Changelog

```
2024-12-06  antlers  <antlers@illucid.net>

	heatmap.py: Make the graph even prettier
	* bin/heatmap.py: Add `-H` flag for hexbins.
	* bin/heatmap.py: Upcase metric labels w/o numbers in them.
	* bin/heatmap.py: Format axis labels as percents.
	* bin/heatmap.py: Put known layouts w/ SFB < 55 into the dataframe
	before plotting. (Allows `range: None` to fit the viewport around them.)
	* bin/heatmap.py: Adjust background and label colors.
	* bin/heatmap.py: Add best-scoring layout to known-layouts.
	* bin/heatmap.py: Add type-hints for mypy
	* README: Highlight heatmap.py changes.

	Make metric weights signed so m3roll can be inverted
	* src/analysis.rs: Metrics weights u16 -> i16
	* src/main.rs: Metrics weights u16 -> i16

	README: Add README highlighting upstream and changes
	* README.md: New file.

2024-12-05  antlers  <antlers@illucid.net>

	heatmap.py: Revamp from `main.py`
	* heatmap.py: Rename `main.py` to `heatmap.py`.
	* heatmap.py: Add `pipx` shebang and PEP723 requirements metadata.
	* heatmap.py: Add docopt argument parsing.

2024-12-05  antlers  <antlers@illucid.net>

	run-generation: Create new file with random suffix.
	Lazy-ass way of multi-threading.
	Will error out if the file already exists.

	* src/analysis.rs(output_generation): Create new file with random
	suffix.

2024-12-05  antlers  <antlers@illucid.net>

	Fork from Semi <3
	* Cargo.lock: semilin->antler5
	* Cargo.toml: semilin->antler5
```
