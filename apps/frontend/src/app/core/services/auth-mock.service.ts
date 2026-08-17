import { Injectable, signal } from '@angular/core';

export interface MockUser {
  name: string;
  rank: string;
  avatar: string;
  email: string;
  joinedAlpha: string;
  astralMarks: number;
}

@Injectable({
  providedIn: 'root'
})
export class AuthMockService {
  readonly isLoggedIn = signal<boolean>(false);
  readonly currentUser = signal<MockUser | null>(null);

  loginMock(email: string) {
    this.isLoggedIn.set(true);
    this.currentUser.set({
      name: email.split('@')[0] || 'Wayfarer',
      rank: 'Vanguard Initiate · Level 14',
      avatar: 'assets/images/wayfarer-cloak.jpg',
      email,
      joinedAlpha: 'August 2026',
      astralMarks: 2400
    });
  }

  logoutMock() {
    this.isLoggedIn.set(false);
    this.currentUser.set(null);
  }
}
