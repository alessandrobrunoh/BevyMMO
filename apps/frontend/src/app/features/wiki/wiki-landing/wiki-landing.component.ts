import { Component, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { ContentService } from '../../../core/services/content.service';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { RuneDividerComponent } from '../../../shared/ui/rune-divider/rune-divider.component';

@Component({
  selector: 'app-wiki-landing',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent, RuneDividerComponent],
  templateUrl: './wiki-landing.component.html',
  styleUrls: ['./wiki-landing.component.scss']
})
export class WikiLandingComponent {
  private contentService = inject(ContentService);

  readonly categories = this.contentService.wikiCategories;
  readonly catalogError = this.contentService.catalogError;
  readonly searchFilter = signal<string>('');

  readonly popularArticles = computed(() => this.contentService.wikiArticles().slice(0, 3));

  readonly filteredCategories = computed(() => {
    const q = this.searchFilter().trim().toLowerCase();
    if (!q) return this.categories();
    return this.categories().filter(c =>
      c.name.toLowerCase().includes(q) ||
      c.description.toLowerCase().includes(q)
    );
  });

  onSearch(event: Event) {
    const val = (event.target as HTMLInputElement).value;
    this.searchFilter.set(val);
  }
}
