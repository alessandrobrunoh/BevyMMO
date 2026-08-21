import { Component, EventEmitter, Input, Output } from '@angular/core';

@Component({
  selector: 'app-eivar-pagination',
  standalone: true,
  template: `
    @if (totalPages > 1) {
      <nav class="eivar-pagination" aria-label="Pagination">
        <button type="button" class="eivar-pagination__arrow" [disabled]="currentPage === 1" aria-label="Previous page" (click)="selectPage(currentPage - 1)">
          <span aria-hidden="true">‹</span>
        </button>
        @for (item of visibleItems(); track $index) {
          @if (item === 'ellipsis') {
            <span class="eivar-pagination__ellipsis" aria-hidden="true">…</span>
          } @else {
            <button
              type="button"
              class="eivar-pagination__page"
              [class.is-current]="item === currentPage"
              [attr.aria-current]="item === currentPage ? 'page' : null"
              [attr.aria-label]="'Page ' + item"
              (click)="selectPage(item)"
            >{{ item }}</button>
          }
        }
        <button type="button" class="eivar-pagination__arrow" [disabled]="currentPage === totalPages" aria-label="Next page" (click)="selectPage(currentPage + 1)">
          <span aria-hidden="true">›</span>
        </button>
      </nav>
    }
  `,
  styleUrls: ['./pagination.component.scss']
})
export class EivarPaginationComponent {
  @Input() currentPage = 1;
  @Input() totalPages = 1;
  @Output() pageChange = new EventEmitter<number>();

  visibleItems(): Array<number | 'ellipsis'> {
    if (this.totalPages <= 5) {
      return Array.from({ length: this.totalPages }, (_, index) => index + 1);
    }

    if (this.currentPage <= 3) {
      return [1, 2, 3, 'ellipsis', this.totalPages];
    }

    if (this.currentPage >= this.totalPages - 2) {
      return [1, 'ellipsis', this.totalPages - 2, this.totalPages - 1, this.totalPages];
    }

    return [1, 'ellipsis', this.currentPage, 'ellipsis', this.totalPages];
  }

  selectPage(page: number): void {
    if (page >= 1 && page <= this.totalPages && page !== this.currentPage) {
      this.pageChange.emit(page);
    }
  }
}
