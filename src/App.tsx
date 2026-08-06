import type { CSSProperties } from "react";
import { Toaster } from "sonner";
import { HashRouter } from "react-router-dom";
import { AppRoutes } from "./app/AppRoutes";
import { useInitializeAppSession } from "./app/appSession";
import { AppRuntimeServices, useAppBootstrap } from "./app/useAppBootstrap";
import { useGlobalFileDropGuard } from "./app/useGlobalFileDropGuard";
import { AppMaintenanceScreen } from "./components/app/AppMaintenanceScreen";
import { Spinner } from "./ui/Spinner";

type CssVarsStyle = CSSProperties & Record<`--toast-${string}`, string | number>;

const TOASTER_STYLE: CssVarsStyle = {
  "--toast-close-button-start": "unset",
  "--toast-close-button-end": "0",
  "--toast-close-button-transform": "translate(35%, -35%)",
};

export default function App() {
  useInitializeAppSession();
  const { status, synchronized } = useAppBootstrap();
  useGlobalFileDropGuard();

  const inMaintenance = synchronized && status.maintenanceMode;
  const runtimeReady = synchronized && !inMaintenance && status.currentStage === "ready";
  const canRenderRoutes =
    synchronized &&
    !inMaintenance &&
    (status.currentStage === "ready" || status.currentStage === "failed");

  return (
    <>
      <Toaster richColors closeButton position="top-center" style={TOASTER_STYLE} />
      {runtimeReady ? <AppRuntimeServices /> : null}
      {inMaintenance ? (
        <AppMaintenanceScreen status={status} />
      ) : canRenderRoutes ? (
        <HashRouter>
          <AppRoutes />
        </HashRouter>
      ) : (
        <main className="flex min-h-screen items-center justify-center bg-background text-foreground">
          <Spinner />
        </main>
      )}
    </>
  );
}
