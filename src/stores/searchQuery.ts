import { create } from 'zustand';

interface SearchQueryState {
  query: string;
  setQuery: (q: string) => void;
}

export const useSearchQuery = create<SearchQueryState>((set) => ({
  query: '',
  setQuery: (query) => set({ query }),
}));
