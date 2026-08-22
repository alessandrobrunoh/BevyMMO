import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router } from '@angular/router';
import { ContentService } from '../../core/services/content.service';
import { ToastService } from '../../core/services/toast.service';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { RuneDividerComponent } from '../../shared/ui/rune-divider/rune-divider.component';
import { SectionHeadingComponent } from '../../shared/ui/section-heading/section-heading.component';
import { NewsArticle } from '../../shared/models/news.model';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    EivarButtonComponent
  ],
  templateUrl: './home.component.html',
  styleUrls: ['./home.component.scss']
})
export class HomeComponent {
  private contentService = inject(ContentService);
  private toastService = inject(ToastService);
  private router = inject(Router);

  featuredNews = signal<NewsArticle | undefined>(undefined);
  recentNews = signal<NewsArticle[]>([]);

  constructor() {
    this.contentService.getNewsArticles().subscribe(articles => {
      this.featuredNews.set(articles[0]);
      this.recentNews.set(articles.slice(1, 4));
    });
  }

  scrollToSection(id: string) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth' });
    }
  }

  onJoinDiscord() {
    this.toastService.showInfo('Official Eivar Discord invite opened in prototype mode.', 'Eivar Community');
  }

  onLearnMore() {
    this.router.navigate(['/updates']);
  }

  goToNews(): void {
    this.router.navigate(['/news']);
  }
}
