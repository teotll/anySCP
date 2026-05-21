import { useState, useEffect } from "react";
import { useSettingsStore } from "../../stores/settings-store";
import { CustomSelect } from "../shared/CustomSelect";
import type { CursorStyle } from "../../stores/settings-store";

// ─── Shared styles ───────────────────────────────────────────────────────────

const LABEL_CLASS = "text-[length:var(--text-xs)] font-medium text-text-primary";
const DESC_CLASS = "text-[length:var(--text-2xs)] text-text-muted mt-0.5";


const INPUT_CLASS = [
  "w-20 px-2.5 py-1.5 rounded-lg text-[length:var(--text-xs)] tabular-nums",
  "bg-bg-base border border-border text-text-primary",
  "outline-none focus:border-border-focus focus:ring-2 focus:ring-ring",
  "transition-[border-color,box-shadow] duration-[var(--duration-fast)]",
].join(" ");

// ─── Component ───────────────────────────────────────────────────────────────

export function SettingsPage() {
  const fontSize = useSettingsStore((s) => s.terminalFontSize);
  const cursorStyle = useSettingsStore((s) => s.terminalCursorStyle);
  const cursorBlink = useSettingsStore((s) => s.terminalCursorBlink);
  const lineHeight = useSettingsStore((s) => s.terminalLineHeight);
  const scrollback = useSettingsStore((s) => s.terminalScrollback);
  const transferConcurrency = useSettingsStore((s) => s.transferConcurrency);

  const setFontSize = useSettingsStore((s) => s.setTerminalFontSize);
  const setCursorStyle = useSettingsStore((s) => s.setTerminalCursorStyle);
  const setCursorBlink = useSettingsStore((s) => s.setTerminalCursorBlink);
  const setLineHeight = useSettingsStore((s) => s.setTerminalLineHeight);
  const setScrollback = useSettingsStore((s) => s.setTerminalScrollback);
  const setConcurrency = useSettingsStore((s) => s.setTransferConcurrency);

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-xl mx-auto px-6 py-8">
        {/* Header */}
        <h1 className="text-[length:var(--text-base)] font-semibold text-text-primary mb-6">
          Settings
        </h1>

        {/* Terminal Appearance */}
        <section className="mb-8">
          <h2 className="text-[length:var(--text-xs)] font-semibold uppercase tracking-wider text-text-muted mb-4">
            Terminal
          </h2>

          <div className="flex flex-col gap-4">
            {/* Font Size */}
            <SettingRow>
              <div>
                <label htmlFor="s-fontsize" className={LABEL_CLASS}>Font Size</label>
                <p className={DESC_CLASS}>Size in pixels (8–32)</p>
              </div>
              <NumberSetting id="s-fontsize" value={fontSize} min={8} max={32} step={1} onChange={setFontSize} />
            </SettingRow>

            {/* Cursor Style */}
            <SettingRow>
              <div>
                <label htmlFor="s-cursor" className={LABEL_CLASS}>Cursor Style</label>
                <p className={DESC_CLASS}>Shape of the terminal cursor</p>
              </div>
              <CustomSelect
                id="s-cursor"
                value={cursorStyle}
                onChange={(v) => setCursorStyle(v as CursorStyle)}
                options={[
                  { value: "bar", label: "Bar" },
                  { value: "block", label: "Block" },
                  { value: "underline", label: "Underline" },
                ]}
                className="w-32"
              />
            </SettingRow>

            {/* Cursor Blink */}
            <SettingRow>
              <div>
                <label htmlFor="s-blink" className={LABEL_CLASS}>Cursor Blink</label>
                <p className={DESC_CLASS}>Animate the cursor</p>
              </div>
              <Toggle
                id="s-blink"
                checked={cursorBlink}
                onChange={setCursorBlink}
              />
            </SettingRow>

            {/* Line Height */}
            <SettingRow>
              <div>
                <label htmlFor="s-lineheight" className={LABEL_CLASS}>Line Height</label>
                <p className={DESC_CLASS}>Spacing between lines (1.0–2.0)</p>
              </div>
              <NumberSetting id="s-lineheight" value={lineHeight} min={1.0} max={2.0} step={0.1} onChange={setLineHeight} />
            </SettingRow>

            {/* Scrollback */}
            <SettingRow>
              <div>
                <label htmlFor="s-scrollback" className={LABEL_CLASS}>Scrollback Buffer</label>
                <p className={DESC_CLASS}>Number of lines to keep in history (500–100,000)</p>
              </div>
              <NumberSetting id="s-scrollback" value={scrollback} min={500} max={100000} step={500} onChange={setScrollback} />
            </SettingRow>
          </div>
        </section>

        {/* Transfers */}
        <section className="mb-8">
          <h2 className="text-[length:var(--text-xs)] font-semibold uppercase tracking-wider text-text-muted mb-4">
            Transfers
          </h2>

          <div className="flex flex-col gap-4">
            <SettingRow>
              <div>
                <label htmlFor="s-concurrency" className={LABEL_CLASS}>Concurrent Transfers</label>
                <p className={DESC_CLASS}>Maximum simultaneous file transfers (1–10)</p>
              </div>
              <NumberSetting id="s-concurrency" value={transferConcurrency} min={1} max={10} step={1} onChange={setConcurrency} />
            </SettingRow>
          </div>
        </section>

        {/* Note */}
        <p className="text-[length:var(--text-2xs)] text-text-muted">
          Terminal settings apply to new terminals. Existing terminals keep their current settings.
        </p>
      </div>
    </div>
  );
}

// ─── Sub-components ──────────────────────────────────────────────────────────

function SettingRow({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3 rounded-lg bg-bg-surface border border-border/50">
      {children}
    </div>
  );
}

function Toggle({ id, checked, onChange }: { id: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      id={id}
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={[
        "relative w-9 h-5 rounded-full shrink-0",
        "transition-colors duration-[var(--duration-fast)]",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        checked ? "bg-accent" : "bg-bg-muted",
      ].join(" ")}
    >
      <span
        className={[
          "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-text-primary",
          "transition-transform duration-[var(--duration-fast)]",
          checked ? "translate-x-4" : "translate-x-0",
        ].join(" ")}
      />
    </button>
  );
}

/** Number input that uses local state while typing, commits on blur/Enter. */
function NumberSetting({ id, value, min, max, step, onChange }: {
  id: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  const [local, setLocal] = useState(String(value));

  // Sync from store when value changes externally
  useEffect(() => {
    setLocal(String(value));
  }, [value]);

  const commit = () => {
    const n = Number(local);
    if (isNaN(n)) {
      setLocal(String(value)); // revert
      return;
    }
    const clamped = Math.max(min, Math.min(max, n));
    onChange(clamped);
    setLocal(String(clamped));
  };

  return (
    <input
      id={id}
      type="text"
      inputMode="decimal"
      value={local}
      onChange={(e) => setLocal(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          commit();
          (e.target as HTMLInputElement).blur();
        }
        // Arrow keys for increment/decrement
        if (e.key === "ArrowUp") {
          e.preventDefault();
          const next = Math.min(max, Number(local) + step);
          setLocal(String(Number(next.toFixed(2))));
          onChange(next);
        }
        if (e.key === "ArrowDown") {
          e.preventDefault();
          const next = Math.max(min, Number(local) - step);
          setLocal(String(Number(next.toFixed(2))));
          onChange(next);
        }
      }}
      className={INPUT_CLASS}
    />
  );
}
