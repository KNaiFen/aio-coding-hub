import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient, QueryKey } from "@tanstack/react-query";
import {
  validateProviderCliKey,
  validateProviderId,
  type CliKey,
} from "../services/providers/providers";
import {
  providerModelRoutingPolicyGet,
  providerModelRoutingPolicySave,
  routingProviderCandidatesList,
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProviderSetEnabled,
  sortModeProviderSetSessionReusePriority,
  sortModeProvidersList,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
  type SortModeActiveRow,
  type ProviderModelRoutingPolicySaveInput,
  type ProviderModelRoutingPolicyView,
  type RoutingProviderCandidate,
  type SortModeProviderRow,
  validateSortModeId,
  validateSortModeUuid,
} from "../services/providers/sortModes";
import { providersKeys, sortModesKeys } from "./keys";

export function sortModeProvidersQueryPrefix(cliKey: CliKey) {
  return [...sortModesKeys.all, "providers", validateProviderCliKey(cliKey)] as const;
}

export function sortModeProvidersQueryKey(modeId: number, modeUuid: string, cliKey: CliKey) {
  return sortModesKeys.providers(
    validateProviderCliKey(cliKey),
    validateSortModeId(modeId),
    validateSortModeUuid(modeUuid)
  );
}

export function providerRoutingPolicyQueryKey(input: {
  cliKey: CliKey;
  providerId: number | null;
  providerUuid: string | null;
  modeId: number | null;
  modeUuid: string | null;
}) {
  const cliKey = validateProviderCliKey(input.cliKey);
  const providerId = input.providerId == null ? null : validateProviderId(input.providerId);
  const providerUuid =
    input.providerUuid == null ? null : validateSortModeUuid(input.providerUuid);
  const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
  const modeUuid = input.modeUuid == null ? null : validateSortModeUuid(input.modeUuid);
  if ((providerId == null) !== (providerUuid == null) || (modeId == null) !== (modeUuid == null)) {
    throw new Error("SEC_INVALID_INPUT: incomplete routing editor identity");
  }
  return sortModesKeys.routingPolicy(
    cliKey,
    modeId,
    modeUuid,
    providerId,
    providerUuid
  );
}

export function routingProviderCandidatesQueryKey(input: {
  cliKey: CliKey;
  modeId: number;
  modeUuid: string;
}) {
  return sortModesKeys.routingCandidates(
    validateProviderCliKey(input.cliKey),
    validateSortModeId(input.modeId),
    validateSortModeUuid(input.modeUuid)
  );
}

function keyMatchesRoutingPolicyProvider(
  queryKey: QueryKey,
  cliKey: CliKey,
  providerId: number,
  providerUuid?: string
) {
  return (
    queryKey[0] === sortModesKeys.all[0] &&
    queryKey[1] === "routingPolicy" &&
    queryKey[2] === cliKey &&
    queryKey[5] === providerId &&
    (providerUuid == null || queryKey[6] === providerUuid)
  );
}

function keyMatchesRoutingPolicyMode(
  queryKey: QueryKey,
  cliKey: CliKey,
  modeId: number,
  modeUuid: string
) {
  return (
    queryKey[0] === sortModesKeys.all[0] &&
    queryKey[1] === "routingPolicy" &&
    queryKey[2] === cliKey &&
    queryKey[3] === modeId &&
    queryKey[4] === modeUuid
  );
}

function keyMatchesModeIdentity(queryKey: QueryKey, modeId: number, modeUuid: string) {
  return (
    queryKey[0] === sortModesKeys.all[0] &&
    ((queryKey[1] === "providers" && queryKey[3] === modeId && queryKey[4] === modeUuid) ||
      (queryKey[1] === "routingPolicy" && queryKey[3] === modeId && queryKey[4] === modeUuid) ||
      (queryKey[1] === "routingCandidates" &&
        queryKey[3] === modeId &&
        queryKey[4] === modeUuid))
  );
}

export async function invalidateRoutingEditorForCli(queryClient: QueryClient, cliKey: CliKey) {
  const normalizedCliKey = validateProviderCliKey(cliKey);
  await Promise.all([
    queryClient.invalidateQueries({
      queryKey: sortModesKeys.routingCandidatesForCli(normalizedCliKey),
    }),
    queryClient.invalidateQueries({ queryKey: sortModesKeys.routingPolicies(normalizedCliKey) }),
  ]);
}

export async function invalidateRoutingEditorForProvider(
  queryClient: QueryClient,
  input: { cliKey: CliKey; providerId: number; providerUuid?: string }
) {
  const cliKey = validateProviderCliKey(input.cliKey);
  const providerId = validateProviderId(input.providerId);
  const providerUuid =
    input.providerUuid == null ? undefined : validateSortModeUuid(input.providerUuid);
  await Promise.all([
    queryClient.invalidateQueries({
      queryKey: sortModesKeys.routingCandidatesForCli(cliKey),
    }),
    queryClient.invalidateQueries({
      predicate: (query) =>
        keyMatchesRoutingPolicyProvider(query.queryKey, cliKey, providerId, providerUuid),
    }),
  ]);
}

export async function invalidateRoutingEditorForMode(
  queryClient: QueryClient,
  input: { cliKey: CliKey; modeId: number; modeUuid: string }
) {
  const cliKey = validateProviderCliKey(input.cliKey);
  const modeId = validateSortModeId(input.modeId);
  const modeUuid = validateSortModeUuid(input.modeUuid);
  await Promise.all([
    queryClient.invalidateQueries({
      queryKey: sortModesKeys.routingCandidates(cliKey, modeId, modeUuid),
      exact: true,
    }),
    queryClient.invalidateQueries({
      predicate: (query) => keyMatchesRoutingPolicyMode(query.queryKey, cliKey, modeId, modeUuid),
    }),
  ]);
}

export function useSortModesListQuery(options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: sortModesKeys.list(),
    queryFn: () => sortModesList(),
    enabled: options.enabled ?? true,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useSortModeActiveListQuery(options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: sortModesKeys.activeList(),
    queryFn: () => sortModeActiveList(),
    enabled: options.enabled ?? true,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useSortModeProvidersListQuery(
  input: { modeId: number | null; modeUuid: string | null; cliKey: CliKey },
  options: { enabled?: boolean } = {}
) {
  const cliKey = validateProviderCliKey(input.cliKey);
  const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
  const modeUuid = input.modeUuid == null ? null : validateSortModeUuid(input.modeUuid);
  if ((modeId == null) !== (modeUuid == null)) {
    throw new Error("SEC_INVALID_INPUT: incomplete sort mode identity");
  }

  return useQuery({
    queryKey:
      modeId == null
        ? sortModesKeys.providers(cliKey, null, null)
        : sortModeProvidersQueryKey(modeId, modeUuid!, cliKey),
    queryFn: () => {
      if (modeId == null) {
        return Promise.resolve<SortModeProviderRow[] | null>(null);
      }
      return sortModeProvidersList({ mode_id: modeId, cli_key: cliKey });
    },
    enabled: modeId != null && (options.enabled ?? true),
    retry: false,
  });
}

export function useProviderRoutingPolicyQuery(
  input: {
    cliKey: CliKey;
    providerId: number | null;
    providerUuid: string | null;
    modeId: number | null;
    modeUuid: string | null;
  },
  options: { enabled?: boolean } = {}
) {
  const queryKey = providerRoutingPolicyQueryKey(input);
  const providerId = queryKey[5];
  const providerUuid = queryKey[6];
  const modeId = queryKey[3];
  const modeUuid = queryKey[4];

  return useQuery({
    queryKey,
    queryFn: () => {
      if (providerId == null || providerUuid == null) {
        return Promise.resolve<ProviderModelRoutingPolicyView | null>(null);
      }
      return providerModelRoutingPolicyGet({
        provider_id: providerId,
        provider_uuid: providerUuid,
        mode_id: modeId,
        mode_uuid: modeUuid,
      }).then((view) => {
        if (view.cli_key !== queryKey[2]) {
          throw new Error("IPC_INVALID_SCOPE: provider routing policy CLI");
        }
        return view;
      });
    },
    enabled: providerId != null && providerUuid != null && (options.enabled ?? true),
    retry: false,
  });
}

export function useRoutingProviderCandidatesQuery(
  input: { cliKey: CliKey; modeId: number | null; modeUuid: string | null },
  options: { enabled?: boolean } = {}
) {
  const cliKey = validateProviderCliKey(input.cliKey);
  const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
  const modeUuid = input.modeUuid == null ? null : validateSortModeUuid(input.modeUuid);
  if ((modeId == null) !== (modeUuid == null)) {
    throw new Error("SEC_INVALID_INPUT: incomplete candidate mode identity");
  }
  const queryKey =
    modeId == null || modeUuid == null
      ? sortModesKeys.routingCandidates(cliKey, null, null)
      : routingProviderCandidatesQueryKey({ cliKey, modeId, modeUuid });

  return useQuery({
    queryKey,
    queryFn: () => {
      if (modeId == null || modeUuid == null) {
        return Promise.resolve<RoutingProviderCandidate[] | null>(null);
      }
      return routingProviderCandidatesList({
        mode_id: modeId,
        mode_uuid: modeUuid,
        cli_key: cliKey,
      });
    },
    enabled: modeId != null && modeUuid != null && (options.enabled ?? true),
    retry: false,
  });
}

export function useProviderRoutingPolicySaveMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: ProviderModelRoutingPolicySaveInput & { cliKey: CliKey }) =>
      providerModelRoutingPolicySave(input).then((view) => {
        if (view.cli_key !== validateProviderCliKey(input.cliKey)) {
          throw new Error("IPC_INVALID_SCOPE: saved provider routing policy CLI");
        }
        return view;
      }),
    onMutate: async (input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      const providerId = validateProviderId(input.provider_id);
      const providerUuid = validateSortModeUuid(input.provider_uuid);
      await queryClient.cancelQueries({
        predicate: (query) =>
          keyMatchesRoutingPolicyProvider(
            query.queryKey,
            cliKey,
            providerId,
            providerUuid
          ),
      });
    },
    onSuccess: async (view, input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      const queryKey = providerRoutingPolicyQueryKey({
        cliKey,
        providerId: view.provider_id,
        providerUuid: view.provider_uuid,
        modeId: view.selected_mode?.mode_id ?? null,
        modeUuid: view.selected_mode?.mode_uuid ?? null,
      });
      queryClient.setQueriesData<ProviderModelRoutingPolicyView | null>(
        {
          predicate: (query) =>
            keyMatchesRoutingPolicyProvider(
              query.queryKey,
              cliKey,
              view.provider_id,
              view.provider_uuid
            ),
        },
        (cached) =>
          cached == null
            ? cached
            : {
                ...cached,
                provider_override_enabled: view.provider_override_enabled,
                ordinary_policy: view.ordinary_policy,
                ordinary_policy_revision: view.ordinary_policy_revision,
              }
      );
      queryClient.setQueryData(queryKey, view);
      await queryClient.invalidateQueries({
        queryKey: providersKeys.list(cliKey),
        exact: true,
      });
      if (view.selected_mode != null) {
        await queryClient.invalidateQueries({
          queryKey: routingProviderCandidatesQueryKey({
            cliKey,
            modeId: view.selected_mode.mode_id,
            modeUuid: view.selected_mode.mode_uuid,
          }),
          exact: true,
        });
      }
    },
  });
}

export function useSortModeActiveSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; modeId: number | null }) => {
      const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
      return sortModeActiveSet({ cli_key: validateProviderCliKey(input.cliKey), mode_id: modeId });
    },
    onMutate: (input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
      void queryClient.cancelQueries({ queryKey: sortModesKeys.activeList() });

      const previous =
        queryClient.getQueryData<SortModeActiveRow[] | null>(sortModesKeys.activeList()) ?? null;

      if (previous) {
        const next = previous.map((row) =>
          row.cli_key === cliKey ? { ...row, mode_id: modeId } : row
        );
        queryClient.setQueryData(sortModesKeys.activeList(), next);
      }

      return { previous };
    },
    onSuccess: (res) => {
      queryClient.setQueryData<SortModeActiveRow[] | null>(sortModesKeys.activeList(), (prev) => {
        if (!prev) return prev;
        return prev.map((row) => (row.cli_key === res.cli_key ? res : row));
      });
    },
    onError: (_err, _input, ctx) => {
      if (ctx?.previous) {
        queryClient.setQueryData(sortModesKeys.activeList(), ctx.previous);
      }
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.activeList() });
    },
  });
}

export function useSortModeCreateMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { name: string }) => sortModeCreate({ name: input.name }),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
    },
  });
}

export function useSortModeRenameMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; name: string }) =>
      sortModeRename({ mode_id: validateSortModeId(input.modeId), name: input.name }),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
    },
  });
}

export function useSortModeDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; modeUuid: string }) =>
      sortModeDelete({ mode_id: validateSortModeId(input.modeId) }),
    onSettled: (_data, _error, input) => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.activeList() });
      try {
        const modeId = validateSortModeId(input.modeId);
        const modeUuid = validateSortModeUuid(input.modeUuid);
        void queryClient.invalidateQueries({
          predicate: (query) => keyMatchesModeIdentity(query.queryKey, modeId, modeUuid),
        });
      } catch (error) {
        if (error instanceof Error && error.message.includes("SEC_INVALID_INPUT")) return;
        throw error;
      }
    },
  });
}

export function useSortModeProvidersSetOrderMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: {
      modeId: number;
      modeUuid: string;
      cliKey: CliKey;
      orderedProviderIds: number[];
    }) =>
      sortModeProvidersSetOrder({
        mode_id: validateSortModeId(input.modeId),
        cli_key: validateProviderCliKey(input.cliKey),
        ordered_provider_ids: input.orderedProviderIds,
      }),
    onSettled: (_data, _error, input) => {
      try {
        const cliKey = validateProviderCliKey(input.cliKey);
        const modeId = validateSortModeId(input.modeId);
        const modeUuid = validateSortModeUuid(input.modeUuid);
        void queryClient.invalidateQueries({
          queryKey: sortModeProvidersQueryKey(modeId, modeUuid, cliKey),
        });
        void invalidateRoutingEditorForMode(queryClient, { cliKey, modeId, modeUuid });
      } catch (error) {
        if (error instanceof Error && error.message.includes("SEC_INVALID_INPUT")) return;
        throw error;
      }
    },
  });
}

export function useSortModeProviderSetEnabledMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: {
      modeId: number;
      modeUuid: string;
      cliKey: CliKey;
      providerId: number;
      enabled: boolean;
    }) =>
      sortModeProviderSetEnabled({
        mode_id: validateSortModeId(input.modeId),
        cli_key: validateProviderCliKey(input.cliKey),
        provider_id: input.providerId,
        enabled: input.enabled,
      }),
    onSettled: (_data, _error, input) => {
      try {
        const cliKey = validateProviderCliKey(input.cliKey);
        const modeId = validateSortModeId(input.modeId);
        const modeUuid = validateSortModeUuid(input.modeUuid);
        void queryClient.invalidateQueries({
          queryKey: sortModeProvidersQueryKey(modeId, modeUuid, cliKey),
        });
        void invalidateRoutingEditorForMode(queryClient, { cliKey, modeId, modeUuid });
      } catch (error) {
        if (error instanceof Error && error.message.includes("SEC_INVALID_INPUT")) return;
        throw error;
      }
    },
  });
}

export function useSortModeProviderSetSessionReusePriorityMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: {
      modeId: number;
      modeUuid: string;
      cliKey: CliKey;
      providerId: number;
      sessionReusePriority: number;
    }) =>
      sortModeProviderSetSessionReusePriority({
        mode_id: validateSortModeId(input.modeId),
        cli_key: validateProviderCliKey(input.cliKey),
        provider_id: input.providerId,
        session_reuse_priority: input.sessionReusePriority,
      }),
    onSettled: (_data, _error, input) => {
      try {
        const cliKey = validateProviderCliKey(input.cliKey);
        const modeId = validateSortModeId(input.modeId);
        const modeUuid = validateSortModeUuid(input.modeUuid);
        void queryClient.invalidateQueries({
          queryKey: sortModeProvidersQueryKey(modeId, modeUuid, cliKey),
        });
      } catch (error) {
        if (error instanceof Error && error.message.includes("SEC_INVALID_INPUT")) return;
        throw error;
      }
    },
  });
}
