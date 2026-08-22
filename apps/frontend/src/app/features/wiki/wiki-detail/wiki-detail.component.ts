import { Component, inject, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterModule } from '@angular/router';
import { ContentService } from '../../../core/services/content.service';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { WikiInfoBoxComponent } from '../../../shared/ui/wiki-infobox/wiki-infobox.component';
import { WikiCalloutComponent } from '../../../shared/ui/wiki-callout/wiki-callout.component';
import { AbilityCardComponent } from '../../../shared/ui/ability-card/ability-card.component';
import { RuneDividerComponent } from '../../../shared/ui/rune-divider/rune-divider.component';

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

  private readonly routeParams = toSignal(this.route.paramMap);

  readonly categories = this.contentService.wikiCategories;
  readonly catalogError = this.contentService.catalogError;
  readonly catalogLoaded = this.contentService.catalogLoaded;

  readonly currentCategory = computed(() => {
    const slug = this.routeParams()?.get('category') || 'weapons';
    return this.categories().find(cat => cat.slug === slug);
  });

  readonly categoryArticles = computed(() => {
    const slug = this.routeParams()?.get('category') || 'weapons';
    return this.contentService.wikiArticles().filter(article => article.categorySlug === slug);
  });

  readonly article = computed(() => {
    const articles = this.categoryArticles();
    const slug = this.routeParams()?.get('slug');
    if (slug) {
      return articles.find(article => article.slug === slug);
    }
    return articles[0];
  });

  scrollToSection(id: string) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth' });
    }
  }
}
