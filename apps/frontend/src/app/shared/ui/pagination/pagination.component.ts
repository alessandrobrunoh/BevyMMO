import { Component, EventEmitter, Input, Output } from '@angular/core';
import { EivarButtonComponent } from '../button/button.component';

@Component({
  selector: 'app-eivar-pagination',
  standalone: true,
  imports: [EivarButtonComponent],
  template: `
    @if (totalPages > 1) {
      <nav class="eivar-pagination" aria-label="Pagination">
        <app-eivar-button variant="arrow-left" class="eivar-pagination__arrow" [iconOnly]="true" [disabled]="currentPage === 1" ariaLabel="Previous page" (onClick)="selectPage(currentPage - 1)"></app-eivar-button>
        @for (item of visibleItems(); track $index) {
          @if (item === 'ellipsis') {
            <span class="eivar-pagination__ellipsis" aria-hidden="true">…</span>
          } @else {
            <app-eivar-button
              variant="icon-square"
              tone="gold"
              class="eivar-pagination__page"
              [iconOnly]="true"
              [active]="item === currentPage"
              [toggle]="true"
              [ariaCurrent]="item === currentPage ? 'page' : undefined"
              [ariaLabel]="'Page ' + item"
              (onClick)="selectPage(item)"
            >{{ item }}</app-eivar-button>
          }
        }
        <app-eivar-button variant="arrow-right" class="eivar-pagination__arrow" [iconOnly]="true" [disabled]="currentPage === totalPages" ariaLabel="Next page" (onClick)="selectPage(currentPage + 1)"></app-eivar-button>
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
