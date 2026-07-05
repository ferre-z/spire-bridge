/**
 * UI state store.
 *
 * Holds ephemeral UI preferences: which sidebar item is active,
 * whether the right rail is collapsed, etc. Anything that should
 * survive a route change but NOT a window close lives here.
 *
 * Persistence: we deliberately do NOT persist this to disk in v1.
 * The first time the user opens the app they get the default
 * layout, which is what 95% of users want anyway. Phase 2 will
 * add a per-user settings file once we know which prefs actually
 * vary.
 */

import { create } from "zustand";

export interface UiState {
  /** Index of the active sidebar item (1-indexed to match ⌘1-9). */
  activeSidebarIndex: number;
  setActiveSidebarIndex: (i: number) => void;

  /** Right-rail collapsed state. Reserved for Phase 1.5; default open. */
  rightRailCollapsed: boolean;
  toggleRightRail: () => void;

  /** Command palette visibility (Task 9 wires this to ⌘K). */
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  activeSidebarIndex: 1,
  setActiveSidebarIndex: (i) => set({ activeSidebarIndex: i }),

  rightRailCollapsed: false,
  toggleRightRail: () =>
    set((s) => ({ rightRailCollapsed: !s.rightRailCollapsed })),

  paletteOpen: false,
  setPaletteOpen: (open) => set({ paletteOpen: open }),
}));
