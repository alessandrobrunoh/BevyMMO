import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AbilityDefinition } from '../../models/wiki.model';

@Component({
  selector: 'app-ability-card',
  standalone: true,
  imports: [CommonModule],
  template: `
    @if (ability) {
      <div class="ability-card chamfer-box">
        <div class="ability-slot-badge">
          <span class="slot-letter">{{ ability.slot }}</span>
        </div>

        <div class="ability-main">
          <div class="ability-header">
            <div>
              <h4 class="ability-name">{{ ability.name }}</h4>
              <span class="ability-type">{{ ability.baseType }}</span>
            </div>
            <div class="ability-costs">
              <span class="cost-chip cd">⌛ {{ ability.cooldown }}</span>
              <span class="cost-chip mp">💧 {{ ability.energyCost }}</span>
            </div>
          </div>

          <p class="ability-desc">{{ ability.description }}</p>

          <!-- Inscription Recommendations -->
          <div class="inscriptions-block">
            @if (ability.recommendedEssence) {
              <div class="inscription-row">
                <span class="ins-label">Essence:</span>
                <span class="ins-value essence-tag">{{ ability.recommendedEssence }}</span>
              </div>
            }
            @if (ability.recommendedModifiers && ability.recommendedModifiers.length > 0) {
              <div class="inscription-row">
                <span class="ins-label">Modifiers:</span>
                <div class="tags-group">
                  @for (mod of ability.recommendedModifiers; track mod) {
                    <span class="ins-value mod-tag">{{ mod }}</span>
                  }
                </div>
              </div>
            }
            @if (ability.recommendedAncientWord) {
              <div class="inscription-row">
                <span class="ins-label">Ancient Word:</span>
                <span class="ins-value word-tag">{{ ability.recommendedAncientWord }}</span>
              </div>
            }
          </div>
        </div>
      </div>
    }
  `,
  styleUrls: ['./ability-card.component.scss']
})
export class AbilityCardComponent {
  @Input() ability?: AbilityDefinition;
}
