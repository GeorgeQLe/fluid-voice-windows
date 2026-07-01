import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AlertTriangle,
  AudioLines,
  CheckCircle2,
  Cpu,
  Download,
  History,
  KeyRound,
  Keyboard,
  Mic,
  RefreshCw,
  Save,
  Settings,
  Square,
  Trash2,
  Wand2,
  X
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { api } from "./tauri";
import type {
  AppSettings,
  AppStatus,
  DictationResult,
  EnhancementProvider,
  HistoryEntry,
  ModelCacheStatus,
  ModelDownloadProgress,
  PromptProfile,
  RecordingSnapshot
} from "./types";

type Tab = "dictation" | "models" | "enhancement" | "history";

const tabs: Array<{ id: Tab; label: string; icon: typeof Mic }> = [
  { id: "dictation", label: "Dictation", icon: Mic },
  { id: "models", label: "Models", icon: Cpu },
  { id: "enhancement", label: "Enhancement", icon: Wand2 },
  { id: "history", label: "History", icon: History }
];

export function App() {
  const isOverlay = safeWindowLabel() === "overlay";
  return isOverlay ? <OverlayView /> : <MainView />;
}

function MainView() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [activeTab, setActiveTab] = useState<Tab>("dictation");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<DictationResult | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, ModelDownloadProgress>>(
    {}
  );
  const [apiKeyDraft, setApiKeyDraft] = useState("");

  useEffect(() => {
    void refresh();
    const unlisten = listen<ModelDownloadProgress>("model-download-progress", (event) => {
      setDownloadProgress((current) => ({
        ...current,
        [event.payload.modelId]: event.payload
      }));
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  async function refresh() {
    setBusy("Refreshing");
    setError(null);
    try {
      const nextStatus = await api.getAppStatus();
      const entries = await api.listHistory(nextStatus.settings.history.maxItems);
      setStatus(nextStatus);
      setHistory(entries);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function persistSettings(next: AppSettings) {
    setStatus((current) => (current ? { ...current, settings: next } : current));
    try {
      const saved = await api.saveSettings(next);
      setStatus((current) => (current ? { ...current, settings: saved } : current));
    } catch (caught) {
      setError(errorMessage(caught));
      await refresh();
    }
  }

  async function startDictation() {
    setBusy("Recording");
    setError(null);
    setLastResult(null);
    try {
      const recording = await api.startDictation();
      setStatus((current) => (current ? { ...current, recording } : current));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function finishDictation(insert: boolean) {
    setBusy("Transcribing");
    setError(null);
    try {
      const result = await api.finishDictation(insert);
      setLastResult(result);
      await refresh();
    } catch (caught) {
      setError(errorMessage(caught));
      setStatus((current) => (current ? { ...current, recording: null } : current));
    } finally {
      setBusy(null);
    }
  }

  async function cancelDictation() {
    setBusy("Canceling");
    setError(null);
    try {
      await api.cancelDictation();
      setStatus((current) => (current ? { ...current, recording: null } : current));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function downloadModel(model: ModelCacheStatus) {
    setBusy(`Downloading ${model.model.name}`);
    setError(null);
    try {
      await api.downloadModel(model.model.id);
      const models = await api.listModels();
      setStatus((current) => (current ? { ...current, models } : current));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function clearModels(modelId?: string) {
    setBusy("Clearing models");
    setError(null);
    try {
      const models = await api.clearModelCache(modelId);
      setStatus((current) => (current ? { ...current, models } : current));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function saveApiKey() {
    if (!status || !apiKeyDraft.trim()) return;
    setBusy("Saving key");
    setError(null);
    try {
      await api.setApiKey(secretKeyForProvider(status.settings.enhancement.provider), apiKeyDraft);
      setApiKeyDraft("");
      await refresh();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function deleteApiKey() {
    if (!status) return;
    setBusy("Deleting key");
    setError(null);
    try {
      await api.deleteApiKey(secretKeyForProvider(status.settings.enhancement.provider));
      await refresh();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  async function clearHistory() {
    setBusy("Clearing history");
    setError(null);
    try {
      await api.clearHistory();
      setHistory([]);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(null);
    }
  }

  const settings = status?.settings;
  const selectedModel = useMemo(
    () => status?.models.find((item) => item.model.id === settings?.transcription.modelId),
    [settings?.transcription.modelId, status?.models]
  );

  if (!status || !settings) {
    return (
      <main className="boot">
        <RefreshCw className="spin" size={22} />
        <span>Loading FluidVoice</span>
        {error ? <p className="errorText">{error}</p> : null}
      </main>
    );
  }

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand">
          <AudioLines size={26} />
          <div>
            <strong>FluidVoice</strong>
            <span>Windows MVP</span>
          </div>
        </div>

        <nav className="tabs" aria-label="Settings sections">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                className={activeTab === tab.id ? "tab active" : "tab"}
                onClick={() => setActiveTab(tab.id)}
                type="button"
              >
                <Icon size={18} />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="statusStrip">
          <StatusDot ok={selectedModel?.isDownloaded ?? false} />
          <span>{selectedModel?.isDownloaded ? "Model ready" : "Model missing"}</span>
        </div>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h1>{tabs.find((tab) => tab.id === activeTab)?.label}</h1>
            <p>{busy ?? "Ready"}</p>
          </div>
          <button className="iconButton" type="button" onClick={refresh} title="Refresh">
            <RefreshCw size={18} className={busy === "Refreshing" ? "spin" : ""} />
          </button>
        </header>

        {error ? (
          <div className="banner error">
            <AlertTriangle size={18} />
            <span>{error}</span>
          </div>
        ) : null}

        {status.devicesError ? (
          <div className="banner warning">
            <AlertTriangle size={18} />
            <span>{status.devicesError}</span>
          </div>
        ) : null}

        {activeTab === "dictation" ? (
          <DictationPanel
            status={status}
            lastResult={lastResult}
            onStart={startDictation}
            onFinish={finishDictation}
            onCancel={cancelDictation}
            onSettings={persistSettings}
          />
        ) : null}

        {activeTab === "models" ? (
          <ModelsPanel
            settings={settings}
            models={status.models}
            progress={downloadProgress}
            onDownload={downloadModel}
            onClear={clearModels}
            onSettings={persistSettings}
          />
        ) : null}

        {activeTab === "enhancement" ? (
          <EnhancementPanel
            status={status}
            apiKeyDraft={apiKeyDraft}
            onDraftChange={setApiKeyDraft}
            onSaveKey={saveApiKey}
            onDeleteKey={deleteApiKey}
            onSettings={persistSettings}
          />
        ) : null}

        {activeTab === "history" ? (
          <HistoryPanel entries={history} onClear={clearHistory} />
        ) : null}
      </section>
    </main>
  );
}

function DictationPanel({
  status,
  lastResult,
  onStart,
  onFinish,
  onCancel,
  onSettings
}: {
  status: AppStatus;
  lastResult: DictationResult | null;
  onStart: () => Promise<void>;
  onFinish: (insert: boolean) => Promise<void>;
  onCancel: () => Promise<void>;
  onSettings: (settings: AppSettings) => Promise<void>;
}) {
  const { settings } = status;
  const isRecording = Boolean(status.recording);

  return (
    <div className="sectionGrid">
      <section className="panel commandPanel">
        <div className="recordingMeter">
          <div
            className={isRecording ? "pulseRing live" : "pulseRing"}
            style={{ "--level": status.recording?.level.rms ?? 0 } as React.CSSProperties}
          >
            <Mic size={42} />
          </div>
          <div>
            <h2>{isRecording ? "Recording" : "Idle"}</h2>
            <p>{isRecording ? formatDuration(status.recording?.durationMs ?? 0) : "Toggle mode"}</p>
          </div>
        </div>

        <div className="buttonRow">
          {!isRecording ? (
            <button className="primaryButton" type="button" onClick={onStart}>
              <Mic size={18} />
              Start
            </button>
          ) : (
            <>
              <button className="primaryButton" type="button" onClick={() => onFinish(true)}>
                <Square size={18} />
                Finish
              </button>
              <button className="secondaryButton" type="button" onClick={onCancel}>
                <X size={18} />
                Cancel
              </button>
            </>
          )}
        </div>
      </section>

      <section className="panel">
        <PanelHeading icon={Settings} title="Input" />
        <label className="field">
          <span>Microphone</span>
          <select
            value={settings.inputDeviceId ?? ""}
            onChange={(event) =>
              onSettings({ ...settings, inputDeviceId: event.target.value || null })
            }
          >
            <option value="">Default input</option>
            {status.devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name}
                {device.isDefault ? " (default)" : ""}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Language</span>
          <select
            value={settings.language}
            onChange={(event) => onSettings({ ...settings, language: event.target.value })}
          >
            <option value="auto">Auto</option>
            <option value="en">English</option>
            <option value="es">Spanish</option>
            <option value="fr">French</option>
            <option value="de">German</option>
          </select>
        </label>
      </section>

      <section className="panel">
        <PanelHeading icon={Keyboard} title="Hotkey" />
        <label className="toggleLine">
          <input
            type="checkbox"
            checked={settings.hotkey.enabled}
            onChange={(event) =>
              onSettings({
                ...settings,
                hotkey: { ...settings.hotkey, enabled: event.target.checked }
              })
            }
          />
          <span>Enabled</span>
        </label>
        <label className="field">
          <span>Shortcut</span>
          <input
            value={settings.hotkey.shortcut}
            onChange={(event) =>
              onSettings({
                ...settings,
                hotkey: { ...settings.hotkey, shortcut: event.target.value }
              })
            }
          />
        </label>
        <div className="segmented">
          {(["toggle", "hold"] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              className={settings.hotkey.mode === mode ? "selected" : ""}
              onClick={() =>
                onSettings({ ...settings, hotkey: { ...settings.hotkey, mode } })
              }
            >
              {capitalize(mode)}
            </button>
          ))}
        </div>
      </section>

      {lastResult ? (
        <section className="panel wide">
          <PanelHeading icon={CheckCircle2} title="Last Dictation" />
          <textarea className="transcriptBox" readOnly value={lastResult.finalText} />
          {lastResult.enhancementError ? (
            <p className="inlineWarning">{lastResult.enhancementError}</p>
          ) : null}
        </section>
      ) : null}
    </div>
  );
}

function ModelsPanel({
  settings,
  models,
  progress,
  onDownload,
  onClear,
  onSettings
}: {
  settings: AppSettings;
  models: ModelCacheStatus[];
  progress: Record<string, ModelDownloadProgress>;
  onDownload: (model: ModelCacheStatus) => Promise<void>;
  onClear: (modelId?: string) => Promise<void>;
  onSettings: (settings: AppSettings) => Promise<void>;
}) {
  return (
    <div className="modelList">
      {models.map((model) => {
        const active = settings.transcription.modelId === model.model.id;
        const currentProgress = progress[model.model.id];
        const percent =
          currentProgress?.totalBytes && currentProgress.totalBytes > 0
            ? Math.round((currentProgress.downloadedBytes / currentProgress.totalBytes) * 100)
            : null;

        return (
          <article key={model.model.id} className={active ? "itemCard selectedCard" : "itemCard"}>
            <div>
              <h2>{model.model.name}</h2>
              <p>
                {model.model.languages} · {model.model.sizeMb} MB · {model.model.recommendedMinRamGb} GB RAM
              </p>
              <span className={model.isDownloaded ? "pill ready" : "pill"}>
                {model.isDownloaded ? "Downloaded" : "Missing"}
              </span>
            </div>
            <div className="itemActions">
              <button
                className="secondaryButton"
                type="button"
                onClick={() =>
                  onSettings({
                    ...settings,
                    transcription: { ...settings.transcription, modelId: model.model.id }
                  })
                }
              >
                <CheckCircle2 size={17} />
                Select
              </button>
              <button
                className="secondaryButton"
                type="button"
                onClick={() => onDownload(model)}
                disabled={model.isDownloaded}
              >
                <Download size={17} />
                {percent === null ? "Download" : `${percent}%`}
              </button>
              <button
                className="iconButton danger"
                type="button"
                title="Clear model"
                onClick={() => onClear(model.model.id)}
                disabled={!model.isDownloaded}
              >
                <Trash2 size={17} />
              </button>
            </div>
          </article>
        );
      })}
    </div>
  );
}

function EnhancementPanel({
  status,
  apiKeyDraft,
  onDraftChange,
  onSaveKey,
  onDeleteKey,
  onSettings
}: {
  status: AppStatus;
  apiKeyDraft: string;
  onDraftChange: (value: string) => void;
  onSaveKey: () => Promise<void>;
  onDeleteKey: () => Promise<void>;
  onSettings: (settings: AppSettings) => Promise<void>;
}) {
  const { settings } = status;
  const secretExists = secretStatusForProvider(status, settings.enhancement.provider);

  return (
    <div className="sectionGrid">
      <section className="panel wide">
        <PanelHeading icon={Wand2} title="Provider" />
        <label className="toggleLine">
          <input
            type="checkbox"
            checked={settings.enhancement.enabled}
            onChange={(event) =>
              onSettings({
                ...settings,
                enhancement: { ...settings.enhancement, enabled: event.target.checked }
              })
            }
          />
          <span>Enhancement enabled</span>
        </label>

        <label className="field">
          <span>Provider</span>
          <select
            value={settings.enhancement.provider}
            onChange={(event) =>
              onSettings({
                ...settings,
                enhancement: {
                  ...settings.enhancement,
                  provider: event.target.value as EnhancementProvider
                }
              })
            }
          >
            <option value="openAi">OpenAI</option>
            <option value="groq">Groq</option>
            <option value="customOpenAiCompatible">Custom</option>
          </select>
        </label>

        <label className="field">
          <span>Base URL</span>
          <input
            value={settings.enhancement.baseUrl}
            onChange={(event) =>
              onSettings({
                ...settings,
                enhancement: { ...settings.enhancement, baseUrl: event.target.value }
              })
            }
          />
        </label>

        <label className="field">
          <span>Model</span>
          <input
            value={settings.enhancement.model}
            onChange={(event) =>
              onSettings({
                ...settings,
                enhancement: { ...settings.enhancement, model: event.target.value }
              })
            }
          />
        </label>
      </section>

      <section className="panel">
        <PanelHeading icon={KeyRound} title="API Key" />
        <div className="statusStrip inline">
          <StatusDot ok={secretExists} />
          <span>{secretExists ? "Stored" : "Not stored"}</span>
        </div>
        <label className="field">
          <span>Key</span>
          <input
            type="password"
            value={apiKeyDraft}
            onChange={(event) => onDraftChange(event.target.value)}
          />
        </label>
        <div className="buttonRow">
          <button className="primaryButton" type="button" onClick={onSaveKey}>
            <Save size={17} />
            Save
          </button>
          <button className="secondaryButton" type="button" onClick={onDeleteKey}>
            <Trash2 size={17} />
            Delete
          </button>
        </div>
      </section>

      <section className="panel">
        <PanelHeading icon={Settings} title="Prompt" />
        <div className="segmented vertical">
          {(["default", "cleanTranscript", "email", "codeComments"] as PromptProfile[]).map(
            (profile) => (
              <button
                key={profile}
                type="button"
                className={settings.enhancement.promptProfile === profile ? "selected" : ""}
                onClick={() =>
                  onSettings({
                    ...settings,
                    enhancement: { ...settings.enhancement, promptProfile: profile }
                  })
                }
              >
                {labelForPrompt(profile)}
              </button>
            )
          )}
        </div>
      </section>
    </div>
  );
}

function HistoryPanel({
  entries,
  onClear
}: {
  entries: HistoryEntry[];
  onClear: () => Promise<void>;
}) {
  return (
    <div className="historyView">
      <div className="listToolbar">
        <span>{entries.length} entries</span>
        <button className="secondaryButton" type="button" onClick={onClear}>
          <Trash2 size={17} />
          Clear
        </button>
      </div>

      <div className="historyList">
        {entries.map((entry) => (
          <article key={entry.id} className="itemCard historyItem">
            <div>
              <h2>{new Date(entry.createdAt).toLocaleString()}</h2>
              <p>{entry.finalText}</p>
            </div>
            <span className={entry.inserted ? "pill ready" : "pill"}>{entry.modelId}</span>
          </article>
        ))}
      </div>
    </div>
  );
}

function OverlayView() {
  const [snapshot, setSnapshot] = useState<RecordingSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const timer = window.setInterval(() => {
      api
        .getRecordingSnapshot()
        .then(setSnapshot)
        .catch((caught) => setError(errorMessage(caught)));
    }, 250);
    return () => window.clearInterval(timer);
  }, []);

  async function finish() {
    try {
      await api.finishDictation(true);
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  async function cancel() {
    try {
      await api.cancelDictation();
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  return (
    <main className="overlay">
      <div className="overlayMeter">
        <div className="miniPulse" style={{ "--level": snapshot?.level.rms ?? 0 } as React.CSSProperties}>
          <Mic size={24} />
        </div>
        <div className="overlayText">
          <strong>{snapshot ? formatDuration(snapshot.durationMs) : "Recording"}</strong>
          <div className="levelTrack">
            <span style={{ width: `${Math.min((snapshot?.level.peak ?? 0) * 100, 100)}%` }} />
          </div>
        </div>
      </div>
      <div className="overlayActions">
        <button className="iconButton" type="button" onClick={finish} title="Finish">
          <Square size={18} />
        </button>
        <button className="iconButton danger" type="button" onClick={cancel} title="Cancel">
          <X size={18} />
        </button>
      </div>
      {error ? <span className="overlayError">{error}</span> : null}
    </main>
  );
}

function PanelHeading({ icon: Icon, title }: { icon: typeof Mic; title: string }) {
  return (
    <div className="panelHeading">
      <Icon size={18} />
      <h2>{title}</h2>
    </div>
  );
}

function StatusDot({ ok }: { ok: boolean }) {
  return <span className={ok ? "dot ok" : "dot"} />;
}

function safeWindowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function formatDuration(durationMs: number) {
  const seconds = Math.floor(durationMs / 1000);
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}

function capitalize(value: string) {
  return `${value.slice(0, 1).toUpperCase()}${value.slice(1)}`;
}

function secretKeyForProvider(provider: EnhancementProvider) {
  if (provider === "openAi") return "openai";
  if (provider === "groq") return "groq";
  return "custom-openai-compatible";
}

function secretStatusForProvider(status: AppStatus, provider: EnhancementProvider) {
  if (provider === "openAi") return status.openaiSecret.exists;
  if (provider === "groq") return status.groqSecret.exists;
  return status.customSecret.exists;
}

function labelForPrompt(profile: PromptProfile) {
  switch (profile) {
    case "cleanTranscript":
      return "Clean Transcript";
    case "email":
      return "Email";
    case "codeComments":
      return "Code Comments";
    default:
      return "Default";
  }
}

