# The Old Forest Road

![The forest encounter and compact game interface](milestones/forest-sample.png)

[View the cabin cutaway in focus mode](milestones/forest-cabin-cutaway.png) or [inspect the stone stairs](milestones/forest-stairs.png).

Open **Map → Load forest encounter**. This creates a new playable encounter and an editable draft. It keeps the previous scene and its figures in the campaign. Switch to **Play** for the encounter, or keep editing and use **Put on the table** to publish changes.

The encounter covers **36 × 32 cells (1,152 total)**, roughly twice the original 26 × 22 area. The road winds through **72 trees of four species**, four mossy boulders, a furnished log cabin and two raised terraces. Dense groves surround open approaches, with forest-floor flank paths and small clearings that create more choices for movement and line of sight. Both height changes have solid stone treads and landings: the lower path connects tier 0 to 1, and the upper path connects 1 to 2. Four adventurers (Human, Elf, Dwarf, Orc) approach four different bandits. The captain starts inside a seven-by-seven cabin with a bed, writing desk, two barrels and two bookshelves. These are reusable object assets with movement blockers and a clear aisle through the room.

The cabin has full-height log walls and a framed oak door taller than the human figure. The GM can click the door's artwork to open it. During play, selecting a figure inside lowers the room's foreground walls; with no selection, visible occupants determine the cutaway. Edit mode displays the full walls. Characters hidden by fog or GM permissions never gain an outline.

Load a fresh forest encounter after pulling an update to see the revised layout and furnishings. Previously saved scenes keep their existing placements.

The fresh cast includes editable Daggerheart demonstration stats: attacks, damage, thresholds and marked HP. A campaign without a chosen rules system starts using Daggerheart; an existing rules choice is preserved. These are showroom builds, not official adversary stat blocks.

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

Play starts with compact side rails. Open Tools or Dice when needed; click them again to recover map space. Selecting a figure shows a compact summary; expand Details for editing. **F** toggles focus mode while no text field is active. Brass-framed windows rise when clicked, stay within the viewport, and scroll internally on small screens. Menus and the action dial close with Escape. The View menu contains camera controls. See [the interface guide](ui.md) for window and keyboard controls.

All eight characters have four distinct views. Select a figure and use Back, Right, Front or Left in the compact inspector. Movement chooses the closest screen-facing view on the diamond grid. An accepted attack turns both figures toward each other. Facing persists through save and reload. The current artwork approximates diagonals with these four views; no new image assets are needed for this correction.

Try `docs/examples/forest-adventurers-sheet.png` or `docs/examples/forest-bandits-sheet.png` in Import sheet: choose Cast, set cells to **160 × 256**, zero margin/gutter, **128 pixels per world unit**, **1 × 1 footprint**, and foot pivot **80, 245**. The four columns are North, East, South, West; each row becomes one character. Use a new pack ID to preserve the bundled artwork.

The importer expects regular-grid PNG sheets. It does not remove backgrounds or segment arbitrary moodboard collages.

## Select, target and move

Left-click a controllable figure to select it. With a figure selected, click another figure to target it; click that target again to select it if your seat controls it. Clicking an occupied foot cell prioritizes that figure even when another sprite covers the cell. Elsewhere, opaque sprite pixels determine the hit and transparent margins pass through. **Shift-click** cycles overlapping figures and directly selects one your seat controls; another player's figure can be targeted but never taken over.

Click clear ground to move the selected figure, or drag it to a destination. Walking follows a route around blocked objects and walls, through open doors and across connected stairs. An unreachable destination leaves the figure in place. The entire footprint must fit; a large figure cannot squeeze through a narrow gap. GM walking follows the same rules.

Right-click a character for the action dial. It shows selection, targeting, character sheet and available rules actions for that character and the selected actor. Right-click ground for **Move here**, **Center view**, and the GM's explicit **Teleport** option. Teleport bypasses the route; use ordinary movement to walk through the scene. Escape or a click outside closes the dial.

Movement animates locally along the route. Reduced-motion preferences keep movement immediate. Rendering reuses scenery, textures, elevation grids and silhouette masks; stationary lighting updates are capped, and hidden tabs stop drawing. See [performance notes](rendering-performance.md) for the measured checks and limits.

## Character animation

All eight characters have newly drawn frames for idle, melee attack, ranged attack, movement, taking damage and spellcasting in all four directions. During ordinary play, idle uses subtle continuous breathing on a stable drawn pose, avoiding the jump between four different silhouettes and the old long pauses. Zooming does not restart or suspend it. The pack includes 32 animation sheets containing 768 frames. Right-click a controllable character and open **Animations** to preview each state; previews do not roll dice or change game stats. Walking and combat also trigger animations. Spellcasting currently runs through the preview menu. Reduced-motion preferences hold a still pose.

See [asset size measurements](animation-assets.md) for the before/after count and download size.

## Roll and apply damage

In Daggerheart, select the acting figure and click another character once to target it, then use the gold **Attack** button or the action dial. The toolbar follows the selected figure. PCs use Hope d12 + Fear d12 + their equipped weapon's trait; adversaries use d20 + Attack. The server compares the total with the target's Difficulty or Evasion. Hope/Fear criticals still hit regardless of the defense.

For PC attacks, choose **Normal**, **Advantage (+d6)** or **Disadvantage (−d6)** after the GM resolves the net circumstances; the adjacent +/− controls add flat bonuses and penalties. Range, cover and fictional positioning remain GM decisions. These controls do not automatically infer them from scenery. The circumstance die applies to the attack, not its damage roll.

Once the attack dice settle on a hit, **Roll damage** appears directly below the dice. Nothing damages the target until that button is pressed. A miss has no damage button. Standalone damage dice in a sheet or the dice tray are free rolls; they do not apply HP. The command bar uses the linked attack flow instead of its former standalone Damage button.

The damage result shows the target's thresholds and the HP actually marked. Damage below Major marks 1 HP, meeting Major marks 2, and meeting Severe marks 3; zero damage marks none. Adversaries receive these marks immediately. Their public health label and portrait marker update with marked HP, ending at **Defeated** when every HP slot is marked. PCs keep the **Near Death** label at their HP limit, leaving their death move to the table. Critical hits add the maximum of the damage dice, and PCs with available armor can answer the armor prompt. Each successful attack permits one damage roll, including after reconnecting.

## Each player's sight

Select an adventurer, expand **Figure details**, and choose its player under **Owner**. This assigns the linked character sheet, combat actions and all its figures in the campaign's scenes together; changing the sheet's owner does the same. Unlinked figures keep independent owners. Leave vision enabled. A seat sees from its own figures; an unowned figure does not grant everyone sight.

To test without another browser, open **Player view → Create dummy player** in the top bar. Give the dummy a name and choose a player character in the creation form, then choose **Create & see as player**. **Create only** saves the seat and assignment without switching views. Make one dummy per adventurer to compare their sight. The form warns if the chosen character already belongs to someone, has no figure in this scene, or has no active vision.

**Player view** lists real and dummy seats with **See as player** buttons. Preview uses the server's player vision and personal exploration, filters character sheets and secret roll results, and disables table changes. **Return to GM** restores the full view. Preview shows the published scene, so use **Put on the table** after map edits. Your editable draft is kept when switching out of Map.

Dummy seats and their assignments/exploration survive a save and restart. They do not sign in or open another connection. Remove a dummy with **×** beside its name in Player view; its characters and figures stay in the campaign and become unassigned. Its personal fog history is removed; other players keep theirs.

[See the GM's player-preview example](milestones/player-preview.png).

Trees, rocks, cabin walls, closed doors and terrain block sight according to their height. Unseen areas are dark; explored areas outside current sight remain dim, including upright scenery. Another player's exploration does not reveal your map. The GM's explicit reveal brush remains shared. Opening the cabin door or moving around an obstacle updates the view, and personal exploration persists when switching scenes or reloading.
