import { create } from 'zustand';
import type { EntryId, TextEntry } from '../lib/tauri';

interface SelectedEntryState {
  selectedId: EntryId | null;
  selectedEntry: TextEntry | null;
  setSelectedId: (id: EntryId | null) => void;
  setSelectedEntry: (entry: TextEntry | null) => void;
}

export const useSelectedEntry = create<SelectedEntryState>((set) => ({
  selectedId: null,
  selectedEntry: null,
  setSelectedId: (id) => set({ selectedId: id, selectedEntry: null }),
  setSelectedEntry: (entry) => set({ selectedEntry: entry, selectedId: entry?.id ?? null }),
}));
