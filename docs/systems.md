# Game systems

A campaign picks one system, or none. With none, the table is only a map, tokens, fog, and dice.

The shared engine does not know Pathfinder, Daggerheart, or any other game. It knows fields, actions, die tags, a sheet, a toolbar, and a grid rule. A system fills those in.

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
  "sheet": { "sections": [] },
  "toolbar": { "resources": [], "actions": [] },
  "grid": null
}
```

A field is `number`, `text`, `track`, or `pips`. `max` is a fixed ceiling. `maxFrom` names another number field, which is how HP can follow an HP max. An action formula may contain `@fieldId`. The server substitutes the character's number. A negative number replaces a leading plus, so the roll never contains `+-`.

A die tag has one lowercase letter. `1d12h` means one d12 carrying the tag whose letter is `h`. The tag's colors are `body` and `ink`.

`grid.diagonal` is `square` (a diagonal costs one square), `alternate` (5, 10, 5, 10), or `euclidean`. `ranges` are named bands. The first band whose `upTo` is at least the square count is the name shown while dragging.

The Rules screen in the GM bar edits this file. Save writes it. Revert throws away unsaved edits. The preview uses each field's default.

## Rules the data cannot say

```rust
pub trait Rules {
    fn check_roll(&self, formula: &RollFormula) -> Result<(), String>;
    fn outcome(&self, dice: &[ThrownDie], total: i32) -> Option<RollOutcome>;
}
```

`check_roll` runs after the formula parses and after every tag letter has been checked against the active system. `outcome` can add a label, such as `with Hope`. Daggerheart's implementation is the Hope and Fear pair and that label. It does not touch another system's folder.

## Characters

A character is a PC, an NPC, or an adversary. Numbers and text are maps keyed by field id. Keys the current system does not use are kept, so editing a definition does not wipe a sheet.

The GM sees every character. A player sees only characters they own. The bottom-left toolbar shows the chosen character's resources and action buttons. An action roll is built on the server from the character's real numbers, then the dock's + box, − box, and visibility are applied.

## For a coding agent working on one system

Touch only that system's three folders, plus the single registry line on each side if the system is new. Do not edit another system's folder, and do not put a system's rule into `server/src/dice`, `server/src/session`, or `client/src/lib`.
