# Eivar UI kit

The reusable controls live under `src/app/shared/ui`. They reproduce the dark metal, burnished-gold and clipped-corner language from the approved component sheet. Every interactive state is implemented in CSS, not as a static image.

## Card

`app-eivar-card` is the reusable content surface for cards that need the parchment frame from the supplied component sheet. It combines the inset paper surface, gold-and-cyan corners, framed media, and the central media notch. It is intentionally presentation-only: place it inside an `<a>` or router-linked `<article>` when the entire card navigates.

```html
<app-eivar-card
  image="assets/images/world-exploration.jpg"
  imageAlt="A valley in Eivar"
  badge="Featured"
  eyebrow="May 20, 2026 · 4 min read"
  title="Explore the northern highlands"
  description="Ancient paths and forgotten citadels await."
  mediaRatio="wide"
  [interactive]="true"
>
  <div card-footer>
    <span>Read more →</span>
  </div>
</app-eivar-card>
```

| Input | Values | Use |
| --- | --- | --- |
| `theme` | `parchment` (default), `runic` | Light content/news cards or dark game-system cards. |
| `layout` | `vertical` (default), `horizontal`, `auto` | `auto` displays media beside content when its own container is wide enough, otherwise stacks it. |
| `mediaRatio` | `wide`, `standard` (default), `portrait` | Preserves the intended visual balance for arbitrary image dimensions. |
| `compact` | boolean | Reduces content padding. |
| `interactive` | boolean | Adds hover feedback; navigation/click handling remains on the parent semantic element. |

Pass extra content through `[card-body]` and actions or metadata through `[card-footer]`. The component uses container queries, so it remains readable in narrow sidebars as well as wide grid columns.

## Auth shell

`app-auth-shell` supplies the shared visual structure for the full-screen authentication pages: atmospheric world backdrop, brand/lore panel, crystal-and-gold form frame, and responsive single-column fallback. It keeps each form responsible for its own submission and validation.

```html
<app-auth-shell mode="login" title="Enter Eivar" subtitle="Continue your journey.">
  <form><!-- labelled controls and submit action --></form>
</app-auth-shell>
```

The active routes are `/login` and `/register`. Email/password controls use their respective browser autofill tokens (`email`, `current-password`, `new-password`) and expose failures through an assertive live alert.

## Button

`app-eivar-button` is the only button-like control used by the site. Its visual surfaces are extracted from the approved `design-assets/sheets/buttons.png` sheet, while the component keeps native HTML semantics: actions render a `<button>`, Angular navigation renders an `<a>` with `RouterLink`, and external destinations render an `<a href>`.

| Variant | Use |
| --- | --- |
| `primary` / `gold` | Main warm-gold action |
| `cta` | Large hero or campaign action |
| `secondary` / `navigation` | Lower-emphasis and navigation actions |
| `cyan` / `info` / `ornate` | Blue magical, market, and auth actions |
| `success` | Positive or confirmatory action |
| `danger` | Destructive or sign-out action |
| `tag` | Filter or selectable option; pair with `[toggle]="true"` and `[active]` |
| `icon-square` / `icon-circle` | Compact icon-only utility action |
| `arrow-left` / `arrow-right` | Pagination and directional navigation |
| `social` | Social destination icon |
| `outline` / `ghost` | Tertiary action using the dark engraved frame |

The available `tone` values are `blue`, `green`, `red`, and `gold`. They select the matching extracted sheet family for `tag`, square, circle, and social controls. Sizes are `sm`, `md`, and `lg`.

```html
<app-eivar-button
  variant="tag"
  size="sm"
  tone="gold"
  [toggle]="true"
  [active]="isSelected"
  (onClick)="select()"
>
  Weapons
</app-eivar-button>

<app-eivar-button
  variant="icon-circle"
  [iconOnly]="true"
  icon="search"
  ariaLabel="Search"
  (onClick)="openSearch()"
></app-eivar-button>

<app-eivar-button variant="secondary" routerLink="/news">
  View all news
</app-eivar-button>
```

Interactive visuals are mapped to real CSS states: default, `:hover`, `:active`, `.is-active`, and disabled. Keyboard focus uses a separate high-contrast `:focus-visible` ring, loading sets `aria-busy`, and reduced-motion users do not receive transitions or spinner animation. Use `iconSet="glyph"` for a rune or character instead of a Material Symbol. Always provide `ariaLabel` with `[iconOnly]="true"`.

The 77 transparent runtime assets live in `public/assets/ui/buttons`. Regenerate them from the source sheet from `apps/frontend` with:

```sh
python3 design-assets/extract-buttons.py
```

## Checkbox, radio and switch

`app-eivar-selection-control` supports `(checkedChange)`, `[(ngModel)]` and reactive forms through `ControlValueAccessor`.

```html
<app-eivar-selection-control kind="checkbox" label="Remember this device" [(ngModel)]="remember" />
<app-eivar-selection-control kind="radio" name="realm" label="Eivar" [checked]="true" />
<app-eivar-selection-control kind="switch" label="Ambient runes" [(ngModel)]="runesEnabled" />
```

A radio group shares the same `name`; the host application owns the exclusive selection state.

## Pagination

```html
<app-eivar-pagination
  [currentPage]="page"
  [totalPages]="pageCount"
  (pageChange)="page = $event"
/>
```

The component renders only the useful leading, current and trailing pages, with ellipses for skipped ranges.
