import assert from "node:assert/strict";
import test from "node:test";
import { integrationExamples, pullCommand, taskCommand } from "../src/models/examples.js";

function model(tasks, name = "fixture", defaultTag = "latest") {
  return { name, default_tag: defaultTag, tasks };
}

const release = { tag: "latest" };

test("pull command uses canonical default reference", () => {
  assert.equal(pullCommand(model(["tts"]), release), "tako pull fixture");
});

test("task commands match current CLI parser shapes", () => {
  assert.equal(taskCommand(model(["tts"]), release), 'tako speak "Hello from Takokit" --model fixture');
  assert.equal(taskCommand(model(["stt"]), release), "tako transcribe recording.wav --model fixture");
  assert.match(taskCommand(model(["voice-cloning"]), release), /^tako clone reference\.wav --name/);
  assert.match(taskCommand(model(["voice-conversion"]), release), /^tako convert source\.wav --target-voice/);
});

test("HTTP examples use real local API routes and do not imply SDKs", () => {
  const speech = integrationExamples(model(["tts"], "kokoro"), release);
  assert.match(speech["REST API"], /\/v1\/audio\/speech/);
  assert.match(speech.Python, /requests\.post/);
  assert.match(speech.JavaScript, /fetch\(/);

  const transcription = integrationExamples(model(["stt"], "whisper"), release);
  assert.match(transcription["REST API"], /\/v1\/audio\/transcriptions/);
});
