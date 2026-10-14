# Changelog

## 0.1.0 (unreleased)

First public version.

- Schematic capture with multi-sheet designs, wires, junctions, net labels,
  power flags, no-connects, annotation and ERC.
- PCB layout with update from schematic, ratsnest, interactive 45° routing
  with live clearance checks, vias, copper areas, board outline tools,
  1–6 copper layers, design rules with net classes and real-time DRC.
- 3D board preview with drilled holes and generated component bodies.
- Built-in library of 46 parts (including 2x3, 2x5 and 2x10 headers), symbol and footprint editors, device page,
  IC Symbol Wizard, pad-array generator and KiCad footprint/symbol import.
- Fabrication packages (Gerber X2, job file, Excellon, BOM, CPL) for
  JLCPCB, PCBWay, OSH Park, Eurocircuits and generic fabs; SVG and KiCad
  netlist export.
- Two example projects: a 555 LED blinker and a fully routed, DRC-clean
  3.3 V regulator board, both listed on the start page.
- Via stitching, Board Info (size, area, counts, routed length per net),
  Select Similar Parts, net length in track properties, a DRC warning for
  track ends that touch nothing and Route → Remove Dangling Tracks to clean
  them up, and ERC/DRC results saved as a text report.
- IPC-D-356 test netlist, included in every fabrication package.
- Local-first storage with autosave, files, version history and offline
  support; command palette and rebindable shortcuts; light and dark themes.
