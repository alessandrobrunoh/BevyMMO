import { Injectable, inject, signal } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { NewsArticle, NewsCategory } from '../../shared/models/news.model';
import { GameUpdate } from '../../shared/models/update.model';
import { WikiArticle, WikiCategory } from '../../shared/models/wiki.model';
import { StoreItem, StoreCategory } from '../../shared/models/store.model';
import { Catalog } from '../../shared/models/catalog.model';
import { API_BASE_URL } from '../config/api.config';
import { mergeWikiContent } from '../../features/wiki/wiki-from-catalog';
import { LORE_ARTICLES, LORE_CATEGORY_DEFS } from '../../data/wiki-lore';

import { MOCK_NEWS_ARTICLES } from '../../data/mocks/news.mock';
import { MOCK_GAME_UPDATES } from '../../data/mocks/updates.mock';
import { MOCK_STORE_ITEMS } from '../../data/mocks/store.mock';

@Injectable({
  providedIn: 'root'
})
export class ContentService {
  private http = inject(HttpClient);

  readonly wikiArticles = signal<WikiArticle[]>(LORE_ARTICLES);
  readonly wikiCategories = signal<WikiCategory[]>(loreCategories());
  readonly catalogError = signal<string | null>(null);
  readonly catalogLoaded = signal(false);

  constructor() {
    this.loadCatalog();
  }

  private loadCatalog() {
    this.http.get<Catalog>(`${API_BASE_URL}/public/catalog`).subscribe({
      next: catalog => {
        const merged = mergeWikiContent(catalog.items ?? []);
        this.wikiArticles.set(merged.articles);
        this.wikiCategories.set(merged.categories);
        this.catalogError.set(null);
        this.catalogLoaded.set(true);
      },
      error: err => {
        this.wikiArticles.set(LORE_ARTICLES);
        this.wikiCategories.set(loreCategories());
        this.catalogError.set(describeCatalogError(err));
        this.catalogLoaded.set(true);
      }
    });
  }

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

  getWikiCategories(): Observable<WikiCategory[]> {
    return of(this.wikiCategories());
  }

  getWikiCategoryBySlug(slug: string): Observable<WikiCategory | undefined> {
    return of(this.wikiCategories().find(c => c.slug === slug));
  }

  getWikiArticles(categorySlug?: string): Observable<WikiArticle[]> {
    const articles = this.wikiArticles();
    if (!categorySlug) {
      return of(articles);
    }
    return of(articles.filter(a => a.categorySlug === categorySlug));
  }

  getWikiArticleBySlug(categorySlug: string, slug: string): Observable<WikiArticle | undefined> {
    return of(
      this.wikiArticles().find(a => a.categorySlug === categorySlug && a.slug === slug)
    );
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

function loreCategories(): WikiCategory[] {
  return LORE_CATEGORY_DEFS.map(def => ({
    ...def,
    articleCount: LORE_ARTICLES.filter(article => article.categorySlug === def.slug).length
  }));
}

function describeCatalogError(err: unknown): string {
  if (!(err instanceof HttpErrorResponse)) {
    return err instanceof Error ? err.message : 'Could not load the game catalog.';
  }
  if (err.status === 0 || err.status === 200) {
    return 'The gateway is not reachable at /v1. Start the gateway on :8081 and use `npm start` (proxies /v1).';
  }
  const body = err.error as { error?: unknown } | string | null;
  if (typeof body === 'object' && body && typeof body.error === 'string') {
    return body.error;
  }
  return `Request to /public/catalog failed (${err.status}).`;
}
