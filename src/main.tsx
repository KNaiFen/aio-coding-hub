import React from "react";
import ReactDOM from "react-dom/client";
import type { Root } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { AppErrorBoundary } from "./components/AppErrorBoundary";
import "@mdxeditor/editor/style.css";
import "./styles/globals.css";
import { queryClient } from "./query/queryClient";
import { installGlobalErrorReporting } from "./services/frontendErrorReporter";
import { TrayProviderMiniApp } from "./tray/TrayProviderMiniApp";
import { isTrayProviderMiniWindow } from "./tray/windowMode";

export function renderApp(rootElement: HTMLElement): Root {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <AppErrorBoundary>
          <App />
        </AppErrorBoundary>
      </QueryClientProvider>
    </React.StrictMode>
  );
  return root;
}

export function renderTrayProviderMini(rootElement: HTMLElement): Root {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <AppErrorBoundary>
        <TrayProviderMiniApp />
      </AppErrorBoundary>
    </React.StrictMode>
  );
  return root;
}

installGlobalErrorReporting();

const rootElement = document.getElementById("root") as HTMLElement;
export const appRoot = isTrayProviderMiniWindow(window.location.search)
  ? renderTrayProviderMini(rootElement)
  : renderApp(rootElement);
