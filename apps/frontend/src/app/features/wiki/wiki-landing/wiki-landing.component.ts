import { Component, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router } from '@angular/router';
import { ContentService } from '../../../core/services/content.service';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { RuneDividerComponent } from '../../../shared/ui/rune-divider/rune-divider.component';
import { WikiCategory, WikiArticle } from '../../../shared/models/wiki.model';

@Component({
  selector: 'app-wiki-landing',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent, RuneDividerComponent],
  templateUrl: './wiki-landing.component.html',
  styleUrls: ['./wiki-landing.component.scss']
})
export class WikiLandingComponent {
  private contentService = inject(ContentService);
  private router = inject(Router);

  readonly categories = signal<WikiCategory[]>([]);
  readonly popularArticles = signal<WikiArticle[]>([]);
  readonly searchFilter = signal<string>('');

  readonly filteredCategories = computed(() => {
    const q = this.searchFilter().trim().toLowerCase();
    if (!q) return this.categories();
    return this.categories().filter(c =>
      c.name.toLowerCase().includes(q) ||
      c.description.toLowerCase().includes(q)
    );
  });

  constructor() {
    this.contentService.getWikiCategories().subscribe(cats => {
      this.categories.set(cats);
    });

    this.contentService.getWikiArticles().subscribe(arts => {
      this.popularArticles.set(arts.slice(0, 3));
    });
  }

  onSearch(event: Event) {
    const val = (event.target as HTMLInputElement).value;
    this.searchFilter.set(val);
  }
}
