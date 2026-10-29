// ------------------------------------------------------------
// Module: chrome
// Purpose: Which floating panels are open. The layout store
//          remembers where each one sits.
// ------------------------------------------------------------

class Chrome {
  settings = $state(false);
  talk = $state(false);
}

export const chrome = new Chrome();
