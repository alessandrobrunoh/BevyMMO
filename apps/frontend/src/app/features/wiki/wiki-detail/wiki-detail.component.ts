import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterModule } from '@angular/router';
import { ContentService } from '../../../core/services/content.service';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { WikiInfoBoxComponent } from '../../../shared/ui/wiki-infobox/wiki-infobox.component';
import { WikiCalloutComponent } from '../../../shared/ui/wiki-callout/wiki-callout.component';
import { AbilityCardComponent } from '../../../shared/ui/ability-card/ability-card.component';
import { RuneDividerComponent } from '../../../shared/ui/rune-divider/rune-divider.component';
import { WikiArticle, WikiCategory } from '../../../shared/models/wiki.model';

@Component({
  selector: 'app-wiki-detail',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    PageHeaderComponent,
    EivarButtonComponent,
    WikiInfoBoxComponent,
    WikiCalloutComponent,
    AbilityCardComponent,
    RuneDividerComponent
  ],
  templateUrl: './wiki-detail.component.html',
  styleUrls: ['./wiki-detail.component.scss']
})
export class WikiDetailComponent {
  private route = inject(ActivatedRoute);
  private contentService = inject(ContentService);

  readonly article = signal<WikiArticle | undefined>(undefined);
  readonly categories = signal<WikiCategory[]>([]);
  readonly categoryArticles = signal<WikiArticle[]>([]);
  readonly currentCategory = signal<WikiCategory | undefined>(undefined);

  constructor() {
    this.contentService.getWikiCategories().subscribe(cats => {
      this.categories.set(cats);
    });

    this.route.paramMap.subscribe(params => {
      const catSlug = params.get('category') || 'weapons';
      const slug = params.get('slug') || (catSlug === 'weapons' ? 'channeling-staff' : undefined);

      this.contentService.getWikiCategoryBySlug(catSlug).subscribe(cat => {
        this.currentCategory.set(cat);
      });

      this.contentService.getWikiArticles(catSlug).subscribe(articles => {
        this.categoryArticles.set(articles);
        if (slug) {
          const found = articles.find(a => a.slug === slug);
          this.article.set(found || articles[0]);
        } else {
          this.article.set(articles[0]);
        }
      });
    });
  }

  scrollToSection(id: string) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth' });
    }
  }
}
