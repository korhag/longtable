# The table screen

The map fills the window. Everything else sits in a region, so two panels cannot land on the same spot.

## Regions

- **Top bar.** The mode badge and what the app expects, then the campaign, scene, system, and version. Hover the version for the latest note. The view buttons, the party, and Leave sit on the right. A player sees only Play.
- **Left dock.** The tool for the current mode. Players do not get this dock.
- **Center.** The map. In Design the map is covered by an opaque board: Overview, PC sheet, NPC sheet, and Adversary card. While dice are rolling, the tray sits here.
- **Right dock.** The roll history and the dice controls.
- **Bottom strip.** The selected token and the character bar, between the docks. Hidden in Design.

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

Play runs the table. Edit holds Map and Campaign: the map, scenes, and the cast. Design holds Rules. Entering Design loads the campaign's rule set. Load on a system card opens that set, and the center board reads it. PC, NPC, and adversary each have a sheet, and the adversary sheet is a card. The campaign's system is chosen in Campaign, not in Rules.
