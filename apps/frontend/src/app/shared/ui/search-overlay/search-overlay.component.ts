import { Component, inject, ElementRef, ViewChild, AfterViewInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { EivarButtonComponent } from '../button/button.component';
import { SearchService, SearchResultItem } from '../../../core/services/search.service';

@Component({
  selector: 'app-search-overlay',
  standalone: true,
  imports: [CommonModule, EivarButtonComponent],
  template: `
    @if (searchService.isOpen()) {
      <div class="search-overlay-backdrop" (click)="onBackdropClick($event)">
        <div class="search-modal chamfer-box">
          <!-- Search Input Bar -->
          <div class="search-input-wrapper">
            <span class="material-symbols-outlined search-icon">search</span>
            <input
              #searchInput
              type="text"
              class="search-input"
              placeholder="Search the Archives, News, Patch Notes (e.g. 'spada', 'alpha', 'wood')..."
              [value]="searchService.query()"
              (input)="onInputChange($event)"
              (keydown.escape)="searchService.close()"
            />
            <app-eivar-button variant="tag" size="sm" class="close-search-btn" (onClick)="searchService.close()">
              ESC
            </app-eivar-button>
          </div>

          <!-- Quick category pills -->
          <div class="quick-tags">
            <span class="tag-label">QUICK JUMP:</span>
            <app-eivar-button variant="tag" size="sm" class="quick-tag" (onClick)="searchService.setQuery('spada')">Spada</app-eivar-button>
            <app-eivar-button variant="tag" size="sm" class="quick-tag" (onClick)="searchService.setQuery('wood')">Wood</app-eivar-button>
            <app-eivar-button variant="tag" size="sm" class="quick-tag" (onClick)="searchService.setQuery('combat')">Combat</app-eivar-button>
            <app-eivar-button variant="tag" size="sm" class="quick-tag" (onClick)="searchService.setQuery('alpha')">Alpha 0.2.1</app-eivar-button>
          </div>

          <!-- Search Results List -->
          <div class="search-results">
            @if (searchService.query() && searchService.results().length === 0) {
              <div class="empty-results">
                <span class="empty-rune">ᚹ</span>
                <p class="empty-title">No ancient records found for "{{ searchService.query() }}"</p>
                <p class="empty-desc">Try searching for weapons, essences, abilities, or patch notes.</p>
              </div>
            }

            @for (result of searchService.results(); track result.id) {
              <div class="search-result-item" (click)="navigateTo(result)">
                <div class="result-badge" [ngClass]="result.type.toLowerCase()">
                  {{ result.type }}
                </div>
                <div class="result-info">
                  <h4 class="result-title">{{ result.title }}</h4>
                  <p class="result-subtitle">{{ result.subtitle }}</p>
                </div>
                <span class="arrow-icon">→</span>
              </div>
            }
          </div>
        </div>
      </div>
    }
  `,
  styleUrls: ['./search-overlay.component.scss']
})
export class SearchOverlayComponent implements AfterViewInit {
  searchService = inject(SearchService);
  private router = inject(Router);

  @ViewChild('searchInput') searchInput?: ElementRef<HTMLInputElement>;

  ngAfterViewInit() {
    // Auto focus when overlay opens
  }

  onInputChange(event: Event) {
    const val = (event.target as HTMLInputElement).value;
    this.searchService.setQuery(val);
  }

  onBackdropClick(event: MouseEvent) {
    if ((event.target as HTMLElement).classList.contains('search-overlay-backdrop')) {
      this.searchService.close();
    }
  }

  navigateTo(result: SearchResultItem) {
    this.searchService.close();
    this.router.navigate(result.route);
  }
}
