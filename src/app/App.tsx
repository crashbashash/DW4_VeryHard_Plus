import { useEffect, useState } from "react";
import type { PatchOutcome, PlanSummary, ProgressEvent } from "../bindings";
import { AdvancedPanel } from "../features/advanced/AdvancedPanel";
import { DiscPicker } from "../features/disc/DiscPicker";
import { useDisc } from "../features/disc/useDisc";
import { OutputPanel } from "../features/output/OutputPanel";
import { PresetCards } from "../features/presets/PresetCards";
import { presetPlan } from "../features/presets/presetData";
import { usePlan } from "../features/presets/usePlan";
import { BackendProvider, useBackend } from "../ipc/context";
import "./theme.css";
import "./layout.css";

function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() => window.matchMedia(query).matches);
  useEffect(() => {
    const mql = window.matchMedia(query);
    const onChange = (e: MediaQueryListEvent) => setMatches(e.matches);
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [query]);
  return matches;
}

/**
 * The app shell: disc picking and presets on one side, the Advanced dials and
 * output on the other (two columns in a wide window, one column otherwise).
 * The plan lives in one store, so the preset cards and Advanced panel can
 * never disagree — that was the egui bug.
 */
export function App() {
  return (
    <BackendProvider>
      <AppInner />
    </BackendProvider>
  );
}

function AppInner() {
  const backend = useBackend();
  const plan = usePlan();
  const disc = useDisc(backend);
  const wide = useMediaQuery("(min-width: 900px)");

  const [overwrite, setOverwrite] = useState(false);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [busy, setBusy] = useState(false);
  const [log, setLog] = useState<string[]>([]);
  const [summary, setSummary] = useState<PlanSummary | null>(null);
  const [lastOutcome, setLastOutcome] = useState<PatchOutcome | null>(null);

  // Live plan summary: recomputed on every plan change once a disc is analysed.
  useEffect(() => {
    let stale = false;
    backend
      .planSummary(plan.state.plan)
      .then((s) => {
        if (!stale) setSummary(s);
      })
      .catch(() => {
        if (!stale) setSummary(null);
      });
    return () => {
      stale = true;
    };
  }, [backend, plan.state.plan, disc.status]);

  // Patch events: progress streams, one terminal event ends the run.
  useEffect(() => {
    const ups: Promise<() => void>[] = [];
    ups.push(
      backend.onPatchProgress((p) => {
        setProgress(p);
        setBusy(true);
      }),
    );
    ups.push(
      backend.onPatchDone((o) => {
        setLastOutcome(o);
        setProgress(null);
        setBusy(false);
        setLog((lines) => [
          ...lines,
          `patched ${o.bytes.toLocaleString()} bytes, md5 ${o.md5}`,
        ]);
      }),
    );
    ups.push(
      backend.onPatchFailed((message) => {
        setProgress(null);
        setBusy(false);
        setLog((lines) => [...lines, `patch refused: ${message}`]);
      }),
    );
    return () => {
      for (const up of ups) void up.then((unlisten) => unlisten());
    };
  }, [backend]);

  // An ISO dropped onto the window is a chosen disc.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    void backend.onFileDropped((path) => void disc.setInputPath(path)).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [backend, disc]);

  const analyzed = disc.status?.kind === "ok";
  const canPatch = analyzed && disc.outputPath !== null;

  const startPatch = async () => {
    if (disc.outputPath === null) return;
    setBusy(true);
    setLog((lines) => [
      ...lines,
      `patching ${disc.inputPath} -> ${disc.outputPath}`,
    ]);
    try {
      await backend.startPatch(plan.state.plan, disc.outputPath, overwrite);
    } catch (error) {
      setBusy(false);
      setLog((lines) => [...lines, `patch refused: ${String(error)}`]);
    }
  };

  const chooseOutput = async () => {
    const path = await backend.chooseOutput(
      disc.outputPath?.split("/").pop() ?? "game [VeryHardPlus].iso",
    );
    if (path !== null) disc.setOutputPath(path);
  };

  const setOutput = (path: string) => {
    if (disc.inputPath === null) disc.setOutputPath(path);
  };

  return (
    <>
      <header className="app-header">
        <h1>DW4 Very Hard Plus</h1>
        <p className="app-subtitle">
          Turn your own clean Digimon World 4 (USA) disc into a much harder one.
        </p>
      </header>
      <main className={`app-main ${wide ? "layout-two-col" : "layout-one-col"}`}>
        <div className="col">
          <DiscPicker
            inputPath={disc.inputPath}
            outputPath={disc.outputPath}
            status={disc.status}
            analyzing={disc.analyzing}
            onPick={() => void disc.pickDisc()}
            onSetOutput={setOutput}
          />
          <PresetCards preset={plan.state.preset} onPick={plan.applyPreset} />
        </div>
        <div className="col">
          <AdvancedPanel
            state={plan.state}
            onEditField={plan.editField}
            onSetSerialEnabled={plan.setSerialEnabled}
            onSetSerialText={plan.setSerialText}
          />
          <OutputPanel
            outputPath={disc.outputPath}
            overwrite={overwrite}
            busy={busy || disc.analyzing}
            canPatch={canPatch}
            progress={progress}
            log={log}
            status={disc.status}
            summary={summary}
            lastOutcome={lastOutcome}
            onPatch={() => void startPatch()}
            onChooseOutput={() => void chooseOutput()}
            onSetOutput={setOutput}
            onSetOverwrite={setOverwrite}
          />
        </div>
      </main>
    </>
  );
}

// presetPlan is referenced so the preset table ships with the bundle even
// before the first interaction (tree-shaking keeps it either way).
void presetPlan;
