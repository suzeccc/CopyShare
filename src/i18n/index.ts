import englishCatalog from "../../locales/en-US.json" with { type: "json" };
import traditionalCatalog from "../../locales/zh-TW.json" with { type: "json" };
import japaneseCatalog from "../../locales/ja-JP.json" with { type: "json" };

import type { UiLanguage } from "@/types/config";

export type EffectiveLocale = "zh-CN" | "zh-TW" | "en-US" | "ja-JP";

const HAN_PATTERN = /[\u3400-\u9fff]/u;
const LOCALIZED_ATTRIBUTES = ["aria-label", "placeholder", "title"] as const;
const TEXT_IGNORE_SELECTOR = [
  "[data-i18n-ignore]",
  "code",
  "pre",
  "script",
  "style",
  "textarea",
  "[contenteditable='true']",
].join(",");
const ATTRIBUTE_IGNORE_SELECTOR = "[data-i18n-ignore]";
const english = englishCatalog as Record<string, string>;
const catalogs: Record<Exclude<EffectiveLocale, "zh-CN">, Record<string, string>> = {
  "zh-TW": traditionalCatalog as Record<string, string>,
  "en-US": english,
  "ja-JP": { ...english, ...(japaneseCatalog as Record<string, string>) },
};
const nativeErrorSources: Record<string, string> = {
  "invalid input: source file path": "路径无效",
  "invalid input: ": "输入无效：",
  "i/o error: ": "I/O 错误：",
  "json error: ": "JSON 错误：",
  "url error: ": "URL 错误：",
  "websocket error: ": "WebSocket 错误：",
  "unknown device: ": "未知设备：",
  "tauri error: ": "Tauri 错误：",
  "sync is already running": "同步已在运行",
  "sync is not running": "同步未运行",
  "failed to read clipboard image data": "读取剪贴板图片数据失败",
  "failed to read clipboard string": "读取剪贴板文本失败",
  "failed to read clipboard png data": "读取剪贴板 PNG 数据失败",
  "when reading the dibv5 data, it contained fewer bytes than the bitmapv5header size. this is invalid.": "读取剪贴板图片数据失败",
  "unable to register html format": "无法注册 HTML 格式",
  "could not place the specified text to the clipboard": "无法将指定文本写入剪贴板",
  "failed to clear clipboard": "清空剪贴板失败",
  "clipboard error: ": "剪贴板错误：",
  "the clipboard contents were not available in the requested format or the clipboard is empty.": "剪贴板为空或不包含所需格式的内容",
  "the selected clipboard is not supported with the current system configuration.": "当前系统配置不支持此剪贴板",
  "the native clipboard is not accessible due to being held by another party.": "剪贴板被其他程序占用，暂时无法访问",
  "the image or the text that was about the be transferred to/from the clipboard could not be converted to the appropriate format.": "图片或文字无法转换成剪贴板支持的格式",
  "unknown error while interacting with the clipboard: ": "访问剪贴板时发生未知错误：",
};
for (const source of Object.values(nativeErrorSources)) {
  const translated = english[source.replace(/：$/u, "")];
  if (translated) nativeErrorSources[(translated + (source.endsWith("：") ? ": " : "")).toLowerCase()] = source;
}
const nativeErrorPattern = new RegExp(Object.keys(nativeErrorSources).map(escapeRegExp).join("|"), "gi");

function normalizeNativeError(value: string): string {
  return value.replace(nativeErrorPattern, phrase => nativeErrorSources[phrase.toLowerCase()] ?? phrase);
}
const phrasePattern = new RegExp(
  Object.keys(english)
    .filter((phrase) => Object.values(catalogs).some((catalog) => catalog[phrase] !== phrase))
    .sort((left, right) => right.length - left.length)
    .map(escapeRegExp)
    .join("|"),
  "gu",
);

type OriginalValue = { source: string; translated: string };
const originalText = new WeakMap<Text, OriginalValue>();
const originalAttributes = new WeakMap<Element, Map<string, OriginalValue>>();

let preference: UiLanguage = "system";
let effectiveLocale: EffectiveLocale = resolveUiLanguage(preference);
let observer: MutationObserver | undefined;

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function resolveUiLanguage(
  language: UiLanguage,
  systemLanguage = typeof navigator === "undefined" ? "en-US" : navigator.language,
): EffectiveLocale {
  if (language !== "system") {
    return language;
  }
  const system = systemLanguage.toLowerCase();
  if (system.startsWith("zh-tw") || system.startsWith("zh-hk") || system.startsWith("zh-mo")) return "zh-TW";
  if (system.startsWith("zh")) return "zh-CN";
  if (system.startsWith("ja")) return "ja-JP";
  return "en-US";
}

export function getUiLanguage(): UiLanguage {
  return preference;
}

export function getEffectiveLocale(): EffectiveLocale {
  return effectiveLocale;
}

export function translateSource(value: string, locale = effectiveLocale): string {
  value = normalizeNativeError(value);
  if (locale === "zh-CN" || !HAN_PATTERN.test(value)) {
    return value;
  }
  HAN_PATTERN.lastIndex = 0;
  return value
    .replace(phrasePattern, (phrase) => catalogs[locale][phrase] ?? phrase)
    .replaceAll("，", ", ")
    .replaceAll("。", ".")
    .replaceAll("：", ": ")
    .replaceAll("、", ", ")
    .replaceAll("；", "; ")
    .replaceAll("？", "?")
    .replaceAll("！", "!");
}

function textShouldBeIgnored(node: Text): boolean {
  return node.parentElement?.closest(TEXT_IGNORE_SELECTOR) !== null;
}

function attributesShouldBeIgnored(element: Element): boolean {
  return element.closest(ATTRIBUTE_IGNORE_SELECTOR) !== null;
}

function localizeTextNode(node: Text): void {
  if (textShouldBeIgnored(node)) {
    return;
  }

  const current = normalizeNativeError(node.data);
  const remembered = originalText.get(node);
  const source = remembered && current === remembered.translated ? remembered.source : current;
  if (effectiveLocale === "zh-CN") {
    if (node.data !== source) {
      node.data = source;
    }
    originalText.delete(node);
    return;
  }

  if (HAN_PATTERN.test(source)) {
    HAN_PATTERN.lastIndex = 0;
    const translated = translateSource(source);
    originalText.set(node, { source, translated });
    if (node.data !== translated) {
      node.data = translated;
    }
  }
}

function localizeElementAttributes(element: Element): void {
  if (attributesShouldBeIgnored(element)) {
    return;
  }

  const stored = originalAttributes.get(element);
  if (effectiveLocale === "zh-CN") {
    for (const name of LOCALIZED_ATTRIBUTES) {
      const value = stored?.get(name)?.source ?? element.getAttribute(name);
      if (value !== null) {
        if (element.getAttribute(name) !== value) element.setAttribute(name, value);
      }
    }
    originalAttributes.delete(element);
    return;
  }

  for (const name of LOCALIZED_ATTRIBUTES) {
    const value = element.getAttribute(name);
    const current = value === null ? null : normalizeNativeError(value);
    const remembered = stored?.get(name);
    const source = remembered && current === remembered.translated ? remembered.source : current;
    if (!source || !HAN_PATTERN.test(source)) {
      HAN_PATTERN.lastIndex = 0;
      continue;
    }
    HAN_PATTERN.lastIndex = 0;
    let originals = originalAttributes.get(element);
    if (!originals) {
      originals = new Map();
      originalAttributes.set(element, originals);
    }
    const translated = translateSource(source);
    originals.set(name, { source, translated });
    if (value !== translated) element.setAttribute(name, translated);
  }
}

function localizeSubtree(root: Node): void {
  if (root.nodeType === Node.TEXT_NODE) {
    localizeTextNode(root as Text);
    return;
  }
  if (root.nodeType !== Node.ELEMENT_NODE && root.nodeType !== Node.DOCUMENT_NODE) {
    return;
  }

  if (root.nodeType === Node.ELEMENT_NODE) {
    localizeElementAttributes(root as Element);
  }
  const walker = document.createTreeWalker(
    root,
    NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT,
  );
  let current = walker.nextNode();
  while (current) {
    if (current.nodeType === Node.TEXT_NODE) {
      localizeTextNode(current as Text);
    } else {
      localizeElementAttributes(current as Element);
    }
    current = walker.nextNode();
  }
}

function observeDocument(): void {
  if (!document.documentElement) return;
  observer?.disconnect();
  observer = new MutationObserver((records) => {
    observer?.disconnect();
    for (const record of records) {
      if (record.type === "characterData") {
        localizeTextNode(record.target as Text);
      } else if (record.type === "attributes") {
        localizeElementAttributes(record.target as Element);
      } else {
        for (const node of record.addedNodes) {
          localizeSubtree(node);
        }
      }
    }
    observeDocument();
  });
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: [...LOCALIZED_ATTRIBUTES],
    characterData: true,
    childList: true,
    subtree: true,
  });
}

function applyLocaleToDocument(): void {
  if (!document.documentElement) return;
  observer?.disconnect();
  document.documentElement.lang = effectiveLocale;
  localizeSubtree(document.documentElement);
  observeDocument();
}

export function setUiLanguage(language: UiLanguage): void {
  preference = language;
  effectiveLocale = resolveUiLanguage(language);
  applyLocaleToDocument();
  window.dispatchEvent(new CustomEvent("copyshare-locale-changed", {
    detail: { language, locale: effectiveLocale },
  }));
}

export function initializeI18n(language: UiLanguage): void {
  setUiLanguage(language);
}
