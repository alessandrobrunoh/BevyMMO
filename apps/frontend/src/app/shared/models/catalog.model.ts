/** Public catalog DTOs from `GET /v1/public/catalog`. Field names match the gateway (snake_case). */

export interface CatalogAbilityLoadout {
  primary: string[];
  secondary: string[];
  ultimate: string[];
}

export interface CatalogCraftIngredient {
  id: string;
  amount: number;
}

export interface CatalogCraftRecipe {
  channel_seconds: number;
  ingredients: CatalogCraftIngredient[];
}

export interface CatalogRuneProfile {
  capacity: number;
  stability: number;
}

export type CatalogEffect =
  | { kind: 'stat_bonus'; field: string; op: string; value: number }
  | { kind: 'instant_heal'; amount: number };

export interface CatalogItem {
  id: string;
  name: string;
  description: string;
  category: string;
  rarity: string;
  slot?: string;
  family?: string;
  tradable: boolean;
  effects: CatalogEffect[];
  rune_profile?: CatalogRuneProfile;
  abilities?: CatalogAbilityLoadout;
  icon?: string;
  crafting?: CatalogCraftRecipe;
}

export interface Catalog {
  items: CatalogItem[];
}
