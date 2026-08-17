import { WikiCategory, WikiArticle } from '../../shared/models/wiki.model';

export const MOCK_WIKI_CATEGORIES: WikiCategory[] = [
  {
    id: 'cat-weapons',
    slug: 'weapons',
    name: 'Weapons',
    description: 'Physical archetypes that establish base mechanics, attack cadence, and conduit properties.',
    runeSymbol: 'ᛏ',
    articleCount: 6
  },
  {
    id: 'cat-essences',
    slug: 'essences',
    name: 'Essences',
    description: 'Magical and elemental infusions that transform physical abilities into specialized elemental arts.',
    runeSymbol: 'ᚠ',
    articleCount: 8
  },
  {
    id: 'cat-modifiers',
    slug: 'modifiers',
    name: 'Modifiers',
    description: 'Geometric and kinetic adjustments altering area of effect, duration, velocity, and targeting.',
    runeSymbol: 'ᚱ',
    articleCount: 12
  },
  {
    id: 'cat-ancient-words',
    slug: 'ancient-words',
    name: 'Ancient Words',
    description: 'Legendary incantations that trigger catastrophic shifts, echoes, and chain reactions.',
    runeSymbol: 'ᛟ',
    articleCount: 5
  },
  {
    id: 'cat-world',
    slug: 'world',
    name: 'World & Lore',
    description: 'The geography, floating landmasses, stone arches, and history of Eivar.',
    runeSymbol: 'ᚹ',
    articleCount: 9
  },
  {
    id: 'cat-combat',
    slug: 'combat',
    name: 'Combat System',
    description: 'Dead reckoning, timing, hitboxes, defensive barriers, and positional tactics.',
    runeSymbol: 'ᛉ',
    articleCount: 7
  },
  {
    id: 'cat-crafting',
    slug: 'crafting',
    name: 'Crafting & Forging',
    description: 'Gathering celestial ores, carving rune slabs, and attuning magical prisms.',
    runeSymbol: 'ᚲ',
    articleCount: 11
  },
  {
    id: 'cat-guilds',
    slug: 'guilds',
    name: 'Guilds & Strongholds',
    description: 'Municipal alliances, fortress upgrades, territorial banners, and warfare.',
    runeSymbol: 'ᚦ',
    articleCount: 4
  }
];

export const MOCK_WIKI_ARTICLES: WikiArticle[] = [
  {
    id: 'art-channeling-staff',
    slug: 'channeling-staff',
    categorySlug: 'weapons',
    categoryName: 'Weapons',
    title: 'Channeling Staff',
    subtitle: 'A versatile mid-to-long range magical conduit with high resonance stability.',
    overview: 'The Channeling Staff is a two-handed mystic weapon forged from ancient carved stone and weathered iron. Rather than commanding a predetermined element, its internal conduit channels raw celestial energy that adopts the properties of whatever Essences and Modifiers the wielder engraves into its runic slots.',
    lastUpdated: 'August 10, 2026',
    image: 'assets/images/channeling-staff.jpg',
    infobox: {
      title: 'Channeling Staff',
      type: 'Two-Handed Magical Conduit',
      rarity: 'Ancient',
      image: 'assets/images/channeling-staff.jpg',
      stats: [
        { label: 'Weapon Type', value: 'Conduit Staff' },
        { label: 'Base Range', value: '24 meters' },
        { label: 'Channel Cadence', value: '1.2s' },
        { label: 'Resonance Stability', value: '94%', highlight: true },
        { label: 'Engraved Slots', value: '3 Active (Q, W, E)' },
        { label: 'Primary Scaling', value: 'Aetheric Will' }
      ]
    },
    abilities: [
      {
        slot: 'Q',
        name: 'Arcane Orb',
        baseType: 'Linear Projectile',
        description: 'Hurls a concentrated sphere of energy that bursts on impact, dealing kinetic damage to the target.',
        cooldown: '3.5s',
        energyCost: '25 Mana',
        recommendedEssence: 'Fire (converts burst to explosive burning blast)',
        recommendedModifiers: ['Expand (increases explosion radius by 40%)']
      },
      {
        slot: 'W',
        name: 'Runic Barrier',
        baseType: 'Protective Ward',
        description: 'Projects a stationary prism field in front of the caster that absorbs incoming hostile projectiles.',
        cooldown: '12.0s',
        energyCost: '45 Mana',
        recommendedEssence: 'Life (causes barrier to heal allies standing within)',
        recommendedModifiers: ['Persistence (extends field duration by 3.5s)']
      },
      {
        slot: 'E',
        name: 'Great Impact',
        baseType: 'Ground Cataclysm',
        description: 'Strikes the staff to the earth, creating a shockwave that knocks back nearby adversaries and fractures the ground.',
        cooldown: '22.0s',
        energyCost: '80 Mana',
        recommendedEssence: 'Life or Fire',
        recommendedModifiers: ['Expand', 'Persistence'],
        recommendedAncientWord: 'Echo (re-triggers the shockwave after 1.5s delay)'
      }
    ],
    runeFormulas: [
      {
        title: 'Infernal Sunburst Build',
        description: 'Optimized for high burst AoE territory disruption.',
        baseWeapon: 'Channeling Staff',
        essence: 'Fire Essence (Sunfire Variant)',
        modifiers: ['Expand (+45% Radius)', 'Velocity (+20% Speed)'],
        ancientWord: 'Echo',
        resultEffect: 'Arcane Orb detonates in a 12m roaring firestorm, followed 1.5s later by an identical second eruption.'
      },
      {
        title: 'Sanctuary Warder Build',
        description: 'Defensive support build for holding fortress chokepoints.',
        baseWeapon: 'Channeling Staff',
        essence: 'Life Essence (Verdant Flow)',
        modifiers: ['Persistence (+4s Duration)', 'Amplify (+30% Healing)'],
        resultEffect: 'Barrier converts 100% of absorbed enemy projectile kinetic energy into radiating regenerative pulses for allies.'
      }
    ],
    sections: [
      {
        id: 'overview-section',
        title: 'Weapon Overview & Mechanics',
        content: [
          'The Channeling Staff represents the pinnacle of ancient runic metallurgy. Its central core contains a suspended crystal prism that remains in harmonic balance regardless of sudden movement.',
          'Unlike bladed weapons that rely on physical edge retention, staff effectiveness is determined by your channel stability. Disruptions from enemy stuns or heavy knockbacks will temporarily destabilize the conduit flow.'
        ],
        callout: {
          type: 'info',
          title: 'Conduit Principle',
          text: 'Remember that the weapon does not dictate the element. If you prefer ice over fire, simply replace the Fire Essence with an Ice Essence at any Attunement Altar.'
        }
      },
      {
        id: 'compatibility-table',
        title: 'Conduit Attribute Matrix',
        content: [
          'The staff features a balanced power curve with high affinity for sustained channeling and area modifiers.'
        ],
        table: {
          headers: ['Attribute', 'Base Value', 'Tier 2 Scaling', 'Tier 3 Scaling'],
          rows: [
            ['Cast Velocity', '100%', '115%', '130%'],
            ['Aetheric Penetration', '15%', '28%', '42%'],
            ['Max Inscription Weight', '10 Slots', '16 Slots', '24 Slots'],
            ['Energy Efficiency', '100%', '110%', '125%']
          ]
        }
      }
    ],
    relatedSlugs: [
      { title: 'Essences Overview', categorySlug: 'essences', slug: 'essences-overview' },
      { title: 'Ancient Word: Echo', categorySlug: 'ancient-words', slug: 'ancient-word-echo' },
      { title: 'Modifiers Guide', categorySlug: 'modifiers', slug: 'modifiers-overview' }
    ]
  },
  {
    id: 'art-essences-overview',
    slug: 'essences-overview',
    categorySlug: 'essences',
    categoryName: 'Essences',
    title: 'Essences of Eivar',
    subtitle: 'The fundamental primal forces channeled through runic inscriptions.',
    overview: 'Essences are crystallized elemental forces extracted from celestial nodes across the world. When inscribed onto a weapon socket, an Essence infuses the baseline physical ability with distinct behavioral and status consequences.',
    lastUpdated: 'August 06, 2026',
    image: 'assets/images/world-exploration.jpg',
    infobox: {
      title: 'Essences Overview',
      type: 'Core Inscription Material',
      rarity: 'Ancient',
      stats: [
        { label: 'Primary Families', value: 'Fire, Ice, Life, Storm, Void' },
        { label: 'Slot Requirement', value: '1 Primary per Ability' },
        { label: 'Attunement Cost', value: 'Standard Inscription' }
      ]
    },
    sections: [
      {
        id: 'elemental-types',
        title: 'The Five Elemental Spheres',
        content: [
          'Fire: Focuses on explosive initial detonation and lingering thermal burn stacks.',
          'Ice: Slows adversary animation speeds and creates crystalline physical obstacles on the terrain.',
          'Life: Converts kinetic output into regenerative auras and defensive cleansing waves.',
          'Storm: Introduces chain lightning arcs that jump between clustered targets.',
          'Void: Creates localized gravitational singularities that draw enemies toward the epicenter.'
        ],
        callout: {
          type: 'tip',
          title: 'Essence Swapping',
          text: 'You can swap Essences when resting near any safe camp or inside a city attunement shrine.'
        }
      }
    ],
    relatedSlugs: [
      { title: 'Channeling Staff', categorySlug: 'weapons', slug: 'channeling-staff' },
      { title: 'Ancient Word: Echo', categorySlug: 'ancient-words', slug: 'ancient-word-echo' }
    ]
  },
  {
    id: 'art-ancient-word-echo',
    slug: 'ancient-word-echo',
    categorySlug: 'ancient-words',
    categoryName: 'Ancient Words',
    title: 'Ancient Word: Echo',
    subtitle: 'The lost language of the first architects that duplicates ability manifestations.',
    overview: 'Discovered carved into the gigantic stone arches of the highlands, the Ancient Word "Echo" causes an inscribed ability to replicate its primary manifestation after a brief harmonic delay.',
    lastUpdated: 'July 30, 2026',
    image: 'assets/images/ancient-citadel.jpg',
    infobox: {
      title: 'Ancient Word: Echo',
      type: 'Transformative Inscription',
      rarity: 'Mythic',
      stats: [
        { label: 'Word Category', value: 'Temporal Resonance' },
        { label: 'Replication Delay', value: '1.50 seconds' },
        { label: 'Echo Power', value: '75% of original damage/healing' },
        { label: 'Required Rank', value: 'Elder Inscriber' }
      ]
    },
    sections: [
      {
        id: 'echo-mechanics',
        title: 'Harmonic Replication Mechanics',
        content: [
          'When "Echo" is engraved into an ability slot, the game engine registers both the initial trigger and a delayed second iteration at the exact point of impact.',
          'If applied to an area impact like Great Impact, the first shockwave knocks foes into the air, and the echo shockwave strikes them again upon landing.'
        ]
      }
    ],
    relatedSlugs: [
      { title: 'Channeling Staff', categorySlug: 'weapons', slug: 'channeling-staff' },
      { title: 'Essences Overview', categorySlug: 'essences', slug: 'essences-overview' }
    ]
  }
];
