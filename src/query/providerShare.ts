import { useMutation, useQueryClient } from "@tanstack/react-query";
import { providerShareImportConfirm } from "../services/providers/providerShare";
import type { ProviderSummary } from "../services/providers/providers";
import { providersKeys } from "./keys";
import { invalidateRoutingEditorForCli } from "./sortModes";

export function useProviderShareImportMutation() {
  const queryClient = useQueryClient();

  return useMutation<ProviderSummary, Error, { previewToken: string }>({
    mutationFn: ({ previewToken }) => providerShareImportConfirm(previewToken),
    onSuccess: async (imported) => {
      queryClient.setQueryData<ProviderSummary[] | null>(
        providersKeys.list(imported.cli_key),
        (previous) => {
          if (!previous) return [imported];
          return [...previous.filter((provider) => provider.id !== imported.id), imported];
        }
      );
      void queryClient.invalidateQueries({ queryKey: providersKeys.list(imported.cli_key) });
      await invalidateRoutingEditorForCli(queryClient, imported.cli_key);
    },
  });
}
