export type HotkeyMode = "toggle" | "hold";
export type EnhancementProvider = "openAi" | "groq" | "customOpenAiCompatible";
export type PromptProfile = "default" | "cleanTranscript" | "email" | "codeComments";
export type TextInsertionMode = "sendInput" | "clipboardFallback";

export interface HotkeySettings {
  enabled: boolean;
  mode: HotkeyMode;
  shortcut: string;
}

export interface TranscriptionSettings {
  provider: "whisperLocal";
  modelId: string;
  streamingPreview: boolean;
  maxRecordingSeconds: number;
}

export interface EnhancementSettings {
  enabled: boolean;
  provider: EnhancementProvider;
  baseUrl: string;
  model: string;
  promptProfile: PromptProfile;
  timeoutSeconds: number;
}

export interface TextInsertionSettings {
  mode: TextInsertionMode;
  typingDelayMs: number;
  restoreClipboard: boolean;
}

export interface HistorySettings {
  enabled: boolean;
  retainAudio: boolean;
  maxItems: number;
}

export interface StartupSettings {
  launchAtLogin: boolean;
}

export interface AppSettings {
  version: number;
  language: string;
  inputDeviceId: string | null;
  hotkey: HotkeySettings;
  transcription: TranscriptionSettings;
  enhancement: EnhancementSettings;
  insertion: TextInsertionSettings;
  history: HistorySettings;
  startup: StartupSettings;
}

export interface AudioInputDevice {
  id: string;
  name: string;
  isDefault: boolean;
  channels: number[];
  minSampleRate: number | null;
  maxSampleRate: number | null;
}

export interface AudioLevel {
  peak: number;
  rms: number;
}

export interface RecordingSnapshot {
  id: string;
  startedAt: string;
  durationMs: number;
  sampleRate: number;
  level: AudioLevel;
}

export interface ModelDescriptor {
  id: string;
  name: string;
  fileName: string;
  downloadUrl: string;
  sizeMb: number;
  recommendedMinRamGb: number;
  languages: string;
}

export interface ModelCacheStatus {
  model: ModelDescriptor;
  isDownloaded: boolean;
  path: string;
  bytesOnDisk: number;
}

export interface SecretStatus {
  key: string;
  exists: boolean;
}

export interface AppStatus {
  settings: AppSettings;
  devices: AudioInputDevice[];
  devicesError: string | null;
  models: ModelCacheStatus[];
  recording: RecordingSnapshot | null;
  openaiSecret: SecretStatus;
  groqSecret: SecretStatus;
  customSecret: SecretStatus;
}

export interface TranscriptResult {
  rawText: string;
  modelId: string;
  language: string;
  audioDurationMs: number;
}

export interface EnhancementResult {
  text: string;
  provider: string;
  model: string;
}

export interface InsertionResult {
  attemptedMode: TextInsertionMode;
  success: boolean;
  insertedCharacters: number;
  fallbackUsed: boolean;
  message: string;
}

export interface DictationResult {
  transcript: TranscriptResult;
  finalText: string;
  enhancement: EnhancementResult | null;
  enhancementError: string | null;
  insertion: InsertionResult | null;
  historyId: string | null;
}

export interface HistoryEntry {
  id: string;
  createdAt: string;
  rawTranscript: string;
  finalText: string;
  modelId: string;
  enhancementProvider: string | null;
  inserted: boolean;
  targetApp: string | null;
}

export interface ModelDownloadProgress {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number | null;
}

