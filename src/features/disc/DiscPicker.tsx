import { useRef } from "react";

export interface DiscPickerProps {
  inputPath: string | null;
  outputPath: string | null;
  status: { kind: "ok" | "refused" | "unknown"; text: string } | null;
  analyzing: boolean;
  onPick: () => void;
  onSetOutput: (path: string) => void;
}

/** The disc row: input path, Browse, drag-drop hint, verdict line. */
export function DiscPicker({
  inputPath,
  outputPath,
  status,
  analyzing,
  onPick,
  onSetOutput,
}: DiscPickerProps) {
  const fileInput = useRef<HTMLInputElement>(null);

  return (
    <section className="panel">
      <h2>Disc</h2>
      <div className="row">
        <input
          className="path-field"
          type="text"
          value={inputPath ?? ""}
          placeholder="path to the ISO"
          aria-label="disc path"
          onChange={(e) => onSetOutput(e.target.value)}
          readOnly={inputPath !== null}
        />
        <button type="button" onClick={() => (inputPath === null ? onPick() : onPick())}>
          Browse…
        </button>
        <button type="button" className="hidden-file-btn" onClick={() => fileInput.current?.click()}>
          <input
            ref={fileInput}
            type="file"
            accept=".iso"
            hidden
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) onSetOutput(f.name);
            }}
          />
        </button>
      </div>
      {inputPath === null && <p className="hint">choose a disc (Browse, or drag the ISO onto this window)</p>}
      <p className="hint">or drag and drop the ISO onto this window</p>
      {analyzing && <p className="status-line status-unknown">analyzing…</p>}
      {status !== null && (
        <p className={`status-line status-${status.kind}`}>{status.text}</p>
      )}
      {outputPath !== null && <p className="hint">output: {outputPath}</p>}
    </section>
  );
}
