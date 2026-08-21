export interface WikiCategory {
  id: string;
  slug: string;
  name: string;
  description: string;
  runeSymbol: string;
  articleCount: number;
}

export interface WikiStatRow {
  label: string;
  value: string;
  highlight?: boolean;
}

export type WikiRarity = 'Common' | 'Uncommon' | 'Rare' | 'Epic' | 'Legendary' | 'Ancient' | 'Mythic';

export interface WikiInfoBox {
  title: string;
  image?: string;
  type: string;
  stats: WikiStatRow[];
  rarity?: WikiRarity;
}

export interface AbilityDefinition {
  slot: 'Q' | 'W' | 'E' | 'R' | 'Passive';
  name: string;
  baseType: string;
  description: string;
  cooldown: string;
  manaCost: string;
  recommendedEssence?: string;
  recommendedModifiers?: string[];
  recommendedAncientWord?: string;
}

export interface RuneFormulaEntry {
  title: string;
  description: string;
  baseWeapon: string;
  essence: string;
  modifiers: string[];
  ancientWord?: string;
  resultEffect: string;
}

export interface WikiSection {
  id: string;
  title: string;
  content: string[];
  callout?: {
    type: 'info' | 'warning' | 'tip';
    title: string;
    text: string;
  };
  table?: {
    headers: string[];
    rows: string[][];
  };
}

export interface WikiArticle {
  id: string;
  slug: string;
  categorySlug: string;
  categoryName: string;
  title: string;
  subtitle?: string;
  overview: string;
  lastUpdated: string;
  image?: string;
  infobox?: WikiInfoBox;
  abilities?: AbilityDefinition[];
  runeFormulas?: RuneFormulaEntry[];
  sections: WikiSection[];
  relatedSlugs: { title: string; categorySlug: string; slug: string }[];
}
