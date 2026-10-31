import type { Character } from '../net/bindings/Character';
import type { SystemDef } from '../net/bindings/SystemDef';
import type { TokenView } from '../net/bindings/TokenView';

export interface ContextAction { id: string; label: string; icon: string; detail?: string }
export function canControl(token: TokenView | null, gm: boolean, seat: string | null, readOnly = false) {
  return !!token && !readOnly && (gm || (!!seat && token.ownerId === seat));
}
export function contextActions(options: {
  clicked: TokenView | null; selected: TokenView | null; actor: Character | null;
  target: TokenView | null; system: SystemDef | null; gm: boolean; seat: string | null; readOnly: boolean;
  hasPoint: boolean; hasClickedSheet: boolean;
}): ContextAction[] {
  const { clicked, selected, actor, target, system, gm, seat, readOnly } = options;
  const actions: ContextAction[] = [];
  const controlsActor = canControl(selected, gm, seat, readOnly) && !!actor && (gm || actor.ownerId === seat);
  if (!clicked) {
    if (canControl(selected, gm, seat, readOnly) && options.hasPoint) actions.push({ id: 'move', label: 'Move here', icon: '↗' });
    if (gm && !readOnly && selected && options.hasPoint) actions.push({ id: 'teleport', label: 'Teleport', icon: '✦', detail: 'GM reposition: ignores obstacles' });
    if (options.hasPoint) actions.push({ id: 'center-point', label: 'Center view', icon: '⌖' });
    return actions;
  }
  if (controlsActor && system?.id === 'daggerheart' && target?.characterId && target.characterId !== actor?.id) {
    actions.push({ id: 'attack', label: 'Attack', icon: '⚔', detail: `${actor!.name} → ${target.name}` });
  }
  if (clicked.id !== selected?.id) {
    if (canControl(clicked, gm, seat, readOnly)) actions.push({ id: 'select', label: 'Select', icon: '◉' });
    actions.push({ id: 'target', label: 'Target', icon: '◎' });
  }
  if (options.hasClickedSheet) actions.push({ id: 'sheet', label: 'Character', icon: '▤' });
  actions.push({ id: 'center', label: 'Center view', icon: '⌖' });
  if (gm && !readOnly) actions.push({ id: 'hide', label: clicked.hidden ? 'Show' : 'Hide', icon: clicked.hidden ? '◌' : '◑' });
  if (controlsActor && actor && system && clicked.id === selected?.id) {
    for (const id of system.sheets[actor.kind].toolbar.actions) {
      const action = system.actions.find(action => action.id === id);
      if (!action || id === 'attackroll') continue;
      actions.push({ id: `rule:${id}`, label: action.name, icon: '◇', detail: `${actor.name} · ${action.formula}` });
    }
  }
  return actions;
}
