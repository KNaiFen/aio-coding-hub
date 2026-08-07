import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { useWslHostAddressQuery } from "../../../query/wsl";
import { copyText } from "../../../services/clipboard";
import {
  gatewayBearerTokenAcknowledge,
  gatewayBearerTokenReveal,
  gatewayBearerTokenRotate,
} from "../../../services/gateway/gateway";
import { NetworkSettingsCard } from "../NetworkSettingsCard";

let gatewayMetaMock: any = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/clipboard", () => ({ copyText: vi.fn() }));

vi.mock("../../../services/gateway/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../../services/gateway/gateway")>(
    "../../../services/gateway/gateway"
  );
  return {
    ...actual,
    gatewayBearerTokenReveal: vi.fn(),
    gatewayBearerTokenRotate: vi.fn(),
    gatewayBearerTokenAcknowledge: vi.fn(),
  };
});

vi.mock("../../../hooks/useGatewayMeta", () => ({
  useGatewayMeta: () => gatewayMetaMock,
}));

vi.mock("../../../query/wsl", async () => {
  const actual = await vi.importActual<typeof import("../../../query/wsl")>("../../../query/wsl");
  return { ...actual, useWslHostAddressQuery: vi.fn() };
});

beforeEach(() => {
  vi.mocked(gatewayBearerTokenReveal).mockReset().mockResolvedValue(null);
  vi.mocked(gatewayBearerTokenRotate).mockReset();
  vi.mocked(gatewayBearerTokenAcknowledge).mockReset().mockResolvedValue(true);
  vi.mocked(copyText).mockReset().mockResolvedValue(undefined);
});

describe("components/cli-manager/NetworkSettingsCard", () => {
  it("reveals, copies, confirms, and rotates the non-loopback Gateway token", async () => {
    const firstToken = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const rotatedToken = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
    vi.mocked(gatewayBearerTokenReveal)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({
        token: firstToken,
        wsl_sync_error: "WSL_GATEWAY_TOKEN_SYNC_FAILED: review WSL settings",
      });
    vi.mocked(gatewayBearerTokenRotate).mockResolvedValue({
      token: rotatedToken,
      wsl_sync_error: null,
    });

    const settings = {
      preferred_port: 37123,
      gateway_listen_mode: "localhost",
      gateway_custom_listen_address: "",
    } as any;
    const onPersistSettings = vi.fn(async () => ({
      ...settings,
      gateway_listen_mode: "lan",
    }));

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
      />
    );

    fireEvent.change(screen.getByRole("combobox"), { target: { value: "lan" } });
    expect(await screen.findByText(firstToken)).toBeInTheDocument();
    expect(screen.getByText(/WSL_GATEWAY_TOKEN_SYNC_FAILED/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "复制" }));
    await waitFor(() => expect(copyText).toHaveBeenCalledWith(firstToken));
    fireEvent.click(screen.getByRole("button", { name: "已保存" }));
    await waitFor(() => expect(gatewayBearerTokenAcknowledge).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getByRole("button", { name: "轮换访问令牌" }));
    expect(await screen.findByText(rotatedToken)).toBeInTheDocument();
  });

  it("switches listen mode and validates custom address", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);

    gatewayMetaMock = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

    const settings = {
      preferred_port: 37123,
      gateway_listen_mode: "custom",
      gateway_custom_listen_address: "0.0.0.0:37123",
    } as any;

    const onPersistSettings = vi.fn(async () => settings);

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
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
      />
    );

    expect(screen.getByText("1.2.3.4:9999")).toBeInTheDocument();
  });

  it("persists listen mode changes and reverts local state when save fails", async () => {
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
      .mockResolvedValueOnce({ ...settings, gateway_listen_mode: "lan" })
      .mockRejectedValueOnce(new Error("save boom"));

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
      />
    );

    fireEvent.change(screen.getByRole("combobox"), { target: { value: "lan" } });
    await waitFor(() =>
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "lan" })
    );
    expect(toast).toHaveBeenCalledWith("监听模式已保存");
    expect(screen.getByText("0.0.0.0:40000")).toBeInTheDocument();

    vi.mocked(toast).mockClear();
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "wsl_auto" } });
    await waitFor(() =>
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "wsl_auto" })
    );
    await waitFor(() => expect(toast).toHaveBeenCalledWith("更新监听模式失败：请稍后重试"));
    await waitFor(() =>
      expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("localhost")
    );
  });

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
