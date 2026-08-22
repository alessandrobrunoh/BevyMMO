import { CatalogCraftRecipe, CatalogItem } from '../../shared/models/catalog.model';
import {
  WikiArticle,
  WikiCategory,
  WikiInfoBox,
  WikiRarity,
  WikiSection,
  WikiStatRow
} from '../../shared/models/wiki.model';
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

const CRAFTING_CATEGORY = {
  slug: 'crafting',
  name: 'Crafting & Forging'
} as const;

export function wikiCategoryForItem(itemCategory: string): { slug: string; name: string } {
  const def = MECHANICAL_CATEGORY_DEFS.find(category =>
    category.itemCategories.includes(itemCategory)
  );
  if (!def) {
    return { slug: 'equipment', name: 'Equipment' };
  }
  return { slug: def.slug, name: def.name };
}

export function formatChannelSeconds(seconds: number): string {
  return `${seconds.toFixed(1)}s`;
}

export function articleSearchText(article: WikiArticle): string {
  const parts = [article.title, article.overview, article.categoryName, article.subtitle ?? ''];
  for (const section of article.sections) {
    parts.push(section.title, ...section.content);
    if (section.callout) {
      parts.push(section.callout.title, section.callout.text);
    }
    if (section.table) {
      parts.push(...section.table.headers, ...section.table.rows.flat());
    }
  }
  for (const related of article.relatedSlugs) {
    parts.push(related.title);
  }
  return parts.join(' ').toLowerCase();
}

export function itemToArticle(item: CatalogItem, catalog: CatalogItem[] = [item]): WikiArticle {
  const { slug, name } = wikiCategoryForItem(item.category);
  const crafting = craftingSection(item, catalog);
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
    sections: crafting ? [crafting] : [],
    relatedSlugs: []
  };
}

export function articlesFromCatalog(items: CatalogItem[]): WikiArticle[] {
  const articles = items.map(item => itemToArticle(item, items));
  applySameCategoryRelated(articles);
  applyCraftRelations(articles, items);
  return articles;
}

export function mergeWikiContent(items: CatalogItem[]): {
  articles: WikiArticle[];
  categories: WikiCategory[];
} {
  const mechanical = articlesFromCatalog(items);
  const recipes = recipesIndexArticle(items, mechanical);
  const articles = [...mechanical, ...recipes, ...LORE_ARTICLES];
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
      articleCount: articles.filter(article => article.categorySlug === def.slug).length
    }))
  ];
  return { articles, categories };
}

function itemName(catalog: CatalogItem[], id: string): string {
  return catalog.find(item => item.id === id)?.name ?? id;
}

function crafterCallout(category: string): NonNullable<WikiSection['callout']> {
  if (category === 'Weapon') {
    return {
      type: 'tip',
      title: 'Weapon Crafter',
      text: 'Take these materials to a Fabbro (Weapon Crafter).'
    };
  }
  return {
    type: 'tip',
    title: 'Crafter',
    text: `Take these materials to a ${category} crafter.`
  };
}

function craftingSection(item: CatalogItem, catalog: CatalogItem[]): WikiSection | undefined {
  const recipe = item.crafting;
  if (!recipe) {
    return undefined;
  }
  return {
    id: 'crafting',
    title: 'Crafting',
    content: [],
    callout: crafterCallout(item.category),
    table: {
      headers: ['Material', 'Amount'],
      rows: [
        ...recipe.ingredients.map(ingredient => [
          itemName(catalog, ingredient.id),
          String(ingredient.amount)
        ]),
        ['Channel', formatChannelSeconds(recipe.channel_seconds)]
      ]
    }
  };
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
      : []),
    ...(item.crafting
      ? [
          { label: 'Craftable', value: 'Yes' },
          { label: 'Channel', value: formatChannelSeconds(item.crafting.channel_seconds) }
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

function applySameCategoryRelated(articles: WikiArticle[]): void {
  const byCategory = new Map<string, WikiArticle[]>();
  for (const article of articles) {
    const list = byCategory.get(article.categorySlug) ?? [];
    list.push(article);
    byCategory.set(article.categorySlug, list);
  }
  for (const article of articles) {
    article.relatedSlugs = (byCategory.get(article.categorySlug) ?? [])
      .filter(other => other.slug !== article.slug)
      .slice(0, 3)
      .map(other => ({
        title: other.title,
        categorySlug: other.categorySlug,
        slug: other.slug
      }));
  }
}

function applyCraftRelations(articles: WikiArticle[], items: CatalogItem[]): void {
  const bySlug = new Map(articles.map(article => [article.slug, article]));
  const usedIn = new Map<string, CatalogItem[]>();

  for (const item of items) {
    if (!item.crafting) {
      continue;
    }
    for (const ingredient of item.crafting.ingredients) {
      const list = usedIn.get(ingredient.id) ?? [];
      list.push(item);
      usedIn.set(ingredient.id, list);
    }
  }

  for (const [ingredientId, outputs] of usedIn) {
    const article = bySlug.get(ingredientId);
    if (!article) {
      continue;
    }
    article.sections.push({
      id: 'used-in-forging',
      title: 'Used in forging',
      content: [],
      table: {
        headers: ['Item', 'Amount per craft', 'Channel'],
        rows: outputs.flatMap(output => {
          const amount =
            output.crafting?.ingredients.find(ingredient => ingredient.id === ingredientId)
              ?.amount ?? 0;
          return [
            [output.name, String(amount), formatChannelSeconds(output.crafting?.channel_seconds ?? 0)]
          ];
        })
      }
    });
  }

  for (const item of items) {
    const article = bySlug.get(item.id);
    if (!article) {
      continue;
    }
    const extra: WikiArticle['relatedSlugs'] = [];
    if (item.crafting) {
      for (const ingredient of item.crafting.ingredients) {
        const target = bySlug.get(ingredient.id);
        if (target) {
          extra.push({
            title: target.title,
            categorySlug: target.categorySlug,
            slug: target.slug
          });
        }
      }
    }
    for (const output of usedIn.get(item.id) ?? []) {
      const target = bySlug.get(output.id);
      if (target) {
        extra.push({
          title: target.title,
          categorySlug: target.categorySlug,
          slug: target.slug
        });
      }
    }
    article.relatedSlugs = mergeRelated(article.relatedSlugs, extra);
  }
}

function mergeRelated(
  current: WikiArticle['relatedSlugs'],
  extra: WikiArticle['relatedSlugs']
): WikiArticle['relatedSlugs'] {
  const seen = new Set(current.map(related => `${related.categorySlug}/${related.slug}`));
  const merged = [...current];
  for (const related of extra) {
    const key = `${related.categorySlug}/${related.slug}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    merged.push(related);
  }
  return merged;
}

function recipesIndexArticle(items: CatalogItem[], mechanical: WikiArticle[]): WikiArticle[] {
  const craftable = items.filter(
    (item): item is CatalogItem & { crafting: CatalogCraftRecipe } => item.crafting != null
  );
  if (craftable.length === 0) {
    return [];
  }
  const bySlug = new Map(mechanical.map(article => [article.slug, article]));
  return [
    {
      id: 'crafting-recipes',
      slug: 'recipes',
      categorySlug: CRAFTING_CATEGORY.slug,
      categoryName: CRAFTING_CATEGORY.name,
      title: 'Recipes',
      overview: 'Every item that declares a crafting recipe in the game catalog.',
      lastUpdated: 'Live from game catalog',
      sections: [
        {
          id: 'recipe-index',
          title: 'Craftable items',
          content: [],
          table: {
            headers: ['Item', 'Category', 'Materials', 'Channel'],
            rows: craftable.map(item => [
              item.name,
              wikiCategoryForItem(item.category).name,
              item.crafting.ingredients
                .map(ingredient => `${ingredient.amount} ${itemName(items, ingredient.id)}`)
                .join(', '),
              formatChannelSeconds(item.crafting.channel_seconds)
            ])
          }
        }
      ],
      relatedSlugs: craftable.map(item => {
        const article = bySlug.get(item.id);
        return {
          title: item.name,
          categorySlug: article?.categorySlug ?? wikiCategoryForItem(item.category).slug,
          slug: item.id
        };
      })
    }
  ];
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
