import { CatalogItem } from '../../shared/models/catalog.model';
import { WikiArticle, WikiCategory, WikiInfoBox, WikiRarity, WikiStatRow } from '../../shared/models/wiki.model';
import { LORE_ARTICLES, LORE_CATEGORY_DEFS } from '../../data/wiki-lore';

export interface MechanicalCategoryDef {
  id: string;
  slug: string;
  name: string;
  description: string;
  runeSymbol: string;
  itemCategories: string[];
}

export const MECHANICAL_CATEGORY_DEFS: MechanicalCategoryDef[] = [
  {
    id: 'cat-weapons',
    slug: 'weapons',
    name: 'Weapons',
    description: 'Physical archetypes, attack cadence, and the gestures each weapon offers.',
    runeSymbol: 'ᛏ',
    itemCategories: ['Weapon']
  },
  {
    id: 'cat-equipment',
    slug: 'equipment',
    name: 'Equipment',
    description: 'Armor and worn accessories: helms, cuirasses, boots, charms, and offhand pieces.',
    runeSymbol: 'ᛟ',
    itemCategories: ['Armor', 'Accessory']
  },
  {
    id: 'cat-materials',
    slug: 'materials',
    name: 'Materials',
    description: 'Gathered resources that stack in the bag and feed crafting.',
    runeSymbol: 'ᚠ',
    itemCategories: ['Material']
  }
];

export function wikiCategoryForItem(itemCategory: string): { slug: string; name: string } {
  const def = MECHANICAL_CATEGORY_DEFS.find(category =>
    category.itemCategories.includes(itemCategory)
  );
  if (!def) {
    return { slug: 'equipment', name: 'Equipment' };
  }
  return { slug: def.slug, name: def.name };
}

export function itemToArticle(item: CatalogItem): WikiArticle {
  const { slug, name } = wikiCategoryForItem(item.category);
  return {
    id: `item-${item.id}`,
    slug: item.id,
    categorySlug: slug,
    categoryName: name,
    title: item.name,
    overview: item.description,
    lastUpdated: 'Live from game catalog',
    image: item.icon,
    infobox: itemInfobox(item),
    sections: [],
    relatedSlugs: []
  };
}

export function articlesFromCatalog(items: CatalogItem[]): WikiArticle[] {
  const articles = items.map(itemToArticle);
  const byCategory = new Map<string, WikiArticle[]>();
  for (const article of articles) {
    const list = byCategory.get(article.categorySlug) ?? [];
    list.push(article);
    byCategory.set(article.categorySlug, list);
  }
  return articles.map(article => ({
    ...article,
    relatedSlugs: (byCategory.get(article.categorySlug) ?? [])
      .filter(other => other.slug !== article.slug)
      .slice(0, 3)
      .map(other => ({
        title: other.title,
        categorySlug: other.categorySlug,
        slug: other.slug
      }))
  }));
}

export function mergeWikiContent(items: CatalogItem[]): {
  articles: WikiArticle[];
  categories: WikiCategory[];
} {
  const mechanical = articlesFromCatalog(items);
  const articles = [...mechanical, ...LORE_ARTICLES];
  const categories = [
    ...MECHANICAL_CATEGORY_DEFS.map(def => ({
      id: def.id,
      slug: def.slug,
      name: def.name,
      description: def.description,
      runeSymbol: def.runeSymbol,
      articleCount: mechanical.filter(article => article.categorySlug === def.slug).length
    })),
    ...LORE_CATEGORY_DEFS.map(def => ({
      ...def,
      articleCount: LORE_ARTICLES.filter(article => article.categorySlug === def.slug).length
    }))
  ];
  return { articles, categories };
}

function itemInfobox(item: CatalogItem): WikiInfoBox {
  const stats: WikiStatRow[] = [
    { label: 'Category', value: item.category },
    ...(item.slot ? [{ label: 'Slot', value: item.slot }] : []),
    ...(item.family ? [{ label: 'Family', value: item.family }] : []),
    { label: 'Tradable', value: item.tradable ? 'Yes' : 'No' },
    ...item.effects.map(effectToStat),
    ...(item.rune_profile
      ? [
          {
            label: 'Rune Capacity',
            value: String(item.rune_profile.capacity),
            highlight: true
          },
          {
            label: 'Rune Stability',
            value: `${Math.round(item.rune_profile.stability * 100)}%`
          }
        ]
      : [])
  ];
  return {
    title: item.name,
    image: item.icon,
    type: item.slot ?? item.category,
    rarity: toWikiRarity(item.rarity),
    stats
  };
}

function effectToStat(effect: CatalogItem['effects'][number]): WikiStatRow {
  if (effect.kind === 'instant_heal') {
    return { label: 'On use', value: `Heals ${effect.amount}` };
  }
  return {
    label: splitCamel(effect.field),
    value: formatModifier(effect.op, effect.value)
  };
}

function formatModifier(op: string, value: number): string {
  if (op === 'Add') {
    return value >= 0 ? `+${formatNumber(value)}` : formatNumber(value);
  }
  if (op === 'Multiply') {
    return `×${formatNumber(value)}`;
  }
  return formatNumber(value);
}

function formatNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2);
}

function splitCamel(value: string): string {
  return value.replace(/([a-z])([A-Z])/g, '$1 $2');
}

function toWikiRarity(rarity: string): WikiRarity | undefined {
  const allowed: WikiRarity[] = [
    'Common',
    'Uncommon',
    'Rare',
    'Epic',
    'Legendary',
    'Ancient',
    'Mythic'
  ];
  return allowed.find(entry => entry === rarity);
}
