import { Component, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { ContentService } from '../../../core/services/content.service';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { EivarCardComponent } from '../../../shared/ui/card/card.component';
import { NewsArticle, NewsCategory } from '../../../shared/models/news.model';

@Component({
  selector: 'app-news-list',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent, EivarButtonComponent, EivarCardComponent],
  templateUrl: './news-list.component.html',
  styleUrls: ['./news-list.component.scss']
})
export class NewsListComponent {
  private contentService = inject(ContentService);

  readonly articles = signal<NewsArticle[]>([]);
  readonly activeCategory = signal<NewsCategory | 'All'>('All');
  readonly searchQuery = signal<string>('');

  readonly categories: (NewsCategory | 'All')[] = [
    'All',
    'Announcements',
    'Development',
    'Community',
    'Events'
  ];

  readonly filteredArticles = computed(() => {
    let list = this.articles();
    const cat = this.activeCategory();
    const q = this.searchQuery().trim().toLowerCase();

    if (cat !== 'All') {
      list = list.filter(a => a.category === cat);
    }

    if (q) {
      list = list.filter(a =>
        a.title.toLowerCase().includes(q) ||
        a.excerpt.toLowerCase().includes(q) ||
        a.tags.some(t => t.toLowerCase().includes(q))
      );
    }

    return list;
  });

  constructor() {
    this.contentService.getNewsArticles().subscribe(arts => {
      this.articles.set(arts);
    });
  }

  setCategory(category: NewsCategory | 'All') {
    this.activeCategory.set(category);
  }

  onSearchInput(event: Event) {
    const val = (event.target as HTMLInputElement).value;
    this.searchQuery.set(val);
  }
}
