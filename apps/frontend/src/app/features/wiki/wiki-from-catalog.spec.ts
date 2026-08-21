import { CatalogItem } from '../../shared/models/catalog.model';
import { articlesFromCatalog, mergeWikiContent, wikiCategoryForItem } from './wiki-from-catalog';

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
    ...overrides
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
        { label: 'Rune Stability', value: '86%' }
      ])
    );
  });

  it('does not invent Channeling Staff', () => {
    const articles = articlesFromCatalog([sword(), { ...sword(), id: 'wood', name: 'Wood', category: 'Material', slot: undefined, family: undefined, abilities: undefined, rune_profile: undefined, effects: [] }]);
    expect(articles.map(article => article.slug)).toEqual(['sword', 'wood']);
    expect(articles.some(article => article.slug === 'channeling-staff')).toBe(false);
  });

  it('counts mechanical categories from real items and keeps lore at zero until written', () => {
    const { categories, articles } = mergeWikiContent([
      sword(),
      { ...sword(), id: 'wood', name: 'Wood', category: 'Material', slot: undefined, family: undefined, abilities: undefined, rune_profile: undefined, effects: [] }
    ]);
    expect(categories.find(c => c.slug === 'weapons')?.articleCount).toBe(1);
    expect(categories.find(c => c.slug === 'materials')?.articleCount).toBe(1);
    expect(categories.find(c => c.slug === 'world')?.articleCount).toBe(0);
    expect(categories.some(c => c.slug === 'essences' || c.slug === 'modifiers')).toBe(false);
    expect(articles.some(article => article.categorySlug === 'world')).toBe(false);
  });
});
