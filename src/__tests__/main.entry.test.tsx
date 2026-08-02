import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Root } from "react-dom/client";
import type { ReactNode } from "react";

vi.mock("../App", () => ({
  default: () => <div data-testid="main-entry-app">mock app</div>,
}));

vi.mock("../tray/TrayProviderMiniApp", () => ({
  TrayProviderMiniApp: () => <div data-testid="tray-provider-mini-entry">tray mini</div>,
}));

vi.mock("../components/AppErrorBoundary", () => ({
  AppErrorBoundary: ({ children }: { children: ReactNode }) => children,
}));

vi.mock("../services/frontendErrorReporter", async () => {
  const actual = await vi.importActual<typeof import("../services/frontendErrorReporter")>(
    "../services/frontendErrorReporter"
  );
  return {
    ...actual,
    installGlobalErrorReporting: vi.fn(),
  };
});

let appRoot: Root | null = null;

async function importMainEntry() {
  const mainModule = await import("../main");
  appRoot = mainModule.appRoot;
  await new Promise((resolve) => setTimeout(resolve, 0));
  return mainModule;
}

describe("main entry", () => {
  beforeEach(() => {
    vi.resetModules();
    window.history.replaceState(null, "", "/");
    document.documentElement.classList.remove("tray-provider-mini-window");
  });

  afterEach(async () => {
    appRoot?.unmount();
    appRoot = null;
    await new Promise((resolve) => setTimeout(resolve, 0));
  });

  it("renders without crashing", async () => {
    document.body.innerHTML = '<div id="root"></div>';

    await importMainEntry();

    expect(document.querySelector("[data-testid='main-entry-app']")).toBeInTheDocument();
    expect(document.documentElement).not.toHaveClass("tray-provider-mini-window");
  }, 30000);

  it("registers global frontend error handlers", async () => {
    document.body.innerHTML = '<div id="root"></div>';

    const reporter = await import("../services/frontendErrorReporter");
    await importMainEntry();

    expect(reporter.installGlobalErrorReporting).toHaveBeenCalled();
  }, 30000);

  it("renders only the tray mini root for the tray window query", async () => {
    document.body.innerHTML = '<div id="root"></div>';
    window.history.replaceState(null, "", "/?window=tray-provider-mini");

    await importMainEntry();

    expect(document.querySelector("[data-testid='tray-provider-mini-entry']")).toBeInTheDocument();
    expect(document.querySelector("[data-testid='main-entry-app']")).not.toBeInTheDocument();
    expect(document.documentElement).toHaveClass("tray-provider-mini-window");
  }, 30000);
});
