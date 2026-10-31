# The Old Forest Road

![The forest encounter and compact game interface](milestones/forest-sample.png)

[View the cabin cutaway in focus mode](milestones/forest-cabin-cutaway.png).

Open **Edit → Map → Load forest encounter**. This creates a new playable encounter and an editable draft. It keeps the previous scene and its figures in the campaign. Switch to **Play** for the encounter, or keep editing and use **Put on the table** to publish changes.

The road winds past an enclosed timber cabin, four species of trees, a mossy boulder, and two raised terraces. Both height changes have stairs: the lower path connects tier 0 to 1, and the upper path connects 1 to 2. Four adventurers (Human, Elf, Dwarf, Orc) approach four different bandits. The cabin captain starts inside. The GM can click its door to open it. During play, a visible occupant lowers the room's foreground walls; edit mode displays the full walls. Characters hidden by fog or GM permissions never gain an outline.

## Paint a woodland

Select **Ancient woodland** under Biomes. Brush paints a continuous stroke and places spaced trees; Bucket fills the connected ground region; Whole map fills the draft. Variant selection is deterministic, and repeated biome painting respects existing objects. For an unobstructed path, choose **Old cobbled road** under Ground and paint with a wide brush. Forest and road edges blend organically; cabin boards stay crisp.

Walls support freehand, straight line, and rectangular room gestures. Drag a room perimeter, then choose a door and click the edge where it belongs. Height painting offers Ground, Step and Rise. Add stairs between adjacent cells exactly one tier apart.

## Add a sheet to the library

Use **Map → Import sheet**. Choose the category and cell dimensions, preview the cuts, then add it. Props use one asset per cell; cast uses four columns per row (north, east, south, west); wall rows use two axes; door rows use four axis/state frames. This is a regular-grid importer, so keep titles, borders and decorative guide labels out of your production sheets.

Try `docs/examples/forest-floor-sheet.png`: four 128×128 cells, zero margin and gutter, Ground category, grouped as variants. Use a new pack ID. The importer creates and validates the pack, then adds it to the same asset library used by painting and placement. The dialog includes a downloadable guide; [the full asset contract](assets.md) covers collision, pivots, scale and layers.

## Light the encounter

Open **Lighting** in Play to switch between Day, Dusk and Night or adjust ambient brightness. The sample includes warm flickering torchlight in the cabin and cold blue magic on the terrace. **Map → Lights** adds Torch, Magic or Steady sources with adjustable color, reach and intensity. See [the lighting guide](lighting.md).

Open **Sounds** to try female and male combat voices: Attack, Exertion, Take damage and Defeat. These play locally, with voice selection, volume, mute and Stop. See [the combat sound guide](combat-sounds.md) for playback and asset credits.

## Workspace controls

Play starts with compact side rails. Open Tools or Dice when needed; click them again to recover map space. Selecting a figure shows a compact summary; expand Details for editing. **F** toggles focus mode while no text field is active. Sheet windows stay within the viewport and scroll internally on small screens. The View menu contains camera and layout controls.

All eight characters have four distinct views. Select a figure and use North, East, South or West in the compact inspector; moving a figure also updates its facing. Facing persists through save and reload.

Try `docs/examples/forest-adventurers-sheet.png` or `docs/examples/forest-bandits-sheet.png` in Import sheet: choose Cast, set cells to **160 × 256**, zero margin/gutter, **128 pixels per world unit**, **1 × 1 footprint**, and foot pivot **80, 245**. The four columns are North, East, South, West; each row becomes one character. Use a new pack ID to preserve the bundled artwork.

The importer expects regular-grid PNG sheets. It does not remove backgrounds or segment arbitrary moodboard collages.
