import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { http, HttpResponse } from "msw";
import { server } from "../test/msw/server";
import { TAURI_ENDPOINT } from "../test/tauriEndpoint";
import { tauriDialogOpen } from "../test/mocks/tauri";
import { responseCommitPlugin, responseCommitPreview } from "../test/fixtures/responseCommitPlugin";
import { PluginsPage } from "../pages/PluginsPage";
import { createTestQueryClient } from "../test/utils/reactQuery";

function renderPluginsPage() {
  return render(
    <QueryClientProvider client={createTestQueryClient()}>
      <MemoryRouter>
        <PluginsPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe("plugins e2e smoke", () => {
  it("installs and toggles an ordinary commit policy through page, query, service and mock IPC", async () => {
    let plugin = responseCommitPlugin();
    let installed = false;
    let rejectEnable = true;
    const enableCalls = vi.fn();
    server.use(
      http.post(`${TAURI_ENDPOINT}/plugin_list`, () =>
        HttpResponse.json(installed ? [plugin.summary] : [])
      ),
      http.post(`${TAURI_ENDPOINT}/plugin_get`, () => HttpResponse.json(plugin)),
      http.post(`${TAURI_ENDPOINT}/plugin_preview_from_file`, () =>
        HttpResponse.json(responseCommitPreview(plugin))
      ),
      http.post(`${TAURI_ENDPOINT}/plugin_install_from_file`, async ({ request }) => {
        expect(await request.json()).toEqual({ input: { filePath: "/tmp/policy.aio-plugin" } });
        installed = true;
        return HttpResponse.json(plugin);
      }),
      http.post(`${TAURI_ENDPOINT}/plugin_enable`, async ({ request }) => {
        enableCalls(await request.json());
        if (rejectEnable)
          return HttpResponse.json({ error: "validator unavailable" }, { status: 409 });
        plugin = { ...plugin, summary: { ...plugin.summary, status: "enabled" } };
        return HttpResponse.json(plugin);
      }),
      http.post(`${TAURI_ENDPOINT}/plugin_disable`, () => {
        plugin = { ...plugin, summary: { ...plugin.summary, status: "disabled" } };
        return HttpResponse.json(plugin);
      })
    );
    tauriDialogOpen.mockResolvedValueOnce("/tmp/policy.aio-plugin");
    renderPluginsPage();
    fireEvent.click(await screen.findByRole("button", { name: /导入 \.aio-plugin/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("完整响应校验")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: /确认安装/ }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    const card = (await screen.findAllByText(plugin.summary.name))[0].closest("article")!;
    expect(within(card).getByText("已关闭")).toBeInTheDocument();
    fireEvent.click(within(card).getByRole("button", { name: /^启用$/ }));
    await waitFor(() => expect(enableCalls).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(within(card).getByRole("button", { name: /^启用$/ })).toBeEnabled());
    expect(within(card).getByText("已关闭")).toBeInTheDocument();
    rejectEnable = false;
    fireEvent.click(within(card).getByRole("button", { name: /^启用$/ }));
    await waitFor(() => expect(within(card).getByText("运行中")).toBeInTheDocument());
    expect(enableCalls).toHaveBeenLastCalledWith({ input: { pluginId: plugin.summary.plugin_id } });
    expect(screen.getByText(/首字等待会更长/)).toBeInTheDocument();
    fireEvent.click(within(card).getByRole("button", { name: /^禁用$/ }));
    await waitFor(() => expect(within(card).getByText("已关闭")).toBeInTheDocument());
  });

  it("installs and displays the official Privacy Filter through the desktop IPC bridge", async () => {
    renderPluginsPage();

    const privacyFilterCard = (await screen.findByText("Privacy Filter")).closest("article");
    expect(privacyFilterCard).not.toBeNull();

    fireEvent.click(
      within(privacyFilterCard as HTMLElement).getByRole("button", {
        name: /^安装$/,
      })
    );

    await waitFor(() => {
      expect(screen.getAllByText("Privacy Filter").length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText("official.privacy-filter").length).toBeGreaterThan(0);
    expect(await screen.findByText("gateway.request.afterBodyRead")).toBeInTheDocument();
    expect(await screen.findByText("log.beforePersist")).toBeInTheDocument();
    expect(await screen.findByText("Plugin installed")).toBeInTheDocument();
  });
});
