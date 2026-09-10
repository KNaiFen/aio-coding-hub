// Usage:
// - Query adapter for `src/services/providerLimitUsage.ts` used by `src/components/home/HomeProviderLimitPanel.tsx`.

import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  providerLimitUsageV1,
  providerLimitReset,
  type ProviderLimitPeriod,
  validateProviderLimitUsageCliKey,
} from "../services/providers/providerLimitUsage";
import { providerLimitUsageKeys } from "./keys";
import { formatUnknownError } from "../utils/errors";

export function useProviderLimitUsageV1Query(
  cliKey: CliKey | null,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  const normalizedCliKey = validateProviderLimitUsageCliKey(cliKey);

  return useQuery({
    queryKey: providerLimitUsageKeys.list(normalizedCliKey),
    queryFn: () => providerLimitUsageV1(normalizedCliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export class ProviderLimitRefreshError extends Error {
  constructor(readonly cause: unknown) {
    super(`周期已重设，但用量刷新失败：${formatUnknownError(cause)}`);
  }
}

export function useProviderLimitResetMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ providerId, period }: { providerId: number; period: ProviderLimitPeriod }) => {
      await providerLimitReset(providerId, period);
      try {
        await queryClient.cancelQueries({ queryKey: providerLimitUsageKeys.all });
        await queryClient.invalidateQueries(
          { queryKey: providerLimitUsageKeys.all },
          { throwOnError: true }
        );
      } catch (error) {
        throw new ProviderLimitRefreshError(error);
      }
    },
    retry: false,
  });
}
