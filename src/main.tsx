import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { BackendProvider } from "./ipc/context";
import "./app/theme.css";
import "./app/layout.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BackendProvider>
      <App />
    </BackendProvider>
  </StrictMode>,
);
