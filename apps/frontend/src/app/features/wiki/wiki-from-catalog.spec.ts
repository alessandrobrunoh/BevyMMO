import { CatalogItem } from '../../shared/models/catalog.model';
import {
  articleSearchText,
  articlesFromCatalog,
  mergeWikiContent,
  wikiCategoryForItem
} from './wiki-from-catalog';

function sword(overrides: Partial<CatalogItem> = {}): CatalogItem {
  return {
    id: 'sword',
    name: 'Spada',
    description: 'A balanced sword.',
    category: 'Weapon',
    rarity: 'Rare',
    slot: 'Weapon',
    family: 'sword',
    tradable: true,
    effects: [{ kind: 'stat_bonus', field: 'AttackPower', op: 'Add', value: 70 }],
    rune_profile: { capacity: 11, stability: 0.86 },
    abilities: { primary: ['cleave'], secondary: ['lunge'], ultimate: ['blade_storm'] },
    crafting: {
      channel_seconds: 3,
      ingredients: [
        { id: 'wood', amount: 2 },
        { id: 'copper', amount: 4 }
      ]
    },
    ...overrides
  };
}

function wood(): CatalogItem {
  return {
    id: 'wood',
    name: 'Wood',
    description: 'A piece of oak.',
    category: 'Material',
    rarity: 'Common',
    tradable: true,
    effects: []
  };
}

function copper(): CatalogItem {
  return {
    id: 'copper',
    name: 'Copper',
    description: 'A lump of copper ore.',
    category: 'Material',
    rarity: 'Common',
    tradable: true,
    effects: []
  };
}

function helm(): CatalogItem {
  return {
    id: 'simple_helm',
    name: 'Simple Helm',
    description: 'A plain iron cap.',
    category: 'Armor',
    rarity: 'Common',
    slot: 'Helmet',
    tradable: true,
    effects: []
  };
}

describe('wiki-from-catalog', () => {
  it('maps weapon items onto /wiki/weapons', () => {
    expect(wikiCategoryForItem('Weapon')).toEqual({ slug: 'weapons', name: 'Weapons' });
  });

  it('groups armor and accessories as equipment', () => {
    expect(wikiCategoryForItem('Armor').slug).toBe('equipment');
    expect(wikiCategoryForItem('Accessory').slug).toBe('equipment');
  });

  it('builds a sword article from the catalog DTO', () => {
    const [article] = articlesFromCatalog([sword()]);
    expect(article.slug).toBe('sword');
    expect(article.title).toBe('Spada');
    expect(article.categorySlug).toBe('weapons');
    expect(article.overview).toBe('A balanced sword.');
    expect(article.infobox?.rarity).toBe('Rare');
    expect(article.infobox?.stats).toEqual(
      expect.arrayContaining([
        { label: 'Slot', value: 'Weapon' },
        { label: 'Attack Power', value: '+70' },
        { label: 'Rune Capacity', value: '11', highlight: true },
        { label: 'Rune Stability', value: '86%' },
        { label: 'Craftable', value: 'Yes' },
        { label: 'Channel', value: '3.0s' }
      ])
    );
  });

  it('adds a crafting section and related ingredient chips to craftable weapons', () => {
    const articles = articlesFromCatalog([sword(), wood(), copper()]);
    const spada = articles.find(article => article.slug === 'sword');
    expect(spada?.sections.find(section => section.id === 'crafting')?.table?.rows).toEqual([
      ['Wood', '2'],
      ['Copper', '4'],
      ['Channel', '3.0s']
    ]);
    expect(spada?.relatedSlugs).toEqual(
      expect.arrayContaining([
        { title: 'Wood', categorySlug: 'materials', slug: 'wood' },
        { title: 'Copper', categorySlug: 'materials', slug: 'copper' }
      ])
    );
  });

  it('does not add a crafting section to unique equipment', () => {
    const [article] = articlesFromCatalog([helm()]);
    expect(article.sections.find(section => section.id === 'crafting')).toBeUndefined();
    expect(article.infobox?.stats.some(stat => stat.label === 'Craftable')).toBe(false);
  });

  it('lists craft outputs on ingredient articles', () => {
    const articles = articlesFromCatalog([sword(), wood(), copper()]);
    const copperArticle = articles.find(article => article.slug === 'copper');
    expect(copperArticle?.sections.find(section => section.id === 'used-in-forging')?.table?.rows).toEqual([
      ['Spada', '4', '3.0s']
    ]);
    expect(copperArticle?.relatedSlugs).toEqual(
      expect.arrayContaining([{ title: 'Spada', categorySlug: 'weapons', slug: 'sword' }])
    );
  });

  it('does not invent Channeling Staff', () => {
    const articles = articlesFromCatalog([
      sword(),
      { ...sword(), id: 'wood', name: 'Wood', category: 'Material', slot: undefined, family: undefined, abilities: undefined, rune_profile: undefined, effects: [], crafting: undefined }
    ]);
    expect(articles.map(article => article.slug)).toEqual(['sword', 'wood']);
    expect(articles.some(article => article.slug === 'channeling-staff')).toBe(false);
  });

  it('counts mechanical categories from real items and fills crafting from generated recipes', () => {
    const { categories, articles } = mergeWikiContent([sword(), wood(), copper(), helm()]);
    expect(categories.find(c => c.slug === 'weapons')?.articleCount).toBe(1);
    expect(categories.find(c => c.slug === 'materials')?.articleCount).toBe(2);
    expect(categories.find(c => c.slug === 'equipment')?.articleCount).toBe(1);
    expect(categories.find(c => c.slug === 'crafting')?.articleCount).toBe(1);
    expect(categories.find(c => c.slug === 'world')?.articleCount).toBe(0);
    expect(categories.some(c => c.slug === 'essences' || c.slug === 'modifiers')).toBe(false);
    expect(articles.some(article => article.categorySlug === 'world')).toBe(false);
    const recipes = articles.find(article => article.slug === 'recipes');
    expect(recipes?.categorySlug).toBe('crafting');
    expect(recipes?.sections[0].table?.rows).toEqual([
      ['Spada', 'Weapons', '2 Wood, 4 Copper', '3.0s']
    ]);
  });

  it('indexes crafting copy so search can find Spada from copper', () => {
    const articles = articlesFromCatalog([sword(), wood(), copper()]);
    const spada = articles.find(article => article.slug === 'sword');
    expect(articleSearchText(spada!)).toContain('copper');
    expect(articleSearchText(spada!)).toContain('wood');
  });
});
