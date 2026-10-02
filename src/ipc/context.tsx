import { createContext, useContext, useMemo, type ReactNode } from "react";
import { TauriBackend } from "./tauri";
import { MockBackend } from "./mock";
import type { Backend } from "./backend";

const BackendContext = createContext<Backend | null>(null);

export function BackendProvider({ children }: { children: ReactNode }) {
  const backend = useMemo(() => {
    // Tauri sets this global on the webview; a plain browser tab does not.
    if ("__TAURI_INTERNALS__" in window) {
      return new TauriBackend() as Backend;
    }
    return new MockBackend();
  }, []);
  return <BackendContext.Provider value={backend}>{children}</BackendContext.Provider>;
}

/** The shell seam. Throws when a component renders outside the provider. */
export function useBackend(): Backend {
  const backend = useContext(BackendContext);
  if (backend === null) {
    throw new Error("useBackend outside <BackendProvider>");
  }
  return backend;
}
