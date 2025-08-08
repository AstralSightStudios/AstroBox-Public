"use client";
import { createContext, ReactNode, useContext, useState } from 'react';
import en_US from './en.json';
import zh_CN from './zh.json';
import zh_HK from './zh_HK.json';
import zh_Can from './zh_Can.json';
import zh_MS from './zh_MS.json';
import zh_HX from './zh_HX.json';
import zh_Meme from './zh_Meme.json';
import zh_hlt from './zh_hlt.json';

const resources = { zh_CN, en_US, zh_HK, zh_Can, zh_HX, zh_Meme, zh_MS, zh_hlt } as const;
export type Lang = keyof typeof resources;

const STORAGE_KEY = "language";

const LANG_NAME: any = {
  "zh_CN": "中文 (简体)",
  "zh_HK": "中文 (香港)",
  "zh_Can": "中文（粤语)",
  "zh_HX": "文言 (華夏)",
  "zh_Meme": "中文 (神人)",
  "zh_MS": "中文 (巨硬)",
  "zh_hlt":"中文（河南）",
  "en_US": "English (US)",
}

interface I18nContextProps {
  lang: Lang;
  langs: Lang[];
  setLang: (lang: Lang) => void;
  langNames: any;
  t: (key: string | undefined | null) => string;
}

const I18nContext = createContext<I18nContextProps>({
  lang: 'en_US',
  langs: Object.keys(resources) as Lang[],
  setLang: () => { },
  langNames: LANG_NAME,
  t: (key) => (key ?? ''),
});

function getDefaultLang() {
  try {
    if (typeof localStorage !== 'undefined') {
      const lang = localStorage.getItem(STORAGE_KEY);
      if (lang && lang in resources) {
        return lang as Lang;
      }
    }
  } catch (error) {
    console.error('[i18n] getDefaultLang() failed:', error);
  }
  const browserLang = typeof navigator !== 'undefined' ? navigator.language.replaceAll("-", "_") : '';
  if (browserLang in resources) {
    return browserLang as Lang;
  }
  return 'en_US';
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, _setLang] = useState<Lang>(getDefaultLang());

  const t = (key: string | undefined | null): string => {
    if (!key || typeof key !== 'string') {
      if (process.env.NODE_ENV !== 'production') {
        console.warn('[i18n] t() called with invalid key:', key);
      }
      return key ?? '';
    }
    const obj = resources[lang] as Record<string, any>;
    const result = key.split('.').reduce((acc, cur) => acc?.[cur], obj);
    if (typeof result === 'string') return result;
    const fallback = key.split('.').reduce((acc, cur) => acc?.[cur], resources.en_US as Record<string, any>);
    return typeof fallback === 'string' ? fallback : key;
  };

  const setLang = (lang: Lang) => {
    localStorage.setItem(STORAGE_KEY, lang);
    _setLang(lang);
  }

  return (
    <I18nContext.Provider value={{ lang, setLang, t, langs: Object.keys(resources) as Lang[], langNames: LANG_NAME }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() {
  return useContext(I18nContext);
}
