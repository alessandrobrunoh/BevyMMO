import { GameUpdate } from '../../shared/models/update.model';

export const MOCK_GAME_UPDATES: GameUpdate[] = [
  {
    id: 'update-0-2-1',
    version: 'Alpha 0.2.1',
    title: 'Runic Inscription Prototype & Movement Polish',
    type: 'Patch Notes',
    date: 'August 12, 2026',
    status: 'Live Alpha',
    summary: 'Introduces the interactive Channeling Staff weapon prototype, weapon slot engraving, updated character movement interpolation, and interface refinements.',
    highlights: [
      'Interactive Runic Inscription prototype with Q, W, E spell slots',
      'Channeling Staff physical conduit baseline',
      'Refined character dead reckoning and movement smoothing',
      'New UI stone sound effects and visual feedback'
    ],
    sections: [
      {
        category: 'NEW',
        items: [
          'Added prototype runic inscription interface allowing dynamic Essence and Modifier socketing.',
          'Added the Channeling Staff weapon archetype with baseline channel mechanics.',
          'Added three basic Essences for testing: Fire, Life, and Arcane.',
          'Added two primary Modifiers: Expand (AoE scaling) and Persistence (Duration bonus).'
        ]
      },
      {
        category: 'CHANGED',
        items: [
          'Updated character movement presentation and dead reckoning interpolation.',
          'Revised spell target acquisition radius for area abilities.',
          'Adjusted camera pitch limits when navigating steep cliffs and mountain passes.'
        ]
      },
      {
        category: 'BALANCE',
        items: [
          'Reduced mana consumption of baseline Channeling Staff attacks by 12%.',
          'Increased base projectile velocity of Arcane Orb by 15%.'
        ]
      },
      {
        category: 'FIXED',
        items: [
          'Fixed an issue where fast camera rotation caused subtle terrain clipping.',
          'Fixed tooltip positioning on wide desktop resolutions.',
          'Resolved an edge case where rune sockets would retain glow after unslotting.'
        ]
      },
      {
        category: 'TECHNICAL',
        items: [
          'Optimized WebAssembly tick state replication pipeline.',
          'Reduced client draw calls for distant floating island foliage.'
        ]
      }
    ]
  },
  {
    id: 'update-0-2-0',
    version: 'Alpha 0.2.0',
    title: 'Highland Biome & Standing Shrines',
    type: 'Development',
    date: 'July 24, 2026',
    status: 'Live Alpha',
    summary: 'The initial rollout of the Highland wilderness area, featuring floating crags, waterfalls, and interactive ancient rune monoliths.',
    highlights: [
      'Highland wilderness territory open for test exploration',
      'Standing rune monoliths with lore attunements',
      'Day/night atmospheric lighting system',
      'Initial spatial audio integration'
    ],
    sections: [
      {
        category: 'NEW',
        items: [
          'First playable Highland territory featuring stone arch formations and waterfalls.',
          'Five interactive Rune Monoliths scattered throughout the wild zones.',
          'Ambient creature wildlife spawns and pathing routes.'
        ]
      },
      {
        category: 'CHANGED',
        items: [
          'Updated atmospheric sky shaders to match the painterly low-poly concept art.',
          'Enhanced water reflection rendering on cascading falls.'
        ]
      },
      {
        category: 'FIXED',
        items: [
          'Fixed player getting stuck on stone arch collision meshes.',
          'Resolved lighting flicker when passing under dense pine canopies.'
        ]
      }
    ]
  },
  {
    id: 'update-0-1-9',
    version: 'Alpha 0.1.9',
    title: 'Core Engine Foundation & Network Synchronization',
    type: 'Patch Notes',
    date: 'July 02, 2026',
    status: 'Archive',
    summary: 'Initial multiplayer prototype synchronization test with authoritative server tick pipeline.',
    highlights: [
      'Multiplayer entity synchronization test',
      'Core movement prediction foundation',
      'Basic UI shell and inventory frame'
    ],
    sections: [
      {
        category: 'NEW',
        items: [
          'Core networking loop with deterministic tick simulation.',
          'Basic client HUD with health, energy, and ability bar.'
        ]
      },
      {
        category: 'TECHNICAL',
        items: [
          'Memory optimization for client entity pooling.',
          'Initial stress test for concurrent player sessions.'
        ]
      }
    ]
  }
];
