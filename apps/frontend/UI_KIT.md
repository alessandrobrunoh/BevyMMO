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

`app-eivar-button` is the standard action control. It is keyboard accessible and exposes the native button's `type`, `disabled` and loading state.

| Variant | Use |
| --- | --- |
| `primary` / `gold` | Main action: dark metal with warm gold activation |
| `secondary` / `navigation` | Header and compact navigation actions |
| `outline` | Low-emphasis action on a dark surface |
| `tag` | Filter and selectable tag, set `[active]="true"` when selected |
| `cta` | Large hero or campaign action |
| `icon-square` | Compact utility icon |
| `icon-circle` | Circular rune action |
| `social` | Social destination icon |
| `cyan` | Reserved for magic or Essence interactions |
| `ghost` | Text-only tertiary action |

```html
<app-eivar-button variant="tag" [active]="isSelected" (onClick)="select()">
  Weapons
</app-eivar-button>

<app-eivar-button variant="icon-circle" icon="search" ariaLabel="Search"></app-eivar-button>
```

Use `iconSet="glyph"` for a rune or character instead of a Material Symbol. Use `[iconOnly]="true"` with an `ariaLabel` whenever the icon has no visible text.

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
