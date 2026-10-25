// ------------------------------------------------------------
// Module: sound preference
// Purpose: The mute switch. The dice read the same browser key.
// ------------------------------------------------------------

const STORAGE_KEY = 'game-table.sound';

class SoundPref {
  muted = $state(localStorage.getItem(STORAGE_KEY) === 'off');

  set(muted: boolean) {
    this.muted = muted;
    localStorage.setItem(STORAGE_KEY, muted ? 'off' : 'on');
  }
}

export const sound = new SoundPref();
