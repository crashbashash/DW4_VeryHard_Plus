import { useCallback, useState } from "react";
import type { DiscStatus } from "../../bindings";
import type { Backend } from "../../ipc/backend";

/**
 * The disc side of the app state: the chosen input, the suggested output,
 * and the analysis verdict. Picking a disc is all it takes to start
 * analyzing — automatic, as in the egui app.
 */
export function useDisc(backend: Backend) {
  const [inputPath, setInputPathState] = useState<string | null>(null);
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [status, setStatus] = useState<DiscStatus | null>(null);
  const [analyzing, setAnalyzing] = useState(false);

  const setInputPath = useCallback(
    async (path: string) => {
      setInputPathState(path);
      setStatus(null);
      const output = await backend.defaultOutput(path);
      setOutputPath(output);
      setAnalyzing(true);
      try {
        setStatus(await backend.analyze(path));
      } finally {
        setAnalyzing(false);
      }
    },
    [backend],
  );

  const pickDisc = useCallback(async () => {
    const path = await backend.chooseIso();
    if (path !== null) {
      await setInputPath(path);
    }
  }, [backend, setInputPath]);

  return { inputPath, outputPath, status, analyzing, pickDisc, setInputPath, setOutputPath };
}
