export function parseFiniteNumber(value: unknown): number | null {
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === "string") {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

export function fmtInt(
  value: unknown,
  locale?: string,
  fallback: string = "--",
): string {
  const num = parseFiniteNumber(value);
  if (num == null) return fallback;
  return new Intl.NumberFormat(locale).format(Math.trunc(num));
}

export function fmtCny(
  value: unknown,
  digits: number,
  fallback: string = "--",
): string {
  const num = parseFiniteNumber(value);
  if (num == null) return fallback;
  return `¥${num.toFixed(digits)}`;
}

/**
 * 把后端内部单位（USD 语义）的成本换算成 CNY 显示。
 *
 * 内部成本数值（total_cost_usd 等）是 USD 语义：DB 种子价本来就是 USD，
 * 内置定价覆盖层也是 CNY ÷ rate 折成的 USD。前端展示统一 × rate 还原成
 * CNY 再标 ¥，避免把 USD 数值直接贴 ¥ 符号导致量级偏低（约 7× 偏差）。
 *
 * `rate` 来自内置定价覆盖层（get_pricing_rate），字符串形式（rust_decimal）。
 */
export function fmtCost(
  usdValue: unknown,
  rate: unknown,
  digits: number,
  fallback: string = "--",
): string {
  const usd = parseFiniteNumber(usdValue);
  const r = parseFiniteNumber(rate);
  if (usd == null || r == null || r <= 0) return fallback;
  return `¥${(usd * r).toFixed(digits)}`;
}

/** @deprecated 习惯用语保留；成本展示统一改用 fmtCny（人民币） */
export function fmtUsd(
  value: unknown,
  digits: number,
  fallback: string = "--",
): string {
  return fmtCny(value, digits, fallback);
}

function normalizeLanguageTag(language: string): string {
  return language.toLowerCase().replace(/_/g, "-");
}

function isTraditionalChineseLanguage(normalizedLanguage: string): boolean {
  return (
    normalizedLanguage === "zh-tw" ||
    normalizedLanguage.startsWith("zh-hant") ||
    normalizedLanguage.startsWith("zh-hk") ||
    normalizedLanguage.startsWith("zh-mo")
  );
}

export function getLocaleFromLanguage(language: string): string {
  if (!language) return "en-US";
  const normalized = normalizeLanguageTag(language);
  if (normalized === "zh") return "zh-CN";
  if (isTraditionalChineseLanguage(normalized)) {
    return "zh-TW";
  }
  if (normalized.startsWith("zh")) return "zh-CN";
  if (normalized.startsWith("ja")) return "ja-JP";
  return "en-US";
}

interface I18nLike {
  resolvedLanguage?: string;
  language?: string;
}

export function getResolvedLang(i18n: I18nLike): string {
  return i18n.resolvedLanguage || i18n.language || "en";
}

/**
 * Token 数量的紧凑显示。
 *
 * Why: 中日文用户期待 "亿/万" 量纲；英文用户期待 K/M/B。共用同一份格式化
 * 逻辑避免 Hero 卡和分应用卡显示不一致。`compactDecimals=2` 用于 Hero
 * 大数副标（更精确），默认 1 位用于卡片副字段。
 */
export function formatTokensShort(
  value: number,
  lang: string,
  compactDecimals: 1 | 2 = 1,
): string {
  if (!Number.isFinite(value) || value <= 0) return "0";
  const decimals = compactDecimals;
  const normalizedLang = normalizeLanguageTag(lang);
  if (isTraditionalChineseLanguage(normalizedLang)) {
    if (value >= 1e8) return `${(value / 1e8).toFixed(2)} 億`;
    if (value >= 1e4) return `${(value / 1e4).toFixed(decimals)} 萬`;
    return value.toLocaleString("zh-TW");
  }
  if (normalizedLang.startsWith("zh") || normalizedLang.startsWith("ja")) {
    if (value >= 1e8) return `${(value / 1e8).toFixed(2)} 亿`;
    if (value >= 1e4) return `${(value / 1e4).toFixed(decimals)} 万`;
    return value.toLocaleString();
  }
  if (value >= 1e9) return `${(value / 1e9).toFixed(2)}B`;
  if (value >= 1e6) return `${(value / 1e6).toFixed(2)}M`;
  if (value >= 1e3) return `${(value / 1e3).toFixed(decimals)}K`;
  return value.toLocaleString();
}
