# Project file format

Projects are saved as `*.epro.json`: UTF-8 JSON, pretty-printed, with maps
sorted by key so that saving the same project twice produces the same bytes.
That makes the files friendly to git and code review.

```jsonc
{
  "format": "easypro-project",   // always this value
  "version": 1,                  // bumped when the layout changes; older files are migrated on load
  "name": "555 LED Blinker",
  "library": {                   // every part used is copied in, so the file is self-contained
    "symbols":    { "sym:resistor": { ... } },
    "footprints": { "fp:0603": { ... } },
    "devices":    { "dev:R-0603": { "symbol": "sym:resistor", "footprint": "fp:0603", ... } }
  },
  "boards": [
    {
      "id": 1,
      "name": "Board1",
      "schematic": { "sheets": [ { "components": [...], "wires": [...], "labels": [...], "ports": [...] } ] },
      "pcb": { "copper_layers": 2, "thickness": 1600000, "components": [...], "tracks": [...], "vias": [...], "pours": [...], "graphics": [...], "rules": { ... } }
    }
  ],
  "ids": { "next": 64 }          // id counter; repaired on load if a hand edit left it behind
}
```

## Units

All lengths are **integer nanometres** (1 mm = 1 000 000). Integers keep
geometry exact (pins snap exactly, connectivity uses equality) and make Gerber
output lossless: Gerber's 4.6 mm format is the same number.

Coordinates are Y-up (like Gerber). Angles are degrees, counter-clockwise.

## Placement

A part's transform is: rotate about its origin, mirror about the Y axis when
`mirror` (schematic) or `side: "Bottom"` (PCB), then translate by `pos`.

## Connectivity

Schematic nets come from geometry: wire endpoints that touch other wires,
pins at wire ends or on wire segments, and labels or power ports sharing a
name anywhere in the board's schematic. Unnamed nets are called
`Net-(<first designator>-<pin>)`, which stays stable while editing.

On the PCB each footprint stores `pad_nets` (pad number → net name), filled
by *Update PCB from Schematic*.
