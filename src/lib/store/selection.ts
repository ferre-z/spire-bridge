/**
 * Selection store.
 *
 * Cross-route state: which session is currently selected, which
 * filters are active. Lives outside React so the timeline can
 * read it without the sidebar re-rendering (and vice versa).
 *
 * Distinct from `ui.ts` because selection is data-y and ui is
 * chrome-y. Keeping them separate avoids accidental coupling
 * (e.g. collapsing the sidebar clearing your filter).
 */

import { create } from "zustand";
import type { SessionFilter } from "@/lib/normalize/types";

export interface SelectionState {
  selectedSessionId: string | null;
  setSelectedSessionId: (id: string | null) => void;

  filter: SessionFilter;
  setFilter: (patch: Partial<SessionFilter>) => void;
  resetFilter: () => void;
}

export const useSelectionStore = create<SelectionState>((set) => ({
  selectedSessionId: null,
  setSelectedSessionId: (id) => set({ selectedSessionId: id }),

  filter: {},
  setFilter: (patch) =>
    set((s) => ({ filter: { ...s.filter, ...patch } })),
  resetFilter: () => set({ filter: {} }),
}));
