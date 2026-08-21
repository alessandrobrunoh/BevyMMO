export interface ItemDetailInfo {
  id: string;
  name: string;
  category: 'Weapons' | 'Armor' | 'Accessories' | 'Essences' | 'Materials';
  subType: string;
  slot: string;
  rarity: 'Common' | 'Uncommon' | 'Rare' | 'Epic' | 'Legendary' | 'Ancient';
  runeSymbol: string;
  defaultMarketId: string;
  description: string;
  lore: string;
  stats: Array<{ label: string; value: string; highlight?: boolean }>;
  combatAbilities?: Array<{ slot: string; name: string; type: string; desc: string }>;
  craftingOrigin: string;
}

export const ITEM_REGISTRY: Record<string, ItemDetailInfo> = {
  sword: {
    id: 'sword',
    name: 'Iron Broadsword',
    category: 'Weapons',
    subType: 'One-Handed Bladed Weapon',
    slot: 'Main Hand',
    rarity: 'Common',
    runeSymbol: 'ᛏ',
    defaultMarketId: 'market_1',
    description: 'A balanced forged steel blade suited for swift melee strikes, thrusts, and close-quarters parries.',
    lore: 'Standard issue among the vanguard of the High Citadel. Its edge holds a modest runic attunement channel.',
    stats: [
      { label: 'Weapon Type', value: 'One-Handed Sword' },
      { label: 'Damage', value: '45 - 58 Kinetic Slash', highlight: true },
      { label: 'Attack Cadence', value: '0.85s' },
      { label: 'Critical Strike', value: '12% Chance (1.5x)' },
      { label: 'Runic Slots', value: '3 Active (Q, W, E)' },
      { label: 'Durability', value: '250 / 250' }
    ],
    combatAbilities: [
      { slot: 'Q', name: 'Slash', type: 'Instant Cone', desc: 'Sweeping forward slash hitting all foes in melee reach.' },
      { slot: 'W', name: 'Riposte', type: 'Defensive Stance', desc: 'Deflects the next physical attack and retaliates with a precision thrust.' },
      { slot: 'E', name: 'Whirlwind', type: 'Spin Attack', desc: 'Spins in a 360-degree arc dealing heavy kinetic damage.' }
    ],
    craftingOrigin: 'Forged at Citadel Smithies using Iron Ingots and Hardwood Grip.'
  },
  bow: {
    id: 'bow',
    name: 'Recurve Longbow',
    category: 'Weapons',
    subType: 'Two-Handed Ranged Weapon',
    slot: 'Two-Handed',
    rarity: 'Common',
    runeSymbol: 'ᛉ',
    defaultMarketId: 'market_1',
    description: 'A flexible composite longbow capable of loosing high-velocity arrows over significant distances.',
    lore: 'Carved from aged ironwood harvested near the whispering cliffs. Favored by scouts and frontier sentinels.',
    stats: [
      { label: 'Weapon Type', value: 'Two-Handed Bow' },
      { label: 'Damage', value: '52 - 68 Piercing Kinetic', highlight: true },
      { label: 'Base Range', value: '28 meters' },
      { label: 'Draw Speed', value: '1.1s' },
      { label: 'Armor Penetration', value: '18%' },
      { label: 'Runic Slots', value: '3 Active (Q, W, E)' }
    ],
    combatAbilities: [
      { slot: 'Q', name: 'Aimed Shot', type: 'Linear Projectile', desc: 'Draws deeply to loose a piercing projectile with high critical chance.' },
      { slot: 'W', name: 'Volley', type: 'Area Rain', desc: 'Fires a cluster of arrows skyward that rain upon the target area.' },
      { slot: 'E', name: 'Evasive Roll', type: 'Disengage', desc: 'Vaults backward while firing a crippling arrow at the nearest adversary.' }
    ],
    craftingOrigin: 'Fletched from Ironwood Staves and Braided Sinew Strings.'
  },
  hammer: {
    id: 'hammer',
    name: 'Heavy War Hammer',
    category: 'Weapons',
    subType: 'Two-Handed Crushing Weapon',
    slot: 'Two-Handed',
    rarity: 'Rare',
    runeSymbol: 'ᚲ',
    defaultMarketId: 'market_1',
    description: 'A massive stone-and-steel war hammer engineered to deliver concussive impacts and shatter defenses.',
    lore: 'Inscribed with gravity runes that amplify the weapon weight at the exact apex of the downward swing.',
    stats: [
      { label: 'Weapon Type', value: 'Two-Handed Hammer' },
      { label: 'Damage', value: '78 - 105 Concussive Impact', highlight: true },
      { label: 'Attack Cadence', value: '1.45s' },
      { label: 'Stun Duration', value: '1.2s on Impact' },
      { label: 'Shield Destruction', value: '+40% Block Break' },
      { label: 'Runic Slots', value: '3 Active (Q, W, E)' }
    ],
    combatAbilities: [
      { slot: 'Q', name: 'Heavy Smash', type: 'Overhead Slam', desc: 'Slams the ground with immense force, staggering the target.' },
      { slot: 'W', name: 'Ground Slam', type: 'Shockwave AOE', desc: 'Ruptures the earth in front, slowing enemy movement by 40%.' },
      { slot: 'E', name: 'Earth Shatter', type: 'Cataclysm Knockdown', desc: 'Channels energy to erupt earth spikes, knocking enemies airborne.' }
    ],
    craftingOrigin: 'Smelted from Heavy Granite Core and Runic Steel Bands.'
  },
  mage_staff: {
    id: 'mage_staff',
    name: 'Channeling Staff',
    category: 'Weapons',
    subType: 'Two-Handed Mystic Conduit',
    slot: 'Two-Handed',
    rarity: 'Rare',
    runeSymbol: 'ᚠ',
    defaultMarketId: 'market_1',
    description: 'A versatile mid-to-long range magical conduit with high resonance stability.',
    lore: 'Crafted from ancient carved stone and weathered iron. Its internal conduit channels raw celestial energy that adopts the properties of engraved Essences.',
    stats: [
      { label: 'Weapon Type', value: 'Conduit Staff' },
      { label: 'Resonance Stability', value: '94% (Low Jitter)', highlight: true },
      { label: 'Base Range', value: '24 meters' },
      { label: 'Channel Cadence', value: '1.2s' },
      { label: 'Mana Efficiency', value: '+15% Cost Reduction' },
      { label: 'Runic Slots', value: '3 Active (Q, W, E)' }
    ],
    combatAbilities: [
      { slot: 'Q', name: 'Arcane Orb', type: 'Linear Projectile', desc: 'Hurls a concentrated sphere of energy that bursts on impact.' },
      { slot: 'W', name: 'Runic Barrier', type: 'Protective Ward', desc: 'Projects a stationary prism field in front that absorbs hostile projectiles.' },
      { slot: 'E', name: 'Great Impact', type: 'Ground Cataclysm', desc: 'Strikes the staff to the earth, creating a shockwave that knocks back foes.' }
    ],
    craftingOrigin: 'Carved at the Attunement Spires from Polished Granite and Arcane Resonator Crystals.'
  },
  channeling_staff: {
    id: 'channeling_staff',
    name: 'Channeling Staff',
    category: 'Weapons',
    subType: 'Two-Handed Mystic Conduit',
    slot: 'Two-Handed',
    rarity: 'Rare',
    runeSymbol: 'ᚠ',
    defaultMarketId: 'market_1',
    description: 'A versatile mid-to-long range magical conduit with high resonance stability.',
    lore: 'Crafted from ancient carved stone and weathered iron. Its internal conduit channels raw celestial energy that adopts the properties of engraved Essences.',
    stats: [
      { label: 'Weapon Type', value: 'Conduit Staff' },
      { label: 'Resonance Stability', value: '94% (Low Jitter)', highlight: true },
      { label: 'Base Range', value: '24 meters' },
      { label: 'Channel Cadence', value: '1.2s' },
      { label: 'Mana Efficiency', value: '+15% Cost Reduction' },
      { label: 'Runic Slots', value: '3 Active (Q, W, E)' }
    ],
    combatAbilities: [
      { slot: 'Q', name: 'Arcane Orb', type: 'Linear Projectile', desc: 'Hurls a concentrated sphere of energy that bursts on impact.' },
      { slot: 'W', name: 'Runic Barrier', type: 'Protective Ward', desc: 'Projects a stationary prism field in front that absorbs hostile projectiles.' },
      { slot: 'E', name: 'Great Impact', type: 'Ground Cataclysm', desc: 'Strikes the staff to the earth, creating a shockwave that knocks back foes.' }
    ],
    craftingOrigin: 'Carved at the Attunement Spires from Polished Granite and Arcane Resonator Crystals.'
  },
  simple_helm: {
    id: 'simple_helm',
    name: 'Padded Leather Helm',
    category: 'Armor',
    subType: 'Head Armor',
    slot: 'Helmet',
    rarity: 'Common',
    runeSymbol: 'ᛋ',
    defaultMarketId: 'market_2',
    description: 'A sturdy reinforced cap offering basic cranial deflection without obstructing peripheral vision.',
    lore: 'Tanned hide layered with light iron rivets, standard protection for travelers navigating the outer wilds.',
    stats: [
      { label: 'Armor Slot', value: 'Helmet' },
      { label: 'Armor Rating', value: '+14 Defense', highlight: true },
      { label: 'Movement Penalty', value: '0% (Unrestricted)' },
      { label: 'Weight', value: '1.5 kg' },
      { label: 'Durability', value: '120 / 120' }
    ],
    craftingOrigin: 'Stitched with Thick Hide and Iron Rivets.'
  },
  simple_cuirass: {
    id: 'simple_cuirass',
    name: 'Iron Studded Cuirass',
    category: 'Armor',
    subType: 'Chestplate',
    slot: 'Chest / Armor',
    rarity: 'Common',
    runeSymbol: 'ᛋ',
    defaultMarketId: 'market_2',
    description: 'A hardened leather chestpiece studded with overlapping iron discs across vital organs.',
    lore: 'Designed for durability and freedom of movement during prolonged highland skirmishes.',
    stats: [
      { label: 'Armor Slot', value: 'Chestplate' },
      { label: 'Armor Rating', value: '+32 Defense', highlight: true },
      { label: 'Physical Mitigation', value: '12% Slash / Pierce' },
      { label: 'Stamina Regeneration', value: '98% (Minimal Drag)' },
      { label: 'Durability', value: '250 / 250' }
    ],
    craftingOrigin: 'Tanned beast hide reinforced with hammered iron studs.'
  },
  simple_buckler: {
    id: 'simple_buckler',
    name: 'Wooden Buckler',
    category: 'Armor',
    subType: 'Shield',
    slot: 'Off-Hand',
    rarity: 'Common',
    runeSymbol: 'ᛋ',
    defaultMarketId: 'market_2',
    description: 'A compact round shield used to deflect incoming blows and parry opponent weapon arcs.',
    lore: 'Lightweight and agile, allowing rapid defensive recovery when paired with a one-handed sword.',
    stats: [
      { label: 'Equipment Slot', value: 'Off-Hand Shield' },
      { label: 'Block Value', value: '28 Damage Absorbed', highlight: true },
      { label: 'Parry Window', value: '0.25s Active Reaction' },
      { label: 'Block Stamina Cost', value: '-15% Reduction' },
      { label: 'Durability', value: '180 / 180' }
    ],
    craftingOrigin: 'Carved from hardened oak with a central iron boss.'
  },
  simple_cape: {
    id: 'simple_cape',
    name: 'Traveler Cloak',
    category: 'Armor',
    subType: 'Back Accessory',
    slot: 'Cloak',
    rarity: 'Common',
    runeSymbol: 'ᛋ',
    defaultMarketId: 'market_2',
    description: 'A durable weather-resistant mantle that protects from windchill and damp highland mists.',
    lore: 'Woven with coarse mountain wool by the weavers of the lower settlements.',
    stats: [
      { label: 'Equipment Slot', value: 'Back / Cloak' },
      { label: 'Elemental Resistance', value: '+8 Frost & Fire', highlight: true },
      { label: 'Stamina Recovery', value: '+3% Passive' },
      { label: 'Weight', value: '0.8 kg' }
    ],
    craftingOrigin: 'Woven from Highland Wool on Village Looms.'
  },
  simple_boots: {
    id: 'simple_boots',
    name: 'Traveler Treads',
    category: 'Armor',
    subType: 'Footwear',
    slot: 'Boots',
    rarity: 'Common',
    runeSymbol: 'ᛋ',
    defaultMarketId: 'market_2',
    description: 'Sturdy leather boots with gripped soles designed for traversing rocky paths and mountain steps.',
    lore: 'Built for endurance and steady footing during long overland journeys across Eivar.',
    stats: [
      { label: 'Equipment Slot', value: 'Boots' },
      { label: 'Movement Speed', value: '+5% Sprint', highlight: true },
      { label: 'Armor Rating', value: '+8 Defense' },
      { label: 'Terrain Drag', value: '-20% Penalty' },
      { label: 'Durability', value: '150 / 150' }
    ],
    craftingOrigin: 'Cobbled from Cured Leather and Iron Tread-Nails.'
  },
  robust_cuirass: {
    id: 'robust_cuirass',
    name: 'Robust Vanguard Cuirass',
    category: 'Armor',
    subType: 'Heavy Plate Armor',
    slot: 'Chest / Armor',
    rarity: 'Rare',
    runeSymbol: 'ᛟ',
    defaultMarketId: 'market_2',
    description: 'Forged plate armor providing exceptional defense against heavy slashing and crushing impacts.',
    lore: 'Layered with tempered steel and inscribed with reinforcement wards along the breastplate seam.',
    stats: [
      { label: 'Armor Slot', value: 'Chestplate' },
      { label: 'Armor Rating', value: '+65 Defense', highlight: true },
      { label: 'Max Health Bonus', value: '+120 HP' },
      { label: 'Physical Mitigation', value: '22% All Damage' },
      { label: 'Knockback Resistance', value: '+30%' },
      { label: 'Durability', value: '450 / 450' }
    ],
    craftingOrigin: 'Master-forged with Tempered Steel Plates and Runic Alloy Rivets.'
  },
  warding_helm: {
    id: 'warding_helm',
    name: 'Warding Greathelm',
    category: 'Armor',
    subType: 'Heavy Helmet',
    slot: 'Helmet',
    rarity: 'Rare',
    runeSymbol: 'ᛟ',
    defaultMarketId: 'market_2',
    description: 'An enclosed steel greathelm etched with protective runes against concussive trauma and psychic feedback.',
    lore: 'Worn by Citadel Knights when holding breaches against corrupted entities.',
    stats: [
      { label: 'Armor Slot', value: 'Helmet' },
      { label: 'Armor Rating', value: '+38 Defense', highlight: true },
      { label: 'Concussive Resistance', value: '+25% Stun Reduction' },
      { label: 'Spell Ward', value: '+15 Arcane Resistance' },
      { label: 'Durability', value: '300 / 300' }
    ],
    craftingOrigin: 'Forged from Tempered Plate and inlaid with Silver Warding Inscriptions.'
  },
  swift_boots: {
    id: 'swift_boots',
    name: 'Swift Windstride Greaves',
    category: 'Armor',
    subType: 'Light Footwear',
    slot: 'Boots',
    rarity: 'Rare',
    runeSymbol: 'ᛟ',
    defaultMarketId: 'market_2',
    description: 'Feather-light greaves enchanted with wind runes that drastically increase sprint cadence and roll recovery.',
    lore: 'Crafted for vanguard scouts who need to outpace hostile creatures in open terrain.',
    stats: [
      { label: 'Equipment Slot', value: 'Boots' },
      { label: 'Movement Speed', value: '+12% Sprint', highlight: true },
      { label: 'Dodge Recovery', value: '-20% Cooldown' },
      { label: 'Armor Rating', value: '+18 Defense' },
      { label: 'Sprint Stamina Cost', value: '-15%' }
    ],
    craftingOrigin: 'Stitched from Shadow-Leopard Hide and Wind Essence Needles.'
  },
  purity_charm: {
    id: 'purity_charm',
    name: 'Purity Amulet',
    category: 'Accessories',
    subType: 'Runic Talisman',
    slot: 'Accessory',
    rarity: 'Rare',
    runeSymbol: 'ᚹ',
    defaultMarketId: 'market_2',
    description: 'A carved stone amulet containing an uncorrupted crystal that slowly dispels status afflictions.',
    lore: 'Blessed at the Sunken Shrine to preserve the wearer mind from aetheric corruption.',
    stats: [
      { label: 'Equipment Slot', value: 'Neck Accessory' },
      { label: 'Passive Health Regen', value: '+3.5 HP/s', highlight: true },
      { label: 'Corruption Ward', value: '+35 Resistance' },
      { label: 'Debuff Duration', value: '-25% Duration' },
      { label: 'Attunement Cost', value: 'Zero Conduit Load' }
    ],
    craftingOrigin: 'Carved from Alabaster Shrine Stone and blessed with Life Essence.'
  }
};

export function getItemDetail(itemId: string): ItemDetailInfo {
  if (ITEM_REGISTRY[itemId]) {
    return ITEM_REGISTRY[itemId];
  }

  // Generic fallback for any item id
  const formattedName = itemId
    .replace(/_/g, ' ')
    .replace(/\b\w/g, l => l.toUpperCase());

  return {
    id: itemId,
    name: formattedName,
    category: 'Materials',
    subType: 'Trade Commodity',
    slot: 'Inventory',
    rarity: 'Common',
    runeSymbol: 'ᛟ',
    defaultMarketId: 'market_1',
    description: `A tradeable item within the realm of Eivar. Can be bought, sold, and traded in regional market halls.`,
    lore: 'Recorded in the ancient trade scrolls of the Citadel exchange.',
    stats: [
      { label: 'Item Type', value: 'Trade Commodity' },
      { label: 'Stackable', value: 'Yes (Up to 99)', highlight: true },
      { label: 'Weight', value: '0.1 kg / unit' },
      { label: 'Trade Status', value: 'Free Exchange' }
    ],
    craftingOrigin: 'Gathered or crafted across the floating landmasses of Eivar.'
  };
}
