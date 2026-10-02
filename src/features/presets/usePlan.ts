import { useReducer } from "react";
import type { PatchPlan, PresetId } from "../../bindings";
import { DEFAULT_SERIAL, detectPreset, presetPlan } from "./presetData";

/** The editable fields of a plan; serial goes through setSerial* instead. */
export type PlanField = Exclude<keyof PatchPlan, "serial">;

/** Everything the preset selector and Advanced panel agree on. */
export interface PlanUiState {
  preset: PresetId;
  plan: PatchPlan;
  serialEnabled: boolean;
  serialText: string;
}

type Action =
  | { type: "presetApplied"; preset: PresetId }
  | { type: "fieldEdited"; field: PlanField; value: PatchPlan[PlanField] }
  | { type: "serialEnabled"; on: boolean }
  | { type: "serialText"; text: string };

function reducer(state: PlanUiState, action: Action): PlanUiState {
  switch (action.type) {
    case "presetApplied": {
      // Custom has no canonical plan: it keeps whatever is on screen.
      if (action.preset === "Custom") {
        return { ...state, preset: "Custom" };
      }
      // A named preset loads its whole plan and resets the serial controls,
      // so Advanced always shows the values the selected build will write.
      const plan = presetPlan(action.preset);
      return {
        preset: action.preset,
        plan,
        serialEnabled: false,
        serialText: DEFAULT_SERIAL,
      };
    }
    case "fieldEdited": {
      const plan = { ...state.plan, [action.field]: action.value };
      // Recomputed on the same dispatch as the edit: preset label and plan
      // cannot disagree, which was the egui bug.
      return { ...state, plan, preset: detectPreset(plan) };
    }
    case "serialEnabled": {
      const plan = {
        ...state.plan,
        serial: action.on && state.serialText.length > 0 ? state.serialText : null,
      };
      return { ...state, serialEnabled: action.on, plan, preset: detectPreset(plan) };
    }
    case "serialText": {
      const plan = {
        ...state.plan,
        serial: state.serialEnabled && action.text.length > 0 ? action.text : null,
      };
      return { ...state, serialText: action.text, plan, preset: detectPreset(plan) };
    }
  }
}

/**
 * The single store for the patch plan. Preset selection and Advanced edits
 * are actions on one reducer, so the two panels can never desync.
 */
export function usePlan() {
  const [state, dispatch] = useReducer(reducer, {
    preset: "VeryHardPlus",
    plan: presetPlan("VeryHardPlus"),
    serialEnabled: false,
    serialText: DEFAULT_SERIAL,
  });

  return {
    state,
    applyPreset: (preset: PresetId) => dispatch({ type: "presetApplied", preset }),
    editField: <K extends PlanField>(field: K, value: PatchPlan[K]) =>
      dispatch({ type: "fieldEdited", field, value }),
    setSerialEnabled: (on: boolean) => dispatch({ type: "serialEnabled", on }),
    setSerialText: (text: string) => dispatch({ type: "serialText", text }),
  };
}
