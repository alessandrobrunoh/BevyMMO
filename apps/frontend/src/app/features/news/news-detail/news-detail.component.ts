import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterModule, Router } from '@angular/router';
import { ContentService } from '../../../core/services/content.service';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { RuneDividerComponent } from '../../../shared/ui/rune-divider/rune-divider.component';
import { NewsArticle } from '../../../shared/models/news.model';

@Component({
  selector: 'app-news-detail',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent, EivarButtonComponent, RuneDividerComponent],
  templateUrl: './news-detail.component.html',
  styleUrls: ['./news-detail.component.scss']
})
export class NewsDetailComponent {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private contentService = inject(ContentService);

  readonly article = signal<NewsArticle | undefined>(undefined);
  readonly relatedArticles = signal<NewsArticle[]>([]);

  constructor() {
    this.route.paramMap.subscribe(params => {
      const slug = params.get('slug');
      if (slug) {
        this.contentService.getNewsArticleBySlug(slug).subscribe(art => {
          this.article.set(art);
          if (art) {
            this.contentService.getNewsArticles().subscribe(all => {
              this.relatedArticles.set(all.filter(a => a.id !== art.id).slice(0, 2));
            });
          }
        });
      }
    });
  }
}
