import { Component, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ContentService } from '../../core/services/content.service';
import { ToastService } from '../../core/services/toast.service';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { ModalComponent } from '../../shared/ui/modal/modal.component';
import { StoreItem, StoreCategory } from '../../shared/models/store.model';

@Component({
  selector: 'app-store',
  standalone: true,
  imports: [CommonModule, PageHeaderComponent, EivarButtonComponent, ModalComponent],
  templateUrl: './store.component.html',
  styleUrls: ['./store.component.scss']
})
export class StoreComponent {
  private contentService = inject(ContentService);
  private toastService = inject(ToastService);

  readonly items = signal<StoreItem[]>([]);
  readonly activeCategory = signal<StoreCategory | 'All'>('All');
  readonly selectedItem = signal<StoreItem | null>(null);

  readonly categories: (StoreCategory | 'All')[] = [
    'All',
    'Featured',
    'Cosmetics',
    'Supporter Packs',
    'Account',
    'Other'
  ];

  readonly filteredItems = computed(() => {
    const list = this.items();
    const cat = this.activeCategory();
    if (cat === 'All') return list;
    if (cat === 'Featured') return list.filter(i => i.featured);
    return list.filter(i => i.category === cat);
  });

  constructor() {
    this.contentService.getStoreItems().subscribe(data => {
      this.items.set(data);
    });
  }

  setCategory(cat: StoreCategory | 'All') {
    this.activeCategory.set(cat);
  }

  openItemModal(item: StoreItem) {
    this.selectedItem.set(item);
  }

  closeItemModal() {
    this.selectedItem.set(null);
  }

  onMockPurchase(item: StoreItem) {
    this.toastService.showWarning(
      `Purchasing "${item.name}" is unavailable in this prototype.`,
      'Prototype Store'
    );
  }
}
