import type { PresetId } from "../../bindings";
import { PRESET_LABELS, PRESET_NOTES } from "./presetData";

const NAMED: PresetId[] = ["VeryHardPlus", "Extreme", "Brutal"];

/**
 * One selectable card per preset. Custom is a card only when it is the
 * active preset — a plan the player edited. Clicking a card applies it.
 */
export function PresetCards({
  preset,
  onPick,
}: {
  preset: PresetId;
  onPick: (preset: PresetId) => void;
}) {
  const cards: PresetId[] = preset === "Custom" ? [...NAMED, "Custom"] : NAMED;
  return (
    <section className="panel">
      <h2>Preset</h2>
      <div className="preset-row">
        {cards.map((id) => (
          <button
            key={id}
            type="button"
            className={`preset-card${preset === id ? " preset-active" : ""}`}
            onClick={() => onPick(id)}
          >
            <span className="preset-name">{PRESET_LABELS[id]}</span>
            <span className="preset-note">{PRESET_NOTES[id]}</span>
          </button>
        ))}
      </div>
      {preset === "Custom" && (
        <p className="hint">
          these settings follow the selected preset until you edit one — then the preset becomes
          Custom
        </p>
      )}
    </section>
  );
}
