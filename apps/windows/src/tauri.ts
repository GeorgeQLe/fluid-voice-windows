import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AppStatus,
  AudioInputDevice,
  DictationResult,
  HistoryEntry,
  InsertionResult,
  ModelCacheStatus,
  RecordingSnapshot,
  SecretStatus
} from "./types";

export const api = {
  getAppStatus: () => invoke<AppStatus>("get_app_status"),
  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) => invoke<AppSettings>("save_settings", { settings }),
  listInputDevices: () => invoke<AudioInputDevice[]>("list_input_devices"),
  startDictation: () => invoke<RecordingSnapshot>("start_dictation"),
  getRecordingSnapshot: () => invoke<RecordingSnapshot | null>("get_recording_snapshot"),
  cancelDictation: () => invoke<void>("cancel_dictation"),
  finishDictation: (insert: boolean) => invoke<DictationResult>("finish_dictation", { insert }),
  listModels: () => invoke<ModelCacheStatus[]>("list_models"),
  validateModelCache: (modelId: string) =>
    invoke<ModelCacheStatus>("validate_model_cache", { modelId, model_id: modelId }),
  downloadModel: (modelId: string) =>
    invoke<ModelCacheStatus>("download_model", { modelId, model_id: modelId }),
  clearModelCache: (modelId?: string) =>
    invoke<ModelCacheStatus[]>("clear_model_cache", {
      modelId: modelId ?? null,
      model_id: modelId ?? null
    }),
  insertText: (text: string) => invoke<InsertionResult>("insert_text", { text }),
  listHistory: (limit?: number) => invoke<HistoryEntry[]>("list_history", { limit: limit ?? null }),
  clearHistory: () => invoke<void>("clear_history"),
  setApiKey: (key: string, secret: string) =>
    invoke<SecretStatus>("set_api_key", { key, secret }),
  hasApiKey: (key: string) => invoke<SecretStatus>("has_api_key", { key }),
  deleteApiKey: (key: string) => invoke<SecretStatus>("delete_api_key", { key })
};

