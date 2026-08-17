import { NewsArticle } from '../../shared/models/news.model';

export const MOCK_NEWS_ARTICLES: NewsArticle[] = [
  {
    id: 'news-1',
    slug: 'the-world-is-being-forged-alpha-overview',
    title: 'The World is Being Forged: An Introduction to Eivar Online',
    subtitle: 'Discover our development philosophy, world architecture, and the journey ahead.',
    excerpt: 'Welcome to the official chronicle of Eivar Online. Discover how our floating lands, ancient stone arches, and runic weapon system come together.',
    content: [
      'Eivar Online is an ambitious online fantasy world built from the ground up to return a genuine sense of adventure, mystery, and deep player agency to the genre.',
      'Our world is not a sequence of linear quest corridors. From the towering ancient stone arches that pierce the sky to the floating islands suspended over pristine pine valleys, every horizon in Eivar is a real place waiting to be charted.',
      'Central to this vision is our Runic Inscription framework. We rejected rigid class systems with fixed elemental weapons. In Eivar, you choose a weapon for its physical mechanics and kinetic weight, then inscribe Essences, Modifiers, and Ancient Words to define how magic flows through it.',
      'Over the coming months, our team will share deep dives into exploration mechanics, world persistence, city fortresses, and tactical combat encounters.'
    ],
    category: 'Development',
    publishedAt: 'August 14, 2026',
    image: 'assets/images/hero-cover.png',
    readingTime: 4,
    tags: ['Alpha', 'Vision', 'World Design', 'Combat'],
    featured: true,
    author: {
      name: 'Eivar Development Team',
      role: 'Lead World Architect'
    }
  },
  {
    id: 'news-2',
    slug: 'runic-weapon-system-deep-dive',
    title: 'Deep Dive: Essences, Modifiers, and Ancient Words',
    subtitle: 'How weapon crafting and rune engraving replace traditional static spellbooks.',
    excerpt: 'Explore how combining physical base weapons with elemental essences and runic modifiers yields hundreds of emergent spell behaviors.',
    content: [
      'In traditional fantasy RPGs, finding a "Fire Staff" means you are locked into a predefined set of fire spells. In Eivar Online, the weapon itself is merely the conduit.',
      'A Channeling Staff dictates cast time cadence, channel stability, and kinetic recoil. When you slot a Fire Essence into its Q-slot (Arcane Orb), it converts the spell to flame. Adding an "Expand" Modifier increases its blast radius upon detonation.',
      'When you reach higher tiers and discover lost Ancient Words such as "Echo", your ultimate abilities can repeat or cascade through enemies, forging a truly custom spellcasting signature.'
    ],
    category: 'Development',
    publishedAt: 'August 08, 2026',
    image: 'assets/images/channeling-staff.jpg',
    readingTime: 5,
    tags: ['Weapons', 'Magic', 'Runes', 'Crafting'],
    featured: false,
    author: {
      name: 'Combat Engineering',
      role: 'Systems Designer'
    }
  },
  {
    id: 'news-3',
    slug: 'exploring-the-floating-lands-of-eivar',
    title: 'Into the Wilds: Floating Islands and Ancient Shrines',
    subtitle: 'A look at the environmental design and vertical navigation across the archipelago.',
    excerpt: 'Traverse the highlands where ancient monoliths hum with forgotten energy and waterfalls descend into mist.',
    content: [
      'The geography of Eivar is characterized by enormous scale and vertical diversity. Floating crags drift silently above the tree line, held aloft by ancient runic ley lines.',
      'Travelers will encounter standing stone shrines etched with ancestral glyphs. Activating these shrines can reveal hidden celestial bridges, attune local wind currents, or alert roaming creatures guarding ancient caches.',
      'Our team is tuning environmental lighting, atmospheric fog, and low-poly painterly textures to ensure every plateau offers breathtaking views without compromising performance.'
    ],
    category: 'Announcements',
    publishedAt: 'July 28, 2026',
    image: 'assets/images/world-exploration.jpg',
    readingTime: 3,
    tags: ['Exploration', 'Environment', 'Art'],
    featured: false,
    author: {
      name: 'Environment Art',
      role: 'Level Designer'
    }
  },
  {
    id: 'news-4',
    slug: 'citadels-and-guild-strongholds',
    title: 'Strongholds & Fortresses: Civilizations in Conflict',
    subtitle: 'How cities and defensive outposts form the backbone of territorial control.',
    excerpt: 'Inside the high stone walls and blue banners of Eivar’s capital citadels, players forge alliances and prepare for sieges.',
    content: [
      'Cities in Eivar Online are more than trade hubs. They are defensive sanctuaries built around ancient magical conduits.',
      'Guilds and factions can contribute resources to repair stone towers, reinforce city gates, and unlock municipal forge upgrades. During open conflict phases, defending the outer ramparts requires coordinated tactics and weapon synergies.',
      'Stay tuned as we reveal the siege engine prototypes and territory influence systems in our upcoming development build.'
    ],
    category: 'Community',
    publishedAt: 'July 15, 2026',
    image: 'assets/images/ancient-citadel.jpg',
    readingTime: 4,
    tags: ['Guilds', 'PvP', 'Citadels', 'Factions'],
    featured: false,
    author: {
      name: 'Community Nexus',
      role: 'Community Lead'
    }
  }
];
