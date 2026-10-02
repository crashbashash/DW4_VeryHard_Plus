import type { DiscStatus, PatchOutcome, PlanSummary, ProgressEvent } from "../../bindings";

export interface OutputPanelProps {
  outputPath: string | null;
  overwrite: boolean;
  busy: boolean;
  canPatch: boolean;
  progress: ProgressEvent | null;
  log: string[];
  status: DiscStatus | null;
  summary: PlanSummary | null;
  lastOutcome: PatchOutcome | null;
  onPatch: () => void;
  onChooseOutput: () => void;
  onSetOutput: (path: string) => void;
  onSetOverwrite: (overwrite: boolean) => void;
}

/** The attack column's i16 ceiling, as the note states it. */
const ATTACK_CEILING = 32767;

/** The old form.rs notes, ported verbatim. */
function attackPinNote(summary: PlanSummary): string | null {
  return summary.attackPinned > 0
    ? `attack: ${summary.attackPinned} of ${summary.liveRows} live rows pinned at ${ATTACK_CEILING}`
    : null;
}

function collapseNote(summary: PlanSummary): string | null {
  return summary.collapsedRows > 0
    ? `collapse: ${summary.collapsedRows} rows now carry their type's strongest row — every enemy of a type is identical, EXP included`
    : null;
}

/** Output path, overwrite, the one Patch button, progress and the log. */
export function OutputPanel({
  outputPath,
  overwrite,
  busy,
  canPatch,
  progress,
  log,
  status,
  summary,
  lastOutcome,
  onPatch,
  onChooseOutput,
  onSetOutput,
  onSetOverwrite,
}: OutputPanelProps) {
  const fraction =
    progress === null || progress.total === 0
      ? 0
      : Math.min(1, Math.max(0, progress.done / progress.total));

  return (
    <section className="panel">
      <h2>Output</h2>
      <div className="row">
        <input
          className="path-field"
          type="text"
          value={outputPath ?? ""}
          placeholder="where to write the patched ISO"
          aria-label="output path"
          onChange={(e) => onSetOutput(e.target.value)}
        />
        <button type="button" onClick={onChooseOutput}>
          Save as…
        </button>
      </div>
      <label>
        <input
          type="checkbox"
          checked={overwrite}
          onChange={(e) => onSetOverwrite(e.target.checked)}
        />
        overwrite existing file
      </label>
      {status !== null && (
        <p className="hint">
          free-space note: the output copy needs {status.size.toLocaleString()} bytes free in the
          destination folder
        </p>
      )}
      {summary !== null && attackPinNote(summary) !== null && (
        <p className="note-warn">{attackPinNote(summary)}</p>
      )}
      {summary !== null && collapseNote(summary) !== null && (
        <p className="note-warn">{collapseNote(summary)}</p>
      )}
      {lastOutcome !== null && (
        <p className="note-ok">
          patched {lastOutcome.bytes.toLocaleString()} bytes, md5 {lastOutcome.md5}
        </p>
      )}
      <div className="row">
        <button type="button" onClick={onPatch} disabled={!canPatch || busy}>
          Patch ISO
        </button>
        {progress !== null && (
          <span className="progress-line">
            <span>{progress.phase}</span>
            <progress value={fraction} />
            <span>
              {progress.done.toLocaleString()}/{progress.total.toLocaleString()}
            </span>
          </span>
        )}
        {busy && progress === null && <span className="hint">working…</span>}
      </div>
      <div className="log-panel" aria-label="log">
        {log.map((line, i) => (
          /* The log is append-only; the index is stable per line. */
          <div key={i} className="log-line">
            {line}
          </div>
        ))}
      </div>
    </section>
  );
}
