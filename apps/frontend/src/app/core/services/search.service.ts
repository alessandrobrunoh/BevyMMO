import { Injectable, inject, signal, computed } from '@angular/core';
import { ContentService } from './content.service';
import { MOCK_NEWS_ARTICLES } from '../../data/mocks/news.mock';
import { MOCK_GAME_UPDATES } from '../../data/mocks/updates.mock';
import { articleSearchText } from '../../features/wiki/wiki-from-catalog';

export interface SearchResultItem {
  id: string;
  type: 'WIKI' | 'NEWS' | 'UPDATE';
  title: string;
  subtitle: string;
  route: string[];
  tags: string[];
}

@Injectable({
  providedIn: 'root'
})
export class SearchService {
  private contentService = inject(ContentService);

  readonly isOpen = signal<boolean>(false);
  readonly query = signal<string>('');

  readonly results = computed<SearchResultItem[]>(() => {
    const q = this.query().trim().toLowerCase();
    if (!q) return [];

    const list: SearchResultItem[] = [];

    // Search Wiki Articles
    for (const art of this.contentService.wikiArticles()) {
      if (articleSearchText(art).includes(q)) {
        list.push({
          id: art.id,
          type: 'WIKI',
          title: art.title,
          subtitle: `${art.categoryName} · ${art.subtitle || 'Wiki Entry'}`,
          route: ['/wiki', art.categorySlug, art.slug],
          tags: [art.categoryName]
        });
      }
    }

    // Search News Articles
    for (const news of MOCK_NEWS_ARTICLES) {
      if (
        news.title.toLowerCase().includes(q) ||
        news.excerpt.toLowerCase().includes(q) ||
        news.tags.some(t => t.toLowerCase().includes(q))
      ) {
        list.push({
          id: news.id,
          type: 'NEWS',
          title: news.title,
          subtitle: `${news.category} · ${news.publishedAt}`,
          route: ['/news', news.slug],
          tags: news.tags
        });
      }
    }

    // Search Updates
    for (const upd of MOCK_GAME_UPDATES) {
      if (
        upd.version.toLowerCase().includes(q) ||
        upd.title.toLowerCase().includes(q) ||
        upd.summary.toLowerCase().includes(q)
      ) {
        list.push({
          id: upd.id,
          type: 'UPDATE',
          title: `${upd.version}: ${upd.title}`,
          subtitle: `${upd.type} · ${upd.date}`,
          route: ['/updates'],
          tags: [upd.version, upd.type]
        });
      }
    }

    return list;
  });

  open() {
    this.isOpen.set(true);
  }

  close() {
    this.isOpen.set(false);
    this.query.set('');
  }

  setQuery(text: string) {
    this.query.set(text);
  }
}
