import { ComponentFixture, TestBed } from '@angular/core/testing';
import { EivarCardComponent } from './card.component';

describe('EivarCardComponent', () => {
  let fixture: ComponentFixture<EivarCardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EivarCardComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(EivarCardComponent);
  });

  it('renders its supplied content and image', () => {
    fixture.componentRef.setInput('title', 'Explore');
    fixture.componentRef.setInput('image', 'assets/images/world-exploration.jpg');
    fixture.componentRef.setInput('imageAlt', 'A green valley');
    fixture.detectChanges();

    const card = fixture.nativeElement.querySelector('.eivar-card');
    const image = fixture.nativeElement.querySelector('img');

    expect(card.textContent).toContain('Explore');
    expect(image.alt).toBe('A green valley');
  });
});
