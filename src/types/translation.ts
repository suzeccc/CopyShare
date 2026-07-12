export type TranslationEngine = "google" | "ai";

export interface TranslateResponse {
  sourceText: string;
  targetText: string;
  engine: TranslationEngine;
}

export interface TranslateTargetLanguage {
  code: string;
  label: string;
  badge: string;
}
