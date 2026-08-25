from pathlib import Path


def edit(path, old, new, count=1):
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"missing expected block in {path}: {old[:80]!r}")
    p.write_text(text.replace(old, new, count), encoding="utf-8")

# Carry Files → Voices intent into the correct creation mode.
edit(
    "apps/gui/src/lib/workflowIntent.ts",
    "export type VoiceIntent = {\n  samplePath?: string;\n};",
    "export type VoiceIntent = {\n  samplePath?: string;\n  mode?: \"instant\" | \"advanced\";\n};",
)

# Voice page: explicit Instant / Advanced entry and managed RVC voices in the normal library.
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    'import { useVoiceProfileCreation } from "../../hooks/useVoiceProfileCreation";\n',
    'import { useVoiceProfileCreation } from "../../hooks/useVoiceProfileCreation";\nimport { AdvancedVoiceStudio } from "./AdvancedVoiceStudio";\n',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    'import { consumeVoiceIntent, setSpeakIntent } from "../../lib/workflowIntent";',
    'import { consumeVoiceIntent, setCloneIntent, setSpeakIntent } from "../../lib/workflowIntent";',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '  const voiceIntent = useMemo(() => consumeVoiceIntent(), []);\n',
    '  const voiceIntent = useMemo(() => consumeVoiceIntent(), []);\n  const [createMode, setCreateMode] = useState<"instant" | "advanced">(voiceIntent?.mode ?? "instant");\n',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '  const localVoices = runtime.voices.filter((voice) => voice.source === "local-profile");\n  const builtInVoices = runtime.voices.filter((voice) => voice.source !== "local-profile");',
    '  const localVoices = runtime.voices.filter((voice) => voice.source === "local-profile" || voice.source === "managed-rvc");\n  const builtInVoices = runtime.voices.filter((voice) => voice.source !== "local-profile" && voice.source !== "managed-rvc");',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '  function useInSpeak(voiceId: string, modelId: string) {\n    setSpeakIntent({ voiceId, modelId });\n    onNavigate("speak");\n  }\n',
    '  function useInSpeak(voiceId: string, modelId: string) {\n    setSpeakIntent({ voiceId, modelId });\n    onNavigate("speak");\n  }\n\n  function useInConvert(voiceId: string) {\n    setCloneIntent({ targetPath: voiceId, mode: "rvc" });\n    onNavigate("convert");\n  }\n',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '      <ProductPageHeader\n        eyebrow="Voice cloning"\n        title="Voices"\n        description="Create a reusable local voice from a clean reference recording. Browse an existing clip, reuse Files, or record one right here."\n      />\n\n      <div className="tk-voice-studio">',
    '      <ProductPageHeader\n        eyebrow="Voice cloning"\n        title="Voices"\n        description="Create an instant reference clone or build and train a persistent managed RVC voice in Voice Studio."\n      />\n\n      <div className="tk-voice-create-modes" role="tablist" aria-label="Voice creation type">\n        <button type="button" className={createMode === "instant" ? "is-active" : ""} onClick={() => setCreateMode("instant")}>\n          <strong>Instant Clone</strong><span>One clean reference recording → reusable voice profile</span>\n        </button>\n        <button type="button" className={createMode === "advanced" ? "is-active" : ""} onClick={() => setCreateMode("advanced")}>\n          <strong>Advanced Clone</strong><span>Multi-sample RVC dataset → prepare, train, checkpoint, test</span>\n        </button>\n      </div>\n\n      {createMode === "instant" ? (<>\n      <div className="tk-voice-studio">',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '      <section className="tk-voice-library">',
    '      </>) : (\n        <AdvancedVoiceStudio initialSamplePath={voiceIntent?.samplePath} onNavigate={onNavigate} onRefresh={onRefresh} />\n      )}\n\n      <section className="tk-voice-library">',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '                  <span>Saved voice · {voice.model === "none" ? "model-defined" : voice.model}</span>',
    '                  <span>{voice.source === "managed-rvc" ? "Managed RVC voice · ready for Convert" : `Saved voice · ${voice.model === "none" ? "model-defined" : voice.model}`}</span>',
)
edit(
    "apps/gui/src/features/voices/VoicesPage.tsx",
    '                  {voice.model !== "none" ? (\n                    <button className="tk-voice-use" type="button" onClick={() => useInSpeak(voice.id, voice.model)}>\n                      Use in Speak <ArrowRight size={13} strokeWidth={1.9} />\n                    </button>\n                  ) : null}',
    '                  {voice.source === "managed-rvc" ? (\n                    <button className="tk-voice-use" type="button" onClick={() => useInConvert(voice.id)}>\n                      Use in Convert <ArrowRight size={13} strokeWidth={1.9} />\n                    </button>\n                  ) : voice.model !== "none" ? (\n                    <button className="tk-voice-use" type="button" onClick={() => useInSpeak(voice.id, voice.model)}>\n                      Use in Speak <ArrowRight size={13} strokeWidth={1.9} />\n                    </button>\n                  ) : null}',
)

# Files: deliberate Add to Voice Dataset path, in addition to instant clone.
edit(
    "apps/gui/src/features/files/FilesPage.tsx",
    '  function useAudio(file: WorkspaceFile, destination: "transcribe" | "voices" | "clone") {',
    '  function useAudio(file: WorkspaceFile, destination: "transcribe" | "voices" | "advanced" | "clone") {',
)
edit(
    "apps/gui/src/features/files/FilesPage.tsx",
    '    if (destination === "voices") {\n      setVoiceIntent({ samplePath: file.path });\n      onNavigate("voices");\n      return;\n    }\n    setCloneIntent({ sourcePath: file.path, mode: "reference" });',
    '    if (destination === "voices") {\n      setVoiceIntent({ samplePath: file.path, mode: "instant" });\n      onNavigate("voices");\n      return;\n    }\n    if (destination === "advanced") {\n      setVoiceIntent({ samplePath: file.path, mode: "advanced" });\n      onNavigate("voices");\n      return;\n    }\n    setCloneIntent({ sourcePath: file.path, mode: "reference" });',
)
edit(
    "apps/gui/src/features/files/FilesPage.tsx",
    '                  <button type="button" onClick={() => useAudio(file, "voices")}>Create voice</button>\n                  <button type="button" onClick={() => useAudio(file, "clone")}>Clone audio <ArrowRight size={13} /></button>',
    '                  <button type="button" onClick={() => useAudio(file, "voices")}>Instant voice</button>\n                  <button type="button" onClick={() => useAudio(file, "advanced")}>Add to Voice Dataset</button>\n                  <button type="button" onClick={() => useAudio(file, "clone")}>Clone audio <ArrowRight size={13} /></button>',
)

# Convert: surface managed RVC voices directly while retaining legacy checkpoint-folder browsing.
edit(
    "apps/gui/src/features/convert/ConvertPage.tsx",
    '  const cloneIntent = useMemo(() => consumeCloneIntent(), []);\n',
    '  const cloneIntent = useMemo(() => consumeCloneIntent(), []);\n  const managedRvcVoices = useMemo(\n    () => runtime.voices.filter((voice) => voice.source === "managed-rvc" && voice.model === "rvc"),\n    [runtime.voices]\n  );\n',
)
edit(
    "apps/gui/src/features/convert/ConvertPage.tsx",
    '  const selectedModel = modeModels.find((item) => item.id === model) ?? modeModels[0];\n',
    '  const selectedModel = modeModels.find((item) => item.id === model) ?? modeModels[0];\n  const selectedManagedRvcVoice = mode === "rvc" ? managedRvcVoices.find((voice) => voice.id === targetPath) : undefined;\n',
)
edit(
    "apps/gui/src/features/convert/ConvertPage.tsx",
    '            <ConversionPathCard\n              label={mode === "rvc" ? "Target RVC package" : "Target reference"}',
    '            {mode === "rvc" && managedRvcVoices.length > 0 ? (\n              <ProductSelect\n                label="Managed RVC voice"\n                value={selectedManagedRvcVoice?.id ?? ""}\n                onChange={(event) => setTargetPath(event.target.value)}\n                options={[{ value: "", label: "Choose a managed voice" }, ...managedRvcVoices.map((voice) => ({ value: voice.id, label: voice.name }))]}\n                hint="Voice Studio checkpoints selected as Ready appear here automatically."\n              />\n            ) : null}\n\n            <ConversionPathCard\n              label={mode === "rvc" ? (selectedManagedRvcVoice ? "Target managed voice" : "Target RVC package") : "Target reference"}',
)
edit(
    "apps/gui/src/features/convert/ConvertPage.tsx",
    '                mode === "rvc"\n                  ? "Choose the folder containing the target checkpoint and optional index."',
    '                mode === "rvc"\n                  ? selectedManagedRvcVoice\n                    ? "This managed Voice Studio checkpoint will be resolved by the shared RVC service."\n                    : "Choose a managed voice above or browse a legacy folder containing a checkpoint and optional index."',
)
edit(
    "apps/gui/src/features/convert/ConvertPage.tsx",
    '              actionLabel={targetPath ? "Choose another" : mode === "rvc" ? "Browse package" : "Browse reference"}',
    '              actionLabel={mode === "rvc" ? "Browse legacy package" : targetPath ? "Choose another" : "Browse reference"}',
)

# Import Voice Studio styling without enlarging existing shared files.
edit(
    "apps/gui/src/styles/v2/index.css",
    '@import "./voices-library.css";\n',
    '@import "./voices-library.css";\n@import "./rvc-studio.css";\n',
)
