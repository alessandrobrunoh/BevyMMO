import { WikiArticle, WikiCategory } from '../shared/models/wiki.model';

/** Handmade lore categories. articleCount is filled from `LORE_ARTICLES`. */
export const LORE_CATEGORY_DEFS: Omit<WikiCategory, 'articleCount'>[] = [
  {
    id: 'cat-world',
    slug: 'world',
    name: 'World & Lore',
    description: 'The geography, floating landmasses, stone arches, and history of Eivar.',
    runeSymbol: 'ᚹ'
  },
  {
    id: 'cat-combat',
    slug: 'combat',
    name: 'Combat System',
    description: 'Dead reckoning, timing, hitboxes, defensive barriers, and positional tactics.',
    runeSymbol: 'ᛉ'
  },
  {
    id: 'cat-crafting',
    slug: 'crafting',
    name: 'Crafting & Forging',
    description: 'Gathering celestial ores, carving rune slabs, and attuning magical prisms.',
    runeSymbol: 'ᚲ'
  },
  {
    id: 'cat-guilds',
    slug: 'guilds',
    name: 'Guilds & Strongholds',
    description: 'Municipal alliances, fortress upgrades, territorial banners, and warfare.',
    runeSymbol: 'ᚦ'
  }
];

/** Handmade lore articles. Empty until writers add them; mechanical pages come from the catalog API. */
export const LORE_ARTICLES: WikiArticle[] = [];
