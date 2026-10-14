# User guide

EasyPRO is laid out like EasyEDA Pro: menus in the blue bar, tools in the
toolbar below it, the project tree and component library on the left, open
documents as tabs in the middle, properties on the right and the log and rule
check results at the bottom. Everything is also reachable from the command
palette (**Ctrl+K**), which searches commands, library parts and the
designators of placed parts.

## Projects and saving

- **File → New Project** starts a project with one schematic sheet and a
  50 × 40 mm two-layer board.
- Every change is saved to this browser within a second. The project list on
  the left (*In this browser*) and the start page show these projects.
- **Ctrl+S** also writes a `.epro.json` file. In Chrome and Edge the first
  save asks where to put it and later saves overwrite that file; other
  browsers download a copy each time.
- Each Ctrl+S keeps a version in **File → Version History**, where you can
  also name and save versions by hand and restore any of them (restoring is
  undoable).
- Drop a `.epro.json` file on the window to open it. Drop `.kicad_mod` or
  `.kicad_sym` files to import KiCad footprints or symbols.

## Schematic

| Action | How |
|---|---|
| Place a part | Search the **Common Library** (Shift+F), double-click a result or press **Place**. While placing: **Space**/**R** rotates, **X** mirrors, **Esc** stops. |
| Wire | **W**, click to start, click to add corners, double-click or **Esc** to finish. **/** flips the bend, **Backspace** removes the last corner. Ending on a pin or wire finishes automatically. |
| Net label | **N** and click a wire or pin. Labels with the same name are connected on every sheet. Double-click a label to rename it. |
| Power / ground | **P** / **G**. Rename in Properties (VCC, +3V3, GND…). |
| No-connect | **Shift+X** on a pin that is unused on purpose. |
| Move | Drag. Wires attached to moved pins follow and stay orthogonal. Arrow keys nudge by one grid step (Shift: ten). |
| Annotate | **Design → Annotate** numbers parts top-to-bottom, left-to-right. New and pasted parts are numbered as they are placed. |
| Check | **Design → Electrical Rule Check**. Click an issue to jump to it. |

Junction dots appear automatically where three or more connections meet.

## PCB

1. **Design → Update PCB from Schematic** (**Ctrl+U**) adds footprints to the
   right of the board, updates values and nets, and removes footprints whose
   symbol was deleted. Placement you did is kept.
2. Drag parts onto the board. **R** rotates, **F** flips to the other side.
   Hold **Alt** to move off-grid.
3. **W** routes: click a pad, click to add corners. The preview turns red when
   the route would break clearance; the router tries the other 45° bend
   automatically. **V** drops a via and continues on the other layer, **/**
   flips the bend, **Esc** finishes, **Ctrl+click** forces a blocked segment.
   The toolbar width picker sets the width for new tracks (and changes
   selected ones).
4. **E** draws a copper area: click the corners and double-click; you are
   asked for its net (GND by default). Areas refill automatically.
5. **T** / **B** switch the active layer; the layer panel on the right toggles
   visibility. Double-click a track to highlight its net; **Esc** clears it.
6. **Design → Design Rule Check** (**Ctrl+Alt+D**). While you edit, a live
   DRC marks rule errors with red rings. Track ends that touch nothing are
   reported as warnings; **Route → Remove Dangling Tracks** deletes them,
   including whole abandoned stubs. The download button in the Issues tab
   saves the results as a text report.
7. **Design → Board Setup** sets layer count, thickness, mask colour, the
   board size with rounded corners, and adds M3 mounting holes. Its *Via
   stitching* row fills free board area with vias on one net (usually GND)
   at the pitch you pick, keeping clear of pads, tracks and the edge.
8. **Design → Board Info** shows the board size and area, part, pad, hole and
   via counts, and the routed length of every net. Selecting a track also
   shows its whole net's length in Properties.

**Edit → Select Similar Parts** (also on the right-click menu) selects every
part with the same device and value as the selection, in schematic or PCB.

Zoom in to see pad numbers and net names on the pads.

## 3D

**View → 3D View** (**Alt+3**). Drag to orbit, right-drag to pan, scroll to
zoom. Component bodies come from each device's 3D settings and their leads
are generated from the footprint pads. **PNG** saves a picture.

## Making parts

- **Tools → IC Symbol Wizard**: paste a pin table (number, name, type) and
  pick a footprint; it creates the symbol and device.
- **Tools → New Device** creates a device with a starter symbol and footprint
  and opens its page, where you choose the symbol, footprint, pin-to-pad
  mapping, 3D body and attributes (manufacturer, MPN, LCSC part number…).
- In the **footprint editor**, **P** adds SMD pads and **H** through-hole pads;
  new pads copy the previous pad's size and number upwards. *Generate Pads*
  in Properties builds single or dual rows (headers, SOIC, DIP) in one step.
- In the **symbol editor**, **P** adds pins (Space rotates them before
  placing). Pin type matters for the ERC.

## Fabrication

**Export → Fabrication Package** (**Ctrl+Alt+G**) builds a zip with Gerber
X2 layers, a Gerber job file, Excellon drill files, BOM and pick-and-place
CSV, named the way the chosen manufacturer expects. The dialog shows the DRC
status first. The package also holds an IPC-D-356 netlist (`.ipc`) that
fabs use to electrically test bare boards; **Export → IPC-D-356 Test
Netlist** saves it on its own. **Design → Bill of Materials** lists grouped parts and lets
you fill in MPN and LCSC numbers for assembly.

## Shortcuts

See [SHORTCUTS.md](SHORTCUTS.md), or press **?** in the app. Every shortcut
can be changed in **Settings → Keyboard Shortcuts**.
