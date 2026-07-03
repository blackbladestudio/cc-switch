import { createContext, useContext, useMemo, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { usageApi } from "@/lib/api/usage";
import { parseFiniteNumber } from "./format";

/**
 * 内置定价覆盖层的 CNY→USD 汇率。
 *
 * 后端成本数值（total_cost_usd 等）是 USD 语义；前端展示统一 × rate 还原成
 * CNY 再标 ¥（见 fmtCost）。rate 随应用内置定价资源发布，运行时不变，
 * 所以这里用 staleTime: Infinity 缓存，避免重复 IPC。
 */
const PricingRateContext = createContext<number | null>(null);

export function PricingRateProvider({ children }: { children: ReactNode }) {
  const { data } = useQuery({
    queryKey: ["pricing-rate"],
    queryFn: () => usageApi.getPricingRate(),
    staleTime: Infinity,
    gcTime: Infinity,
  });

  const value = useMemo(() => parseFiniteNumber(data) ?? null, [data]);

  return (
    <PricingRateContext.Provider value={value}>
      {children}
    </PricingRateContext.Provider>
  );
}

/**
 * 取当前 CNY→USD 汇率，供 fmtCost 使用。
 * 返回 null 表示尚未加载完成，调用方应回退到 "--"。
 */
export function usePricingRate(): number | null {
  return useContext(PricingRateContext);
}
