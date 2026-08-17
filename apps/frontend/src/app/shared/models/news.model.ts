export type NewsCategory = 'Announcements' | 'Development' | 'Community' | 'Events';

export interface NewsArticle {
  id: string;
  slug: string;
  title: string;
  subtitle?: string;
  excerpt: string;
  content: string[];
  category: NewsCategory;
  publishedAt: string;
  image: string;
  readingTime: number; // in minutes
  tags: string[];
  featured?: boolean;
  author?: {
    name: string;
    role: string;
  };
}
