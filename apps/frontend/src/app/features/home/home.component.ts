import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router } from '@angular/router';
import { ContentService } from '../../core/services/content.service';
import { ToastService } from '../../core/services/toast.service';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { RuneDividerComponent } from '../../shared/ui/rune-divider/rune-divider.component';
import { SectionHeadingComponent } from '../../shared/ui/section-heading/section-heading.component';
import { RuneSlotComponent } from '../../shared/ui/rune-slot/rune-slot.component';
import { NewsArticle } from '../../shared/models/news.model';

interface WeaponSlotPreview {
  slot: 'Q' | 'W' | 'E';
  name: string;
  type: string;
  essence: string;
  essenceRune: string;
  modifiers: { name: string; rune: string }[];
  ancientWord?: { name: string; rune: string };
  effectSummary: string;
}

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    EivarButtonComponent,
    RuneDividerComponent,
    SectionHeadingComponent,
    RuneSlotComponent
  ],
  templateUrl: './home.component.html',
  styleUrls: ['./home.component.scss']
})
export class HomeComponent {
  private contentService = inject(ContentService);
  private toastService = inject(ToastService);
  private router = inject(Router);

  // Latest news data
  featuredNews = signal<NewsArticle | undefined>(undefined);
  recentNews = signal<NewsArticle[]>([]);

  // Interactive Weapon Showcase State
  readonly activeSlot = signal<'Q' | 'W' | 'E'>('E');

  readonly weaponSlots: Record<'Q' | 'W' | 'E', WeaponSlotPreview> = {
    Q: {
      slot: 'Q',
      name: 'Arcane Orb',
      type: 'Linear Kinetic Blast',
      essence: 'Fire Essence (Sunfire)',
      essenceRune: 'ᚠ',
      modifiers: [{ name: 'Expand (+40% AoE)', rune: 'ᚱ' }],
      effectSummary: 'Hurls a searing sphere of solar flame that bursts on contact, dealing explosive fire damage over an enlarged blast radius.'
    },
    W: {
      slot: 'W',
      name: 'Runic Barrier',
      type: 'Protective Prismatic Ward',
      essence: 'Life Essence (Verdant Flow)',
      essenceRune: 'ᛉ',
      modifiers: [{ name: 'Persistence (+3.5s Duration)', rune: 'ᚱ' }],
      effectSummary: 'Erects a towering stationary rune gate that absorbs hostile projectiles and radiates continuous restorative healing pulses to allies behind it.'
    },
    E: {
      slot: 'E',
      name: 'Great Impact',
      type: 'Ground Cataclysmic Shockwave',
      essence: 'Life & Arcane Resonance',
      essenceRune: 'ᛟ',
      modifiers: [
        { name: 'Expand (+50% Radius)', rune: 'ᚱ' },
        { name: 'Persistence (+4s Ground Rupture)', rune: 'ᚱ' }
      ],
      ancientWord: { name: 'Echo', rune: 'ᛟ' },
      effectSummary: 'Strikes the staff to the terrain, fracturing the earth and detonating an initial shockwave, followed 1.5s later by an identical Echo shockwave.'
    }
  };

  constructor() {
    this.contentService.getNewsArticles().subscribe(articles => {
      this.featuredNews.set(articles[0]);
      this.recentNews.set(articles.slice(1, 4));
    });
  }

  selectSlot(slot: 'Q' | 'W' | 'E') {
    this.activeSlot.set(slot);
  }

  scrollToSection(id: string) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth' });
    }
  }

  onJoinPlaytest() {
    this.toastService.showRunic('Alpha registration requested. Directing to account portal...', 'Alpha Access');
    this.router.navigate(['/login']);
  }
}
