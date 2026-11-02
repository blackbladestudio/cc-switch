import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useMoneyFormat } from "@/hooks/useMoneyFormat";
import type { Settings } from "@/types";
import { createTestQueryClient } from "../utils/testQueryClient";

const renderWithSettings = (settings: Partial<Settings>) => {
  const client = createTestQueryClient();
  client.setQueryData(["settings"], settings);
  return renderHook(() => useMoneyFormat(), {
    wrapper: ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    ),
  }).result.current;
};

describe("useMoneyFormat", () => {
  it("默认沿用定制版人民币汇率并保留美元悬停值", () => {
    const fmt = renderWithSettings({});
    expect(fmt.currency).toBe("CNY");
    expect(fmt.money(1.5, 2)).toBe("¥10.71");
    expect(fmt.usdHint(1.5)).toBe("$1.500000");
    expect(fmt.moneyTitle(1.5, 6)).toBe("¥10.71 ($1.500000)");
  });

  it("converts with the saved rate and keeps the USD value for hover", () => {
    const fmt = renderWithSettings({
      usageDisplay: { currency: "CNY", exchangeRates: { CNY: 7 } },
    });
    expect(fmt.currency).toBe("CNY");
    expect(fmt.rate).toBe(7);
    expect(fmt.money(1.5, 2)).toBe("¥10.50");
    expect(fmt.moneyShort(41.2)).toBe("¥288");
    expect(fmt.usdHint(1.5)).toBe("$1.500000");
    expect(fmt.moneyTitle(1.5, 6)).toBe("¥10.50 ($1.500000)");
  });
});
