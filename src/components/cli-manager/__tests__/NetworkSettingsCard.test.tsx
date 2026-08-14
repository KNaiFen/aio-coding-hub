import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { useWslHostAddressQuery } from "../../../query/wsl";
import type { AppSettings } from "../../../services/settings/settings";
import { createTestAppSettings } from "../../../test/fixtures/settings";
import { createDeferred } from "../../../test/utils/deferred";
import { NetworkSettingsCard, type NetworkSettingsCardProps } from "../NetworkSettingsCard";

let gatewayMetaMock: any = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../../hooks/useGatewayMeta", () => ({
  useGatewayMeta: () => gatewayMetaMock,
}));

vi.mock("../../../query/wsl", async () => {
  const actual = await vi.importActual<typeof import("../../../query/wsl")>("../../../query/wsl");
  return { ...actual, useWslHostAddressQuery: vi.fn() };
});

beforeEach(() => {
  vi.clearAllMocks();
  gatewayMetaMock = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };
});

function createTokenControllerProps(
  overrides: Partial<
    Pick<
      NetworkSettingsCardProps,
      "gatewayTokenActionPending" | "onGatewayListenSaved" | "onRotateGatewayToken"
    >
  > = {}
) {
  return {
    gatewayTokenActionPending: false,
    onGatewayListenSaved: vi.fn().mockResolvedValue(undefined),
    onRotateGatewayToken: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

const persistOutcomes = ["success", "null", "error"] as const;

describe("components/cli-manager/NetworkSettingsCard", () => {
  it("shows applying state, commits canonical settings, and can switch LAN back to localhost", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);

    const settings = createTestAppSettings({
      preferred_port: 37123,
      gateway_listen_mode: "localhost",
      gateway_custom_listen_address: "",
    });
    const firstSave = createDeferred<AppSettings | null>();
    const onPersistSettings = vi
      .fn()
      .mockReturnValueOnce(firstSave.promise)
      .mockResolvedValueOnce({ ...settings, gateway_listen_mode: "localhost" });
    const onGatewayListenSaved = vi.fn().mockResolvedValue(undefined);

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
        {...createTokenControllerProps({ onGatewayListenSaved })}
      />
    );

    const modeSelect = screen.getByRole("combobox") as HTMLSelectElement;
    fireEvent.change(modeSelect, { target: { value: "lan" } });
    expect(screen.getByRole("status")).toHaveTextContent("正在应用");
    expect(modeSelect).toBeDisabled();

    await act(async () => {
      firstSave.resolve({ ...settings, gateway_listen_mode: "lan" });
      await firstSave.promise;
    });
    await waitFor(() => expect(modeSelect.value).toBe("lan"));
    expect(modeSelect).not.toBeDisabled();
    expect(onGatewayListenSaved).toHaveBeenCalledTimes(1);

    fireEvent.change(modeSelect, { target: { value: "localhost" } });
    await waitFor(() => expect(modeSelect.value).toBe("localhost"));
    expect(onPersistSettings).toHaveBeenLastCalledWith({ gateway_listen_mode: "localhost" });
    expect(onGatewayListenSaved).toHaveBeenCalledTimes(1);
  });

  it("switches listen mode and validates custom address", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);

    gatewayMetaMock = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

    const settings = {
      preferred_port: 37123,
      gateway_listen_mode: "custom",
      gateway_custom_listen_address: "0.0.0.0:37123",
    } as any;

    const onPersistSettings = vi.fn(async (patch) => ({ ...settings, ...patch }));

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
        {...createTokenControllerProps()}
      />
    );

    expect(screen.getByText("网络设置")).toBeInTheDocument();

    // Switch to WSL auto mode -> should use host IP.
    const modeSelect = screen.getByRole("combobox");
    fireEvent.change(modeSelect, { target: { value: "wsl_auto" } });
    await waitFor(() => {
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "wsl_auto" });
    });
    expect(screen.getByText("172.20.0.1:37123")).toBeInTheDocument();

    // Switch back to custom and enter an invalid address -> input resets on blur.
    fireEvent.change(modeSelect, { target: { value: "custom" } });
    const input = screen.getByPlaceholderText("0.0.0.0 或 0.0.0.0:37123");
    fireEvent.change(input, { target: { value: "http://bad" } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect((input as HTMLInputElement).value).toBe("0.0.0.0:37123");
    });
  });

  it("prefers live gateway listen_addr when running", () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
    gatewayMetaMock = {
      gatewayAvailable: "available",
      gateway: { running: true, listen_addr: "1.2.3.4:9999" },
      preferredPort: 37123,
    };

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={
          {
            preferred_port: 37123,
            gateway_listen_mode: "localhost",
            gateway_custom_listen_address: "",
          } as any
        }
        onPersistSettings={vi.fn(async () => null)}
        {...createTokenControllerProps()}
      />
    );

    expect(screen.getByText("1.2.3.4:9999")).toBeInTheDocument();
  });

  it("rolls listen mode back to the latest settings on null and error", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);
    gatewayMetaMock = {
      gatewayAvailable: "available",
      gateway: { running: true, listen_addr: null },
      preferredPort: 37123,
    };

    const settings = {
      preferred_port: 40000,
      gateway_listen_mode: "localhost",
      gateway_custom_listen_address: "",
    } as any;
    const onPersistSettings = vi
      .fn()
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error("save boom"));
    const onGatewayListenSaved = vi.fn().mockResolvedValue(undefined);

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
        {...createTokenControllerProps({ onGatewayListenSaved })}
      />
    );

    fireEvent.change(screen.getByRole("combobox"), { target: { value: "lan" } });
    await waitFor(() =>
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "lan" })
    );
    await waitFor(() =>
      expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("localhost")
    );

    vi.mocked(toast).mockClear();
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "wsl_auto" } });
    await waitFor(() =>
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "wsl_auto" })
    );
    await waitFor(() => expect(toast).toHaveBeenCalledWith("更新监听模式失败：请稍后重试"));
    await waitFor(() =>
      expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("localhost")
    );
    expect(onGatewayListenSaved).not.toHaveBeenCalled();
  });

  it("syncs external settings in an effect and delegates token rotation", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
    const settings = createTestAppSettings({
      preferred_port: 37123,
      gateway_listen_mode: "localhost",
      gateway_custom_listen_address: "",
    });
    const onRotateGatewayToken = vi.fn().mockResolvedValue(undefined);
    const controllerProps = createTokenControllerProps({ onRotateGatewayToken });
    const view = render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={vi.fn(async () => null)}
        {...controllerProps}
      />
    );

    view.rerender(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={{ ...settings, gateway_listen_mode: "lan" }}
        onPersistSettings={vi.fn(async () => null)}
        {...controllerProps}
      />
    );

    await waitFor(() =>
      expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("lan")
    );
    fireEvent.click(screen.getByRole("button", { name: "轮换访问令牌" }));
    await waitFor(() => expect(onRotateGatewayToken).toHaveBeenCalledTimes(1));
  });

  it.each(persistOutcomes)(
    "adopts an external listen mode that arrives while a %s save is applying",
    async (outcome) => {
      vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);
      const settings = createTestAppSettings({
        gateway_listen_mode: "localhost",
        gateway_custom_listen_address: "",
      });
      const externalSettings = { ...settings, gateway_listen_mode: "wsl_auto" } as AppSettings;
      const save = createDeferred<AppSettings | null>();
      const onPersistSettings = vi.fn().mockReturnValue(save.promise);
      const controllerProps = createTokenControllerProps();
      const view = render(
        <NetworkSettingsCard
          available={true}
          saving={false}
          settings={settings}
          onPersistSettings={onPersistSettings}
          {...controllerProps}
        />
      );

      fireEvent.change(screen.getByRole("combobox"), { target: { value: "lan" } });
      expect(screen.getByRole("status")).toHaveTextContent("正在应用");
      view.rerender(
        <NetworkSettingsCard
          available={true}
          saving={false}
          settings={externalSettings}
          onPersistSettings={onPersistSettings}
          {...controllerProps}
        />
      );
      expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("lan");

      await act(async () => {
        if (outcome === "success") {
          save.resolve({ ...settings, gateway_listen_mode: "lan" });
        } else if (outcome === "null") {
          save.resolve(null);
        } else {
          save.reject(new Error("save boom"));
        }
      });

      await waitFor(() =>
        expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("wsl_auto")
      );
    }
  );

  it.each(persistOutcomes)(
    "adopts an external custom address that arrives while a %s save is applying",
    async (outcome) => {
      vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
      const settings = createTestAppSettings({
        gateway_listen_mode: "custom",
        gateway_custom_listen_address: "0.0.0.0:37123",
      });
      const externalSettings = {
        ...settings,
        gateway_custom_listen_address: "192.168.1.20:37123",
      };
      const save = createDeferred<AppSettings | null>();
      const onPersistSettings = vi.fn().mockReturnValue(save.promise);
      const controllerProps = createTokenControllerProps();
      const view = render(
        <NetworkSettingsCard
          available={true}
          saving={false}
          settings={settings}
          onPersistSettings={onPersistSettings}
          {...controllerProps}
        />
      );

      const input = screen.getByPlaceholderText("0.0.0.0 或 0.0.0.0:37123");
      fireEvent.change(input, { target: { value: "10.0.0.2:37123" } });
      fireEvent.blur(input);
      expect(screen.getByRole("status")).toHaveTextContent("正在应用");
      view.rerender(
        <NetworkSettingsCard
          available={true}
          saving={false}
          settings={externalSettings}
          onPersistSettings={onPersistSettings}
          {...controllerProps}
        />
      );
      expect((input as HTMLInputElement).value).toBe("10.0.0.2:37123");

      await act(async () => {
        if (outcome === "success") {
          save.resolve({
            ...settings,
            gateway_custom_listen_address: "10.0.0.2:37123",
          });
        } else if (outcome === "null") {
          save.resolve(null);
        } else {
          save.reject(new Error("save boom"));
        }
      });

      await waitFor(() =>
        expect((input as HTMLInputElement).value).toBe("192.168.1.20:37123")
      );
    }
  );

  it("validates IPv6 custom address and handles non-tauri persist failure", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
    gatewayMetaMock = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

    const settings = {
      preferred_port: 37123,
      gateway_listen_mode: "custom",
      gateway_custom_listen_address: "0.0.0.0:37123",
    } as any;

    const onPersistSettings = vi.fn(async () => null);

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
        {...createTokenControllerProps()}
      />
    );

    const input = screen.getByPlaceholderText("0.0.0.0 或 0.0.0.0:37123");
    fireEvent.change(input, { target: { value: "[::1]" } });
    fireEvent.blur(input);

    await waitFor(() => expect(onPersistSettings).toHaveBeenCalled());
    await waitFor(() => expect((input as HTMLInputElement).value).toBe("0.0.0.0:37123"));

    vi.mocked(toast).mockClear();
    fireEvent.change(input, { target: { value: "0.0.0.0:80" } });
    fireEvent.blur(input);
    await waitFor(() => expect(toast).toHaveBeenCalledWith("端口必须 >= 1024"));
  });
});
