import { useState } from "react";
import { AudioLines, ChevronDown, Download, MoreVertical, Play, Volume2 } from "lucide-react";
import { SectionHeader } from "../../components/SectionHeader";
import type { ModelSummary, VoiceSummary } from "../../lib/types";

type SpeakPageProps = {
  models: ModelSummary[];
  voices: VoiceSummary[];
  onViewModels: () => void;
};

export function SpeakPage({ models, voices, onViewModels }: SpeakPageProps) {
  const ttsModels = models.filter((model) => model.capabilities.includes("tts"));
  const [text, setText] = useState("");
  const [model, setModel] = useState(ttsModels[0]?.id ?? "mock-tts");
  const [voice, setVoice] = useState(voices[0]?.id ?? "af_sky");
  const [lastRun, setLastRun] = useState<string | null>(null);
  const selectedModel = ttsModels.find((item) => item.id === model) ?? ttsModels[0];
  const selectedVoice = voices.find((item) => item.id === voice) ?? voices[0];

  return (
    <section className="speak-screen">
      <SectionHeader title="Speak" description="Generate natural speech from text using local models." />

      <form
        className="speak-form"
        onSubmit={(event) => {
          event.preventDefault();
          setLastRun(`Generated speech with ${model} and ${voice}.`);
        }}
      >
        <div className="selector-grid">
          <label className="field-block">
            <span>Model</span>
            <select value={model} onChange={(event) => setModel(event.target.value)}>
              {ttsModels.map((item) => (
                <option key={item.id} value={item.id}>{item.name}</option>
              ))}
            </select>
            <small>{selectedModel.language} • {selectedModel.params} params • {selectedModel.backend}</small>
          </label>

          <label className="field-block">
            <span>Voice</span>
            <select value={voice} onChange={(event) => setVoice(event.target.value)}>
              {voices.map((item) => (
                <option key={item.id} value={item.id}>{item.name}</option>
              ))}
            </select>
            <small>{selectedVoice.label}</small>
          </label>
        </div>

        <div className="input-grid">
          <label className="field-block text-field">
            <span>Text Input</span>
            <textarea
              value={text}
              onChange={(event) => setText(event.target.value)}
              maxLength={5000}
              placeholder="Enter text to speak..."
            />
            <small className="character-count">{text.length} / 5000</small>
          </label>

          <aside className="generation-panel">
            <button className="generate-button" type="submit"><AudioLines size={18} /> Generate Speech</button>
            <button className="preview-button" type="button"><Play size={17} /> Preview (5s)</button>
            <button className="advanced-button" type="button">
              <span>Advanced Options</span>
              <ChevronDown size={16} />
            </button>
          </aside>
        </div>
      </form>

      <section className="output-section" aria-label="Output">
        <h3>Output</h3>
        <div className="output-panel">
          <div className="audio-player">
            <Play size={18} fill="currentColor" />
            <span>00:00 / 00:00</span>
            <div className="audio-track"><span /></div>
            <Volume2 size={19} />
            <MoreVertical size={18} />
          </div>
          <div className="output-footer">
            <span>{lastRun ?? "Audio will appear here after generation."}</span>
            <button className="download-button" type="button"><Download size={16} /> Download Audio</button>
          </div>
        </div>
      </section>

      <section className="installed-models">
        <h3>Installed Models</h3>
        <div className="model-table" role="table" aria-label="Installed models">
          <div className="model-row model-head" role="row">
            <span>Model Name</span>
            <span>Params</span>
            <span>Size</span>
            <span>Backend</span>
            <span />
          </div>
          {ttsModels.slice(0, 3).map((item, index) => (
            <div className="model-row" role="row" key={item.id}>
              <span>
                {item.name}
                {index === 0 && <strong className="active-badge">Active</strong>}
              </span>
              <span>{item.params}</span>
              <span>{item.size}</span>
              <span>{item.backend}</span>
              <MoreVertical size={17} />
            </div>
          ))}
          <div className="model-table-link">
            <button type="button" onClick={onViewModels}>
              View all models <span aria-hidden="true">→</span>
            </button>
          </div>
        </div>
      </section>
    </section>
  );
}
