import { StoreItem } from '../../shared/models/store.model';

export const MOCK_STORE_ITEMS: StoreItem[] = [
  {
    id: 'store-item-1',
    name: 'Wayfarer Cloak',
    category: 'Cosmetics',
    description: 'Rugged adventurer travel cloak with carved stone raven clasps and glowing cyan runic embroidery.',
    detailedLore: 'Worn by the earliest cartographers who charted the floating archipelagos of Eivar. The hem is enchanted with subtle luminescent thread that gently glows in dim cavernous environments.',
    image: 'assets/images/wayfarer-cloak.jpg',
    mockPrice: '1,200 Astral Marks',
    rarity: 'Rare',
    featured: true,
    tags: ['Cloak', 'Armor Skin', 'Runic Glow'],
    includes: ['Wayfarer Hooded Cloak Skin', 'Raven Stone Clasp Variant', 'Cosmetic Dye Slot']
  },
  {
    id: 'store-item-2',
    name: 'Founder War Banner',
    category: 'Supporter Packs',
    description: 'A tall heraldic war banner with silver snowflake sigil, glowing runic borders, and gold carved pedestal.',
    detailedLore: 'Commissioned to commemorate the pioneering vanguard who first set foot in Eivar during the Alpha era. Can be planted in guild halls or territorial encampments.',
    image: 'assets/images/founder-banner.jpg',
    mockPrice: '2,500 Astral Marks',
    rarity: 'Ancient',
    featured: true,
    tags: ['Supporter', 'Banner', 'Guild Decoration'],
    includes: ['Founder War Banner Monument', 'Exclusive Title: "The First Vanguard"', 'Alpha Supporter Discord Role badge']
  },
  {
    id: 'store-item-3',
    name: 'Runic Camp Decoration',
    category: 'Cosmetics',
    description: 'Cozy wilderness campsite with standing ancient rune monoliths and crackling stone hearth.',
    detailedLore: 'Rest in the wild with the mystical tranquility of ancient stone wards. Replaces the default resting camp visual with glowing runic obelisks and starry ambient particles.',
    image: 'assets/images/runic-camp.jpg',
    mockPrice: '1,800 Astral Marks',
    rarity: 'Ancient',
    featured: false,
    tags: ['Camp', 'Rest Skin', 'Housing'],
    includes: ['Runic Monolith Camp Skin', 'Stone Hearth Campfire', 'Ambient Rune Attunement Audio']
  },
  {
    id: 'store-item-4',
    name: 'Citadel Gatekeeper Armor Pack',
    category: 'Supporter Packs',
    description: 'Burnished plate armor inspired by the city watch of the capital fortress with glowing cyan conduit lines.',
    detailedLore: 'Forged within the deep masonry of the high citadel, this armor represents the steadfast defenders who hold the great stone bridges against outside incursions.',
    image: 'assets/images/ancient-citadel.jpg',
    mockPrice: '3,000 Astral Marks',
    rarity: 'Mythic',
    featured: true,
    tags: ['Armor', 'Supporter', 'Full Set'],
    includes: ['Gatekeeper Helm', 'Gatekeeper Cuirass & Pauldrons', 'Gatekeeper Greatshield Cosmetic']
  },
  {
    id: 'store-item-5',
    name: 'Explorer Codex Portrait Frame',
    category: 'Account',
    description: 'Carved stone UI portrait frame with animated cyan corner runes and gold inlay for your player card.',
    detailedLore: 'An account-wide cosmetic border that frames your player avatar in the party screen, guild roster, and leaderboard displays.',
    image: 'assets/images/hero-cover.png',
    mockPrice: '600 Astral Marks',
    rarity: 'Rare',
    featured: false,
    tags: ['Account', 'UI Skin', 'Frame'],
    includes: ['Animated Runic Frame', 'Title: "Archivist"']
  },
  {
    id: 'store-item-6',
    name: 'Aetheric Compass Emote',
    category: 'Other',
    description: 'Conjure a floating celestial compass of glowing runes to scout the horizon and point toward celestial nodes.',
    detailedLore: 'A social emote that summons a spinning ring of glowing ancient letters around your character before dissolving in cyan mist.',
    image: 'assets/images/world-exploration.jpg',
    mockPrice: '450 Astral Marks',
    rarity: 'Common',
    featured: false,
    tags: ['Emote', 'Animation', 'Social'],
    includes: ['Aetheric Compass Emote Animation', 'Special Sound Effect']
  }
];
