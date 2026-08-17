export type StoreCategory = 'Featured' | 'Cosmetics' | 'Supporter Packs' | 'Account' | 'Other';
export type ItemRarity = 'Common' | 'Rare' | 'Ancient' | 'Mythic';

export interface StoreItem {
  id: string;
  name: string;
  category: StoreCategory;
  description: string;
  detailedLore: string;
  image: string;
  mockPrice: string;
  rarity: ItemRarity;
  featured?: boolean;
  tags: string[];
  includes?: string[];
}
