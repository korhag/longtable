// ------------------------------------------------------------
// Module: creation
// Purpose: Which Daggerheart maker is open. The shared screen
//          only asks for it through systems/index.ts.
// ------------------------------------------------------------

class CreationUi {
  kind = $state<'pc' | 'adversary' | null>(null);

  open(next: 'pc' | 'adversary') {
    this.kind = next;
  }

  close() {
    this.kind = null;
  }
}

export const creation = new CreationUi();
