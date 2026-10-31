# The table screen

The map fills the window. Everything else sits in a region, so two panels cannot land on the same spot.

## Regions

- **Top bar.** The current scene and mode, then Play, Map, Campaign, and Rules. View holds zoom, fit, centering, and grid controls. The table menu (•••) holds the party, version, reset, and Leave. A player sees only Play.
- **Left rail.** Opens the tool for the current mode. It starts closed in Play; choosing Map, Campaign, or Rules opens it. Players do not get this dock.
- **Center.** The map. In Play, a Daggerheart table draws Fear, adversary portraits, and the party column over this map. In Design the map is covered by an opaque board: Overview, PC sheet, NPC sheet, and Adversary card. While dice are rolling, the tray sits here.
- **Right rail.** Opens the roll history and dice controls when needed.
- **Bottom strip.** A compact selected-figure card and the character action bar. Figure details expands a scrollable inspector; selecting a figure never automatically expands it. Hidden in Design.

Focus (F) hides the rails and bottom controls to give the scene the full window; F or Escape brings them back. On narrow screens, the drawers overlay the map and only one opens at a time.

A panel can collapse, pop out, and be dragged and resized. Floating panels stay within the current viewport, including after resizing the browser. A dock's inner edge sets its width. Those choices are remembered in this browser under `game-table.layout`. Reset layout, in the table menu, puts them back.

Do not place a new control with `fixed` outside these regions. A menu that must cover the screen, the dice tray, a toast, and the lobby are the exceptions, and they use the layers below.

## Layers

From back to front: the map, the docks, a floating panel, a menu, the dice tray, a toast, the lobby.

## Controls

- Text is at least 12px.
- A button or field is at least 32px tall.
- A panel has one primary button. The others stay quiet.
- Three or more choices in a narrow panel are a vertical list, not a row of cramped labels.

## GM modes

Play runs the table. Edit holds Map and Campaign: the map, scenes, and the cast. Design holds Rules. Entering Design loads the campaign's rule set. Load on a system card opens that set, and the center board reads it. PC, NPC, and adversary each have a sheet, and the adversary sheet is a card. The campaign's system is chosen in Campaign, not in Rules.
