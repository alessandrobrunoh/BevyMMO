import { Injectable } from '@angular/core';
import { Observable, of } from 'rxjs';
import { NewsArticle, NewsCategory } from '../../shared/models/news.model';
import { GameUpdate } from '../../shared/models/update.model';
import { WikiArticle, WikiCategory } from '../../shared/models/wiki.model';
import { StoreItem, StoreCategory } from '../../shared/models/store.model';

import { MOCK_NEWS_ARTICLES } from '../../data/mocks/news.mock';
import { MOCK_GAME_UPDATES } from '../../data/mocks/updates.mock';
import { MOCK_WIKI_CATEGORIES, MOCK_WIKI_ARTICLES } from '../../data/mocks/wiki.mock';
import { MOCK_STORE_ITEMS } from '../../data/mocks/store.mock';

@Injectable({
  providedIn: 'root'
})
export class ContentService {
  // News Methods
  getNewsArticles(category?: NewsCategory | 'All'): Observable<NewsArticle[]> {
    if (!category || category === 'All') {
      return of(MOCK_NEWS_ARTICLES);
    }
    return of(MOCK_NEWS_ARTICLES.filter(a => a.category === category));
  }

  getNewsArticleBySlug(slug: string): Observable<NewsArticle | undefined> {
    const article = MOCK_NEWS_ARTICLES.find(a => a.slug === slug);
    return of(article);
  }

  getFeaturedNews(): Observable<NewsArticle | undefined> {
    const featured = MOCK_NEWS_ARTICLES.find(a => a.featured) || MOCK_NEWS_ARTICLES[0];
    return of(featured);
  }

  // Updates Methods
  getGameUpdates(type?: 'Development' | 'Patch Notes' | 'All'): Observable<GameUpdate[]> {
    if (!type || type === 'All') {
      return of(MOCK_GAME_UPDATES);
    }
    return of(MOCK_GAME_UPDATES.filter(u => u.type === type));
  }

  getLatestUpdate(): Observable<GameUpdate> {
    return of(MOCK_GAME_UPDATES[0]);
  }

  // Wiki Methods
  getWikiCategories(): Observable<WikiCategory[]> {
    return of(MOCK_WIKI_CATEGORIES);
  }

  getWikiCategoryBySlug(slug: string): Observable<WikiCategory | undefined> {
    return of(MOCK_WIKI_CATEGORIES.find(c => c.slug === slug));
  }

  getWikiArticles(categorySlug?: string): Observable<WikiArticle[]> {
    if (!categorySlug) {
      return of(MOCK_WIKI_ARTICLES);
    }
    return of(MOCK_WIKI_ARTICLES.filter(a => a.categorySlug === categorySlug));
  }

  getWikiArticleBySlug(categorySlug: string, slug: string): Observable<WikiArticle | undefined> {
    const article = MOCK_WIKI_ARTICLES.find(
      a => a.categorySlug === categorySlug && a.slug === slug
    );
    return of(article);
  }

  // Store Methods
  getStoreItems(category?: StoreCategory | 'All'): Observable<StoreItem[]> {
    if (!category || category === 'All') {
      return of(MOCK_STORE_ITEMS);
    }
    return of(MOCK_STORE_ITEMS.filter(item => item.category === category));
  }

  getStoreItemById(id: string): Observable<StoreItem | undefined> {
    return of(MOCK_STORE_ITEMS.find(i => i.id === id));
  }
}
