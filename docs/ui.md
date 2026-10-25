# The table screen

The map fills the window. Everything else sits in a region, so two panels cannot land on the same spot.

## Regions

- **Top bar.** Campaign, scene, the GM's mode, the view buttons, the party, Leave.
- **Left dock.** The tool for the current mode. Players do not get this dock.
- **Center.** The map. While Rules is open, the sheet preview sits here. While dice are rolling, the tray sits here.
- **Right dock.** The roll history and the dice controls.
- **Bottom strip.** The selected token and the character bar, between the docks.

A panel can collapse, pop out, and be dragged and resized. A dock's inner edge sets its width. Those choices are remembered in this browser under `game-table.layout`. Reset, in the top bar, puts them back.

Do not place a new control with `fixed` outside these regions. A menu that must cover the screen, the dice tray, a toast, and the lobby are the exceptions, and they use the layers below.

## Layers

From back to front: the map, the docks, a floating panel, a menu, the dice tray, a toast, the lobby.

## Controls

- Text is at least 12px.
- A button or field is at least 32px tall.
- A panel has one primary button. The others stay quiet.
- Three or more choices in a narrow panel are a vertical list, not a row of cramped labels.

## GM modes

Play runs the table. Map edits the map and can import or blank one. Campaign creates campaigns on a system, then scenes and the cast. Rules edits a system. The campaign's system is chosen in Campaign, not in Rules.
