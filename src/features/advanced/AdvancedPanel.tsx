import type { PlanUiState, PlanField } from "../presets/usePlan";
import {
  CROWN_COLOURS,
  CROWN_SWATCH,
  DEFAULT_SERIAL,
  STAT_NAMES,
} from "../presets/presetData";

export interface AdvancedPanelProps {
  state: PlanUiState;
  onEditField: <K extends PlanField>(field: K, value: PlanUiState["plan"][K]) => void;
  onSetSerialEnabled: (on: boolean) => void;
  onSetSerialText: (text: string) => void;
}

/** One labelled multiplier input. */
function MultiplierRow({
  label,
  value,
  onEdit,
}: {
  label: string;
  value: number;
  onEdit: (value: number) => void;
}) {
  return (
    <label className="mult-row">
      <span>{label}</span>
      <input
        type="number"
        step="0.01"
        value={value}
        onChange={(e) => {
          const parsed = Number.parseFloat(e.target.value);
          if (!Number.isNaN(parsed)) onEdit(parsed);
        }}
      />
    </label>
  );
}

/** Every dial the engine's PatchPlan exposes, laid out responsively. */
export function AdvancedPanel({
  state,
  onEditField,
  onSetSerialEnabled,
  onSetSerialText,
}: AdvancedPanelProps) {
  const plan = state.plan;

  return (
    <section className="panel">
      <h2>Advanced</h2>
      <div className="mult-grid">
        <MultiplierRow
          label="hp"
          value={plan.hpMult}
          onEdit={(value) => onEditField("hpMult", value)}
        />
        {STAT_NAMES.map((name, c) => (
          <MultiplierRow
            key={name}
            label={name}
            value={plan.statMult[c] ?? 1}
            onEdit={(value) => {
              const statMult = [...plan.statMult];
              statMult[c] = value;
              onEditField("statMult", statMult);
            }}
          />
        ))}
        <MultiplierRow
          label="crit"
          value={plan.critMult}
          onEdit={(value) => onEditField("critMult", value)}
        />
        <MultiplierRow
          label="paralysis"
          value={plan.paraMult}
          onEdit={(value) => onEditField("paraMult", value)}
        />
      </div>

      <label className="row crown-row">
        <span>crown colour</span>
        <select
          aria-label="crown colour"
          value={plan.crownRank}
          onChange={(e) => onEditField("crownRank", Number(e.target.value))}
        >
          {CROWN_COLOURS.map(({ rank, name }) => (
            <option key={rank} value={rank}>
              {name}
            </option>
          ))}
        </select>
        <span
          className="crown-swatch"
          style={{ background: CROWN_SWATCH[plan.crownRank] ?? "transparent" }}
          aria-hidden
        />
      </label>

      <div className="check-col">
        <label>
          <input
            type="checkbox"
            checked={plan.excludeDestructibles}
            onChange={(e) => onEditField("excludeDestructibles", e.target.checked)}
          />
          exclude destructibles
        </label>
        <label>
          <input
            type="checkbox"
            checked={plan.excludeTripwire}
            onChange={(e) => onEditField("excludeTripwire", e.target.checked)}
          />
          exclude tripwire
        </label>
        <label>
          <input
            type="checkbox"
            checked={plan.excludePractice}
            onChange={(e) => onEditField("excludePractice", e.target.checked)}
          />
          leave training-stage enemies vanilla
        </label>
        <label>
          <input
            type="checkbox"
            checked={plan.practiceBuff}
            onChange={(e) => onEditField("practiceBuff", e.target.checked)}
          />
          practice buff
        </label>
        <label>
          <input
            type="checkbox"
            checked={plan.forceVeryHard}
            onChange={(e) => onEditField("forceVeryHard", e.target.checked)}
          />
          force Very Hard
        </label>
        <label>
          <input
            type="checkbox"
            checked={state.serialEnabled}
            onChange={(e) => onSetSerialEnabled(e.target.checked)}
          />
          rename disc serial
        </label>
        {state.serialEnabled && (
          <input
            type="text"
            aria-label="disc serial"
            value={state.serialText}
            placeholder={DEFAULT_SERIAL}
            onChange={(e) => onSetSerialText(e.target.value)}
          />
        )}
      </div>

      <p className="hint">
        these settings follow the selected preset until you edit one — then the preset becomes
        Custom
      </p>
    </section>
  );
}
