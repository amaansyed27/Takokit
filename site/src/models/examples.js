function json(value) {
  return JSON.stringify(value, null, 2);
}

export function pullCommand(model, release) {
  const ref = release.tag === model.default_tag
    ? model.name
    : `${model.name}:${release.tag}`;
  return `tako pull ${ref}`;
}

export function taskCommand(model, release) {
  const ref = release.tag === model.default_tag
    ? model.name
    : `${model.name}:${release.tag}`;
  if (model.tasks.includes("voice-conversion")) {
    return `tako convert source.wav --target-voice ./owned-voice.pth --model ${ref} --consent`;
  }
  if (model.tasks.includes("voice-cloning")) {
    return `tako clone reference.wav --name "My Voice" --model ${ref} --consent`;
  }
  if (model.tasks.includes("stt")) {
    return `tako transcribe recording.wav --model ${ref}`;
  }
  return `tako speak "Hello from Takokit" --model ${ref}`;
}

function apiRequest(model, release) {
  const ref = release.tag === model.default_tag
    ? model.name
    : `${model.name}:${release.tag}`;
  if (model.tasks.includes("voice-conversion")) {
    return {
      route: "/v1/audio/conversions",
      body: {
        model: ref,
        source_path: "source.wav",
        target_voice: "./owned-voice.pth",
        consent_affirmed: true,
      },
    };
  }
  if (model.tasks.includes("voice-cloning")) {
    return {
      route: "/v1/voices/clone",
      body: {
        sample_path: "reference.wav",
        name: "My Voice",
        model: ref,
        consent_affirmed: true,
      },
    };
  }
  if (model.tasks.includes("stt")) {
    return {
      route: "/v1/audio/transcriptions",
      body: { file_path: "recording.wav", model: ref },
    };
  }
  return {
    route: "/v1/audio/speech",
    body: {
      model: ref,
      input: "Hello from Takokit",
      voice: "default",
      response_format: "wav",
    },
  };
}

export function integrationExamples(model, release) {
  const request = apiRequest(model, release);
  const url = `http://127.0.0.1:5050${request.route}`;
  return {
    CLI: taskCommand(model, release),
    "REST API": `curl -X POST ${url} \\\n  -H "Content-Type: application/json" \\\n  -d '${JSON.stringify(request.body)}'`,
    Python: `import requests\n\nresponse = requests.post(\n    "${url}",\n    json=${json(request.body).replace(/\n/g, "\n    ")},\n    timeout=300,\n)\nresponse.raise_for_status()\nprint(response.json())`,
    JavaScript: `const response = await fetch("${url}", {\n  method: "POST",\n  headers: { "Content-Type": "application/json" },\n  body: JSON.stringify(${json(request.body).replace(/\n/g, "\n  ")}),\n});\n\nif (!response.ok) throw new Error(await response.text());\nconsole.log(await response.json());`,
  };
}
