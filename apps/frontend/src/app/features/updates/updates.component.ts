import { Component, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { ContentService } from '../../core/services/content.service';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { GameUpdate } from '../../shared/models/update.model';

@Component({
  selector: 'app-updates',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent],
  templateUrl: './updates.component.html',
  styleUrls: ['./updates.component.scss']
})
export class UpdatesComponent {
  private contentService = inject(ContentService);

  readonly updates = signal<GameUpdate[]>([]);
  readonly activeType = signal<'All' | 'Development' | 'Patch Notes'>('All');

  readonly filteredUpdates = computed(() => {
    const list = this.updates();
    const type = this.activeType();
    if (type === 'All') return list;
    return list.filter(u => u.type === type);
  });

  constructor() {
    this.contentService.getGameUpdates().subscribe(data => {
      this.updates.set(data);
    });
  }

  setType(type: 'All' | 'Development' | 'Patch Notes') {
    this.activeType.set(type);
  }

  scrollToUpdate(id: string) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth' });
    }
  }
}
