# Game systems

A campaign picks one system, or none. With none, the table is only a map, tokens, fog, and dice.

The shared engine does not know Pathfinder, Daggerheart, or any other game. It knows things, rolls, die tags, a sheet for each kind of character, and a grid rule. A system fills those in.

## Where a system lives

Each system has up to three folders, all named with the same id:

| Folder | What it holds |
| --- | --- |
| `systems/<id>/system.json` | The definition. The System Designer writes this file. |
| `server/src/systems/<id>/` | Optional Rust for a rule the data cannot express. |
| `client/src/systems/<id>/` | Optional extra controls, such as a dice button. |

Ids are 2 to 32 lowercase letters or digits. `daggerheart` is the worked example. `pathfinder2e`, `shadowdark`, and `nimble` are empty definitions waiting to be filled in.

## The one shared door

Shared code reaches a system in exactly two places:

- `server/src/systems/registry.rs`, function `rules_for`
- `client/src/systems/index.ts`, function `extraFor`

A system with no code uses `Generic`, which accepts every legal roll and adds no outcome text.

## system.json

```json
{
  "id": "daggerheart",
  "name": "Daggerheart",
  "fields": [],
  "dieTags": [],
  "actions": [],
  "sheets": {
    "pc": { "sections": [], "toolbar": { "resources": [], "actions": [] } },
    "npc": { "sections": [], "toolbar": { "resources": [], "actions": [] } },
    "adversary": { "sections": [], "toolbar": { "resources": [], "actions": [] } }
  },
  "grid": null
}
```

A thing is `number`, `text`, `choice`, `pool`, `thresholds`, or `dice`. A pool is drawn as `boxes`, `pips`, or `count`. `max` is a fixed ceiling. `maxFrom` names another number or pool, which is how HP can follow HP max. If that other thing is missing on the character, `max` is used instead. Older files say `track` or `pips`; both load as a pool.

A thresholds thing has steps. Each step has an id, a starting value, and how many marks it adds. Apply damage takes the highest step whose value is at most the amount, and adds those marks to the linked pool. Below the lowest step, nothing is marked. The step's value is stored on the character, so `@major` still works in a formula.

A dice thing is a formula the character can edit. Rolling it uses that formula, or the thing's starting formula when the character's is blank.

An action formula may contain `@fieldId` or `@stepId`. The server substitutes the character's number. A negative number replaces a leading plus, so the roll never contains `+-`.

A die tag has one lowercase letter. `1d12h` means one d12 carrying the tag whose letter is `h`. The tag's colors are `body` and `ink`.

`sheets.pc`, `sheets.npc`, and `sheets.adversary` each have their own sections and toolbar. A thing is on a character only when a section lists it. That is how Armor can be on a PC and absent from an adversary card. A new character is filled from its own sheet only.

An older file with one `sheet` and one `toolbar` is copied onto all three kinds when it is loaded.

`grid.diagonal` is `square` (a diagonal costs one square), `alternate` (5, 10, 5, 10), or `euclidean`. `ranges` are named bands. The first band whose `upTo` is at least the square count is the name shown while dragging.

The Rules screen edits this file in four steps: Things, Rolls, Sheets, and Table. The center shows the sheet you are designing. Save writes the file. Discard throws away unsaved edits.

## Rules the data cannot say

```rust
pub trait Rules {
    fn check_roll(&self, formula: &RollFormula) -> Result<(), String>;
    fn outcome(&self, dice: &[ThrownDie], total: i32) -> Option<RollOutcome>;
}
```

`check_roll` runs after the formula parses and after every tag letter has been checked against the active system. `outcome` can add a label, such as `with Hope`. Daggerheart's implementation is the Hope and Fear pair and that label. It does not touch another system's folder.

## Characters

A character is a PC, an NPC, or an adversary. Each kind has its own sheet. An adversary's sheet is a card. Numbers and text are maps keyed by thing id, and a threshold step's value is stored the same way. Keys the current system does not use are kept, so editing a definition does not wipe a sheet.

The GM sees every character. A player sees only characters they own. The bottom bar shows the resources and rolls from that character's own sheet. A roll is built on the server from the character's real numbers, then the dock's + box, − box, and visibility are applied. Apply on a threshold sends the damage amount; the server marks the linked pool.

## For a coding agent working on one system

Touch only that system's three folders, plus the single registry line on each side if the system is new. Do not edit another system's folder, and do not put a system's rule into `server/src/dice`, `server/src/session`, or `client/src/lib`.
