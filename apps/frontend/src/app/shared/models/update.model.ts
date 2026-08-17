export type UpdateCategory = 'NEW' | 'CHANGED' | 'BALANCE' | 'FIXED' | 'TECHNICAL';

export interface UpdateEntry {
  category: UpdateCategory;
  items: string[];
}

export interface GameUpdate {
  id: string;
  version: string;
  title: string;
  type: 'Development' | 'Patch Notes';
  date: string;
  status: 'Live Alpha' | 'Upcoming' | 'Archive';
  summary: string;
  highlights: string[];
  sections: UpdateEntry[];
}
