import { FolderOpen } from "lucide-react";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductSelect } from "../../components/ui/ProductSelect";
import type { RvcF0Method } from "../../lib/types";
import { NumberField, f0Options } from "./ConvertComponents";

type ModelOption = { value: string; label: string };

type Props = {
  mode: "reference" | "rvc";
  model: string;
  modelOptions: ModelOption[];
  modelHint?: string;
  onModelChange: (value: string) => void;
  sourcePath: string;
  targetPath: string;
  onSourcePathChange: (value: string) => void;
  onTargetPathChange: (value: string) => void;
  onBrowseLegacyTarget: () => void;
  legacyBusy: boolean;
  f0Method: RvcF0Method;
  pitchShift: number;
  indexRate: number;
  rmsMixRate: number;
  protect: number;
  filterRadius: number;
  onF0MethodChange: (value: RvcF0Method) => void;
  onPitchShiftChange: (value: number) => void;
  onIndexRateChange: (value: number) => void;
  onRmsMixRateChange: (value: number) => void;
  onProtectChange: (value: number) => void;
  onFilterRadiusChange: (value: number) => void;
};

export function ConvertAdvancedSettings(props: Props) {
  return (
    <details className="tk-clone-simple-advanced">
      <summary>Advanced settings</summary>
      <div className="tk-clone-simple-advanced__body">
        {props.modelOptions.length > 1 ? (
          <ProductSelect
            label="Conversion model"
            value={props.model}
            onChange={(event) => props.onModelChange(event.target.value)}
            options={props.modelOptions}
            hint={props.modelHint}
          />
        ) : null}

        {props.mode === "rvc" ? (
          <>
            <div className="tk-clone-simple-advanced__section">
              <strong>RVC tuning</strong>
              <p>Defaults are recommended. Change these only when you know the target voice needs different conversion behavior.</p>
              <ProductSelect
                label="Pitch extraction"
                value={props.f0Method}
                onChange={(event) => props.onF0MethodChange(event.target.value as RvcF0Method)}
                options={f0Options}
              />
              <div className="tk-convert-number-grid">
                <NumberField label="Pitch" value={props.pitchShift} min={-24} max={24} step={1} suffix="semitones" onChange={props.onPitchShiftChange} />
                <NumberField label="Index rate" value={props.indexRate} min={0} max={1} step={0.05} onChange={props.onIndexRateChange} />
                <NumberField label="RMS mix" value={props.rmsMixRate} min={0} max={1} step={0.05} onChange={props.onRmsMixRateChange} />
                <NumberField label="Protect" value={props.protect} min={0} max={0.5} step={0.01} onChange={props.onProtectChange} />
                <NumberField label="Filter radius" value={props.filterRadius} min={0} max={7} step={1} onChange={props.onFilterRadiusChange} />
              </div>
            </div>

            <div className="tk-clone-simple-advanced__section">
              <strong>Legacy RVC target</strong>
              <p>Normal Takokit voices are selected by name above. Use this only for an unmanaged RVC folder from outside Takokit.</p>
              <div className="tk-clone-simple-file-row">
                <input className="tk-input" value={props.targetPath} onChange={(event) => props.onTargetPathChange(event.target.value)} placeholder="C:\\path\\to\\legacy-rvc-folder" spellCheck={false} />
                <ProductButton tone="secondary" loading={props.legacyBusy} onClick={props.onBrowseLegacyTarget}><FolderOpen size={14} /> Browse folder</ProductButton>
              </div>
            </div>
          </>
        ) : null}

        <div className="tk-clone-simple-advanced__section">
          <strong>Manual paths</strong>
          <label className="tk-field">
            <span className="tk-field__label">Source audio</span>
            <input className="tk-input" value={props.sourcePath} onChange={(event) => props.onSourcePathChange(event.target.value)} spellCheck={false} />
          </label>
          {props.mode === "reference" ? (
            <label className="tk-field">
              <span className="tk-field__label">Target reference audio</span>
              <input className="tk-input" value={props.targetPath} onChange={(event) => props.onTargetPathChange(event.target.value)} spellCheck={false} />
            </label>
          ) : null}
        </div>
      </div>
    </details>
  );
}
