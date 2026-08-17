You are a senior frontend engineer, Angular architect, UI/UX designer, and game website designer.

Your task is to DESIGN AND IMPLEMENT a complete frontend prototype for the fantasy MMORPG:

EIVAR ONLINE

Use the latest stable Angular version available in the environment.

The project must be a real Angular application, not a static HTML mockup.

============================================================
0. PRIMARY OBJECTIVE
============================================================

Create a polished, responsive, highly interactive official website prototype for Eivar Online.

The website must include:

- Game presentation / landing page
- News
- Individual news articles
- Development updates / patch notes
- Wiki
- Wiki articles
- Mock Store
- Login
- Basic mock account UI
- Global search
- Responsive navigation
- Footer
- Reusable UI components
- Mock content and data
- Animations and interaction states

IMPORTANT:

There is currently NO finished backend.

DO NOT implement backend functionality.

DO NOT invent APIs.

DO NOT create fake REST endpoints.

DO NOT implement payment processing.

DO NOT implement real authentication.

DO NOT implement database logic.

However, the frontend must NOT feel static.

Implement all reasonable UI interactions and states.

Examples:

- navigation
- dropdowns
- hover states
- active states
- filters
- tabs
- accordions
- forms
- local validation
- modals
- drawers
- tooltips
- toasts
- mock loading states
- search
- responsive navigation
- item previews
- article navigation
- animations
- transitions
- password visibility
- mock login feedback

Use mock/local data whenever data is required.

The architecture must make it easy to replace mock providers with real backend services in the future.

============================================================
1. VISUAL REFERENCE
============================================================

Use the supplied Eivar Online cover artwork as the MAIN artistic reference for the entire project.

Study its visual language.

The artwork contains:

- an enormous bright blue sky
- stylized clouds
- floating islands
- fantasy mountain ranges
- gigantic ancient stone arches
- medieval fantasy cities
- forests and vegetation
- blue magical energy
- ancient glowing runes
- banners
- warriors
- ruins
- exploration
- large environmental scale
- stylized low-poly / painterly fantasy rendering

The website must visually feel like part of that world.

Do NOT create a generic MMORPG website.

Do NOT create a dark black/red interface.

Do NOT create a generic Tailwind/SaaS landing page.

Do NOT create a World of Warcraft imitation.

Do NOT create a futuristic sci-fi interface.

Do NOT cover everything in gold fantasy borders.

Eivar should feel:

ADVENTUROUS
MYSTICAL
ANCIENT
BRIGHT
BLUE
NATURAL
RUNIC
EXPLORATORY
PREMIUM
READABLE
MODERN
FANTASY

============================================================
2. VISUAL IDENTITY
============================================================

Create a visual design system inspired by:

- ancient carved stone
- weathered metal
- parchment
- banners
- mountains
- blue magical crystals
- ancient runes
- clear blue skies

The user interface should appear like a modern interface designed by a civilization from the Eivar world.

Fantasy aesthetics should come from subtle details.

Avoid excessive ornamentation.

Use strong visual hierarchy.

============================================================
3. COLOR DIRECTION
============================================================

Base the palette approximately around:

Sky Blue:
#147CC1
#1C91D0
#75C4E8

Deep Blue:
#102D47
#163B58
#1D4B6C

Stone:
#DDD4C3
#C8BCA5
#A89B84
#756B5A

Parchment:
#F1E8D6
#E7D6B9
#C9B58D

Ancient Gold:
#D0AE61
#B88B38
#E6CB86

Rune Cyan:
#3CCBFF
#69E0FF
#A4EEFF

Do not blindly use these colors everywhere.

Build a coherent token-based system.

Avoid pure #000 backgrounds where possible.

Dark areas should generally use deep blue / blue-grey.

Gold must be used carefully and sparingly.

Cyan is primarily related to magic and rune interactions.

============================================================
4. TYPOGRAPHY
============================================================

Use two complementary typography families.

HEADINGS:
Fantasy serif, carved/ancient feeling, elegant and sharp.

BODY / INTERFACE:
Highly readable modern font.

The logo/title aesthetic should resemble the EIVAR lettering visible in the supplied key art.

Do not use unreadable medieval fonts for body copy.

Use typography to create hierarchy.

============================================================
5. ANGULAR REQUIREMENTS
============================================================

Use modern Angular architecture.

Prefer:

- standalone components
- Angular Router
- lazy-loaded feature routes where appropriate
- Signals for local UI state where useful
- Reactive Forms where appropriate
- strongly typed TypeScript
- semantic templates
- modern Angular template syntax where appropriate
- reusable components

Avoid unnecessary complexity.

DO NOT add NgRx.

DO NOT add a state management framework.

DO NOT create micro-frontends.

DO NOT add a backend.

DO NOT introduce dependencies without a clear reason.

If SCSS is available/configured, use SCSS for the design system.

============================================================
6. PROJECT STRUCTURE
============================================================

Create a clean feature-oriented architecture similar to:

src/
  app/
    core/
      layout/
      navigation/
      services/

    shared/
      ui/
        button/
        modal/
        drawer/
        dropdown/
        rune-divider/
        section-title/
        page-header/
        image-card/
        search-overlay/
        toast/
      models/

    features/
      home/
      news/
      updates/
      wiki/
      store/
      auth/

    data/
      mocks/

    app.routes.ts
    app.config.ts

  assets/
    branding/
    images/
    icons/
    runes/
    textures/

  styles/
    _variables.scss
    _typography.scss
    _animations.scss
    _utilities.scss
    styles.scss

Modify this structure when there is a good architectural reason, but preserve clear separation between:

LAYOUT
FEATURES
REUSABLE UI
MOCK DATA
MODELS
STYLING

============================================================
7. GLOBAL LAYOUT
============================================================

Create an application shell containing:

- global navigation
- routed page content
- global search overlay
- toast container
- footer

The visual layout should alternate between:

cinematic artwork sections

and

structured readable content sections.

Do not create every page as a centered collection of cards.

============================================================
8. NAVIGATION
============================================================

Desktop navigation should approximately contain:

EIVAR logo/symbol

Game
News
Updates
Wiki
Store

Then on the right:

Search
Community
Login

The homepage navbar can initially be transparent over the hero.

After scroll, transition it into a translucent deep-blue header.

Use only restrained backdrop blur.

Add:

- active route states
- hover states
- keyboard focus states
- responsive behavior
- mobile drawer
- dropdown if useful

Mobile navigation should become an immersive full-screen or large side panel.

Make the mobile navigation visually polished.

============================================================
9. HOME PAGE
============================================================

Route:

/

This is the primary Eivar Online presentation page.

------------------------------------------------------------
9.1 HERO
------------------------------------------------------------

Use the supplied cover artwork as the dominant hero artwork.

The art should fill a large part of the initial viewport.

Do not hide the artwork under excessive text or UI.

Use carefully positioned gradients to guarantee readability.

Include:

EIVAR ONLINE logo/title.

Possible subtitle:

"Forge your weapon. Shape its magic. Write your legend."

Description:

A concise introduction to Eivar as an evolving fantasy online world.

Primary CTA:

DISCOVER EIVAR

Secondary CTA:

LATEST NEWS

Optional tertiary text:

AN ONLINE FANTASY WORLD IN DEVELOPMENT

Add subtle cinematic effects.

Possible examples:

- extremely slow background scale
- cloud drift
- parallax
- tiny floating particles
- subtle magical light
- scroll indicator

These effects must not distract from the artwork.

------------------------------------------------------------
9.2 INTRODUCTION
------------------------------------------------------------

Create a strong transition from the hero into the website.

Headline example:

A WORLD WRITTEN IN ANCIENT WORDS

Explain the high-level fantasy:

Eivar is a world of ancient civilizations, unexplored lands, massive structures and forgotten knowledge.

Do not invent excessive lore.

Use concise placeholder copy that can easily be replaced later.

------------------------------------------------------------
9.3 GAME PILLARS
------------------------------------------------------------

Create four large visually differentiated game pillars.

Suggested:

EXPLORE
Travel across cities, wilderness, ruins and unknown lands.

FORGE
Choose weapons that define the foundations of your combat style.

INSCRIBE
Modify abilities through Essences, Modifiers and Ancient Words.

CONQUER
Face creatures, players and conflicts across the world.

These should NOT look like four generic corporate feature cards.

Use environmental or game imagery.

Allow hover interactions.

------------------------------------------------------------
9.4 RUNIC WEAPON SYSTEM
------------------------------------------------------------

This is one of the important elements that differentiates Eivar.

Create a dedicated showcase section.

Core concept:

Eivar does NOT simply rely on rigid elemental weapon classes such as:

Fire Staff
Ice Staff
Lightning Staff

Instead:

the WEAPON defines the physical/base behavior.

An ESSENCE defines an elemental, magical or conceptual influence.

MODIFIERS alter behavior.

ANCIENT WORDS can produce major transformations.

Create a visual mock interface demonstrating this.

Example:

CHANNELING STAFF

Q
Arcane Orb

Essence:
Fire

Modifier:
Expand


W
Barrier

Essence:
Life

Modifier:
Persistence


E
Great Impact

Essence:
Life

Modifier:
Expand

Modifier:
Persistence

Ancient Word:
Echo

Visually represent these elements like inscriptions engraved into the weapon/interface.

They must NOT look like gems placed into generic RPG equipment sockets.

Use rune-inspired engraved slots.

The E / ultimate line can appear slightly more important and complex.

The purpose is to PRESENT the system, not implement gameplay logic.

------------------------------------------------------------
9.5 WORLD SHOWCASE
------------------------------------------------------------

Create a cinematic environmental section.

Possible headline:

THE WORLD DOES NOT END AT THE CITY WALLS

Present concepts such as:

Cities
Wildlands
Ruins
Guilds
PvP
PvE
Crafting
Exploration

Use restrained language because game systems may still change.

Do not make hard promises about unfinished systems.

------------------------------------------------------------
9.6 LATEST NEWS
------------------------------------------------------------

Display:

1 large featured article

and

3 supporting articles.

Use mock data.

Each article needs:

- artwork
- category
- title
- publication date
- short excerpt
- read more interaction

Create polished hover states.

------------------------------------------------------------
9.7 DEVELOPMENT SECTION
------------------------------------------------------------

Clearly communicate that Eivar Online is under development.

Suggested headline:

THE WORLD IS BEING FORGED

Show:

Alpha Development
Recent development update
Community link

Do not fake:

- millions of players
- launch date
- testimonials
- press quotes
- review scores

------------------------------------------------------------
9.8 FINAL CTA
------------------------------------------------------------

Create a large cinematic end section.

Possible copy:

YOUR STORY HAS NOT YET BEEN WRITTEN.

Actions:

JOIN THE COMMUNITY
FOLLOW DEVELOPMENT

Use an atmospheric image or artwork.

============================================================
10. NEWS
============================================================

Route:

/news

Create a proper editorial news experience.

Page header:

NEWS FROM EIVAR

Include:

- featured story
- article list/grid
- category filters
- search integration

Categories:

All
Announcements
Development
Community
Events

Mock article type:

interface NewsArticle {
  id: string;
  slug: string;
  title: string;
  excerpt: string;
  content: string;
  category: string;
  publishedAt: string;
  image: string;
  readingTime: number;
  tags: string[];
}

Keep mock data outside components.

Create several believable Eivar-related sample posts.

Do not create excessive fake lore.

============================================================
11. NEWS ARTICLE
============================================================

Route:

/news/:slug

Create a beautiful editorial page.

Include:

- large header artwork
- category
- article title
- date
- reading time
- article content
- optional inline artwork
- tags
- previous/next navigation
- related articles

Article typography must be very readable.

============================================================
12. UPDATES
============================================================

Route:

/updates

The Updates section is NOT identical to News.

NEWS:
editorial / announcements / community.

UPDATES:
development progress / builds / changes / patch notes.

Create tabs or filters:

DEVELOPMENT
PATCH NOTES

Mock update example:

ALPHA 0.2.1
August 2026

NEW
- Prototype runic inscription interface.
- Channeling Staff prototype.

CHANGED
- Updated character movement presentation.

FIXED
- Various interface issues.

Use sections such as:

NEW
CHANGED
BALANCE
FIXED
TECHNICAL

Make updates highly scannable.

Create an optional version navigation sidebar on large screens.

============================================================
13. WIKI
============================================================

Route:

/wiki

The Wiki should be one of the strongest parts of the application.

It should feel like the game's official archive / codex.

NOT a simple page containing cards.

Desktop layout should support:

LEFT:
Wiki navigation/categories

CENTER:
Article content

RIGHT:
Current article table of contents / metadata

Create a Wiki landing page.

Main title:

THE EIVAR ARCHIVES

Search:

"Search the Archives..."

Categories can include:

Getting Started
World
Combat
Weapons
Abilities
Essences
Modifiers
Ancient Words
Status Effects
Crafting
Items
Creatures
PvP
Guilds
Economy
Locations

Create visually different category symbols.

Use runic/fantasy iconography where appropriate.

============================================================
14. WIKI ARTICLES
============================================================

Example routes:

/wiki/weapons
/wiki/weapons/channeling-staff
/wiki/essences
/wiki/ancient-words

Create a reusable Wiki article layout.

Components should support content patterns such as:

- title
- description
- infobox
- tables
- stat rows
- callouts
- images
- ability blocks
- rune formulas
- breadcrumbs
- article sections
- related articles
- previous/next article

Create reusable components such as:

WikiInfoBox
WikiStatRow
WikiCallout
WikiTable
AbilityCard
RuneFormula
ArticleContents
WikiBreadcrumbs

Create enough mock Wiki content to demonstrate the system convincingly.

============================================================
15. MOCK STORE
============================================================

Route:

/store

THIS IS ONLY A FRONTEND PROTOTYPE.

Absolutely NO:

- Stripe
- PayPal
- payment provider
- checkout API
- real payment information
- transaction logic

Create a polished cosmetic/supporter-oriented fantasy store interface.

Possible categories:

FEATURED
COSMETICS
SUPPORTER PACKS
ACCOUNT
OTHER

Mock items can include:

WAYFARER CLOAK

FOUNDER BANNER

RUNIC CAMP DECORATION

EXPLORER PORTRAIT FRAME

ANCIENT SIGIL BANNER

Make the examples clearly fictional.

Each StoreItem can contain:

id
name
category
description
image
mockPrice
rarity
featured

Clicking an item should either:

open an item detail page

or

open a rich modal.

Include:

- large preview
- description
- mock price
- category
- purchase button

Clicking purchase should show something such as:

"Purchasing is unavailable in this prototype."

Do NOT pretend the purchase was completed.

============================================================
16. LOGIN
============================================================

Route:

/login

Implement the COMPLETE login presentation and frontend interaction.

No authentication backend.

Include:

Email

Password

Remember me

LOGIN

CREATE ACCOUNT

Forgot your password?

Implement:

- email format validation
- password required validation
- show/hide password
- focus states
- disabled states
- mock loading
- mock error
- mock success notification

Submitting must NOT actually authenticate.

Use a cinematic environmental background.

Create a restrained fantasy login panel.

It can combine:

deep translucent blue
stone
parchment
metal
rune accents

Avoid making it look like an enterprise admin login form.

============================================================
17. MOCK ACCOUNT UI
============================================================

Create a small account dropdown accessible from the navigation for demonstration.

Possible entries:

Profile
Account
Settings
Log Out

UI only.

No user account backend.

============================================================
18. GLOBAL SEARCH
============================================================

Implement a global search overlay.

Desktop shortcut interaction is welcome.

Search local mock content from:

News
Updates
Wiki

Provide categories in results.

Example search interaction:

User types:
"staff"

Results:

WIKI
Channeling Staff
Weapons

NEWS
Weapon System Development Update

UPDATES
Alpha 0.2.1

Implement:

- live filtering
- keyboard navigation where reasonable
- close button
- Escape to close
- empty state
- focus management

No backend.

============================================================
19. REUSABLE COMPONENT SYSTEM
============================================================

Create reusable primitives rather than repeatedly recreating markup.

Useful components include:

EivarButton
RuneDivider
SectionHeading
PageHero
ImageCard
NewsCard
UpdateBadge
Modal
Drawer
Dropdown
SearchOverlay
Toast
WikiCallout
WikiInfoBox
StoreItemCard

Do not over-componentize tiny fragments unnecessarily.

============================================================
20. BUTTON DESIGN
============================================================

Buttons should feel subtly fantasy-inspired.

Primary button:

stone / aged metal / subtle gold edge.

Hover:

slight elevation
edge highlight
very subtle rune illumination

Pressed:

small physical depth change

Focus:

clear accessible outline

Secondary buttons:

outlined / translucent

Rune actions:

cyan magical accent

Do not make every button rounded pills.

============================================================
21. CARD DESIGN
============================================================

Create different card archetypes.

NEWS CARD:
editorial.

WORLD CARD:
image-heavy and cinematic.

RUNE CARD:
engraved / system-related.

WIKI CARD:
information-oriented.

STORE CARD:
item showcase.

Do NOT create a single universal rounded card component and use it for everything.

============================================================
22. INTERACTION DESIGN
============================================================

Even without backend logic, every control that visually appears interactive should behave appropriately.

For example:

Buttons react.

Dropdowns open.

Filters filter mock data.

Tabs switch.

Accordions expand.

Navigation navigates.

Store items open.

Forms validate.

Search searches.

Wiki navigation updates.

Modals close.

Escape closes overlays.

Mobile drawer works.

Hover states exist.

Focus states exist.

Loading states exist where appropriate.

Empty states exist.

Do not leave dead controls unless explicitly marked unavailable.

============================================================
23. MOTION
============================================================

Use tasteful motion.

Recommended UI durations:

approximately 150–500ms.

Possible atmospheric animation:

- extremely slow clouds
- subtle background movement
- magical particles
- rune pulses
- parallax
- environmental image depth

Possible interface motion:

- cards rise slightly
- image zoom
- underline grows
- rune highlight appears
- drawer transition
- modal entrance
- accordion expansion
- page section reveal

Do not make the UI constantly move.

Implement prefers-reduced-motion support.

============================================================
24. RESPONSIVE DESIGN
============================================================

The application must be designed for:

Desktop
Laptop
Tablet
Mobile

Do not build desktop first and merely shrink everything.

Design proper mobile adaptations.

Important adaptations:

NAVIGATION
becomes a mobile drawer/menu.

HERO
intelligently crops artwork.

WIKI
left/right sidebars become drawers, accordions or contextual controls.

NEWS
featured layouts become vertical.

STORE
cards become fewer columns.

ARTICLE TYPOGRAPHY
maintains comfortable line lengths.

CTA BUTTONS
remain easily tappable.

============================================================
25. ACCESSIBILITY
============================================================

Implement:

semantic HTML

keyboard accessible navigation

visible focus states

labels

alt text

proper button elements

accessible modal/dialog behavior

appropriate ARIA when needed

sufficient contrast

reduced motion support

logical heading structure

Do not sacrifice accessibility for fantasy aesthetics.

============================================================
26. MOCK DATA
============================================================

Create strongly typed mock data.

Keep it outside feature templates/components.

Suggested files:

news.mock.ts
updates.mock.ts
wiki.mock.ts
store.mock.ts

Suggested interfaces:

NewsArticle
GameUpdate
UpdateSection
WikiCategory
WikiArticle
StoreItem

The future backend should be able to replace these data sources without requiring the UI to be rewritten.

============================================================
27. CONTENT STYLE
============================================================

Write short believable placeholder copy for Eivar Online.

Tone:

mysterious
adventurous
confident
world-building
not overly dramatic

Avoid:

"THE GREATEST MMORPG EVER CREATED"

Avoid fake statistics.

Avoid fake quotes.

Avoid fake awards.

Avoid fake release dates.

Avoid claiming unfinished features definitely exist.

The project is in active development.

Communicate that naturally.

============================================================
28. IMAGERY
============================================================

Use the supplied Eivar Online cover as a central visual reference and hero asset.

If other final artwork is unavailable, create visually appropriate placeholders with clear asset paths.

Do not embed random unrelated stock photography.

Placeholder visual themes should be named around:

floating-islands
ancient-ruins
city
mountains
runes
weapons
forest
battle
ancient-architecture

Create the layout so final game screenshots and artwork can later be dropped in without redesigning the components.

============================================================
29. DESIGN DETAILS
============================================================

Introduce small Eivar-specific details across the application.

Examples:

A small vertical rune used as active nav indicator.

Rune symbols between major homepage sections.

Cyan light appearing inside engraved lines on hover.

Thin gold horizontal lines near major headings.

Subtle mountain silhouettes in footer backgrounds.

Very faint ancient writing patterns behind Wiki sections.

Stone texture only around selected UI elements.

Banner-like shapes for category labels.

Do not use every effect simultaneously.

The interface should remain sophisticated.

============================================================
30. PERFORMANCE
============================================================

Even though this is a visual prototype, keep the frontend reasonable.

Use:

lazy loading where appropriate
optimized images
responsive images when possible
CSS rather than JavaScript for simple effects
limited expensive filters
reasonable animation usage

Avoid enormous UI libraries for simple features.

Avoid loading unnecessary code globally.

============================================================
31. CODE QUALITY
============================================================

Code should be production-like even though backend functionality is mocked.

Requirements:

- TypeScript types
- readable naming
- reusable UI
- small focused components
- no massive monolithic component
- no duplicated mock arrays
- no giant template with the entire site
- no inline style chaos
- consistent styling system
- no excessive !important
- useful comments only
- avoid dead code

============================================================
32. ROUTES
============================================================

At minimum implement:

/
/news
/news/:slug
/updates
/wiki
/wiki/:category
/wiki/:category/:slug
/store
/login

Optionally implement:

/game
/community

if they improve navigation architecture.

Do not create pointless placeholder routes.

============================================================
33. DEVELOPMENT PAGE STATES
============================================================

Create polished states for:

Loading
Empty
Error
Disabled
Coming Soon

These are PRESENTATION states only.

Examples:

Wiki search has no result.

Store section unavailable.

News list empty.

Article not found.

Do not build a backend around these states.

============================================================
34. OPTIONAL EIVAR LOADING INDICATOR
============================================================

Create a small Eivar loading indicator inspired by the rune above the EIVAR logo.

Use CSS animation if possible.

It should feel like an ancient symbol gradually becoming illuminated with cyan magical energy.

Keep it minimal.

============================================================
35. FOOTER
============================================================

Create a polished global footer.

Possible structure:

EIVAR ONLINE

GAME
Overview
News
Updates
Wiki

COMMUNITY
Discord
Community
Support

LEGAL
Terms
Privacy
Cookies

Include a project development disclaimer.

Add the small Eivar rune symbol.

The footer can use:

deep blue
mountain silhouette
extremely subtle runes
faint atmospheric gradient

============================================================
36. WHAT NOT TO DO
============================================================

DO NOT:

- build backend logic
- build authentication APIs
- add Firebase
- add Supabase
- add payment processing
- add Stripe
- invent server endpoints
- invent a database
- create fake player authentication
- overengineer state management
- use NgRx
- create a generic admin dashboard
- use excessive glassmorphism
- use excessive blur
- use excessive neon
- use every component as a rounded card
- make the whole site dark
- make the whole site parchment colored
- make every border gold
- use random gradients everywhere
- overcrowd pages
- create fake player statistics
- create fake reviews
- create fake launch dates
- create fake awards
- create huge lore dumps

============================================================
37. IMPLEMENTATION WORKFLOW
============================================================

Before implementing pages:

1. Inspect the existing Angular project.
2. Inspect all existing assets.
3. Read DESIGN.md completely.
4. Establish design tokens.
5. Establish typography.
6. Establish global layout.
7. Build reusable UI primitives.
8. Build navigation and footer.
9. Build Home.
10. Build News.
11. Build Updates.
12. Build Wiki.
13. Build Store.
14. Build Login.
15. Build Global Search.
16. Add responsive behavior.
17. Add motion.
18. Verify accessibility.
19. Verify routing.
20. Verify there are no dead interactions.

Do not start by generating hundreds of disconnected components.

Create a coherent system first.

============================================================
38. FINAL EXPERIENCE TARGET
============================================================

Opening the website should immediately communicate:

"This is Eivar Online."

The user should see the supplied cover artwork and understand that the website belongs to the same world.

Moving through the site should gradually introduce:

THE WORLD

THE COMBAT / WEAPON PHILOSOPHY

THE RUNIC LANGUAGE

THE DEVELOPMENT

THE COMMUNITY

THE KNOWLEDGE BASE

The site should feel like the official portal into Eivar rather than simply a promotional landing page.

The strongest visual themes should be:

SKY
STONE
RUNES
MAGIC
EXPLORATION
ANCIENT CIVILIZATIONS

Maintain visual consistency across every route.

============================================================
39. MOST IMPORTANT RULE
============================================================

Whenever you design a component or section, ask:

"Would this UI still make sense visually if it were shown directly beside the supplied Eivar Online cover artwork?"

If it feels like it came from another game, another website template, a SaaS product, or a generic component library:

REDESIGN IT.

Do not sacrifice usability for decoration.

Eivar Online must feel unique because of composition, materials, typography, runes, atmosphere and environmental imagery — not because every component has excessive fantasy decoration.

Now inspect the project and supplied assets, create the design foundation, and implement the complete frontend prototype.
