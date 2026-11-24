import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { MonitorCog } from "lucide-react";
import { t, useTranslation } from "../i18n";
import { SettingRow } from "./SettingRow";

/** A connected display, as `list_monitors` reports it. */
interface MonitorInfo {
	id: string;
	name: string;
	x: number;
	y: number;
	width: number;
	height: number;
	primary: boolean;
}

type Surface = "dock" | "notch";

const PRIMARY = "primary";
const ALL = "all";
const SPECIFIC = "monitor:";

/** The select's value for the saved settings (see monitor_logic.rs for the keys). */
export function monitorChoice(mode: unknown, id: unknown): string {
	if (mode === ALL) return ALL;
	if (mode === "specific" && typeof id === "string" && id) return SPECIFIC + id;
	return PRIMARY;
}

/**
 * Where the notch or the dock is shown with more than one display: the main
 * display (Bloom's default), every display, or one chosen display. Loads and
 * saves its own settings, then asks the backend to move the windows.
 */
export function MonitorPicker({ surface }: { surface: Surface }) {
	useTranslation();
	const modeKey = `bloom-${surface}-monitor-mode`;
	const idKey = `bloom-${surface}-monitor-id`;
	const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
	const [value, setValue] = useState(PRIMARY);
	const [busy, setBusy] = useState(false);

	useEffect(() => {
		let disposed = false;
		const load = async () => {
			const [list, settings] = await Promise.all([
				invoke<MonitorInfo[]>("list_monitors").catch(() => [] as MonitorInfo[]),
				invoke<Record<string, unknown>>("load_settings").catch(
					() => ({}) as Record<string, unknown>
				)
			]);
			if (disposed) return;
			setMonitors(list);
			setValue(monitorChoice(settings[modeKey], settings[idKey]));
		};
		void load();
		const unlisten = listen("monitors-changed", () => void load());
		return () => {
			disposed = true;
			void unlisten.then((stop) => stop());
		};
	}, [modeKey, idKey]);

	// One display and nothing saved: there is nothing to choose.
	if (monitors.length < 2 && value === PRIMARY) return null;

	const choose = async (next: string) => {
		const previous = value;
		setValue(next);
		setBusy(true);
		const id = next.startsWith(SPECIFIC) ? next.slice(SPECIFIC.length) : null;
		const mode = id !== null ? "specific" : next;
		try {
			if (id !== null) await invoke("save_setting", { key: idKey, value: id });
			await invoke("save_setting", { key: modeKey, value: mode });
			try {
				if (id !== null) localStorage.setItem(idKey, id);
				localStorage.setItem(modeKey, mode);
			} catch {
				// The file is saved; the localStorage mirror is only a startup cache.
			}
			await invoke("apply_monitor_settings");
		} catch (error) {
			console.error(`Failed to change the ${surface} display:`, error);
			setValue(previous);
		} finally {
			setBusy(false);
		}
	};

	const savedMonitorMissing =
		value.startsWith(SPECIFIC) && !monitors.some((m) => SPECIFIC + m.id === value);

	return (
		<SettingRow
			previewId={`${surface}-monitor`}
			icon={MonitorCog}
			label={t("settings.monitors.label")}
			desc={surface === "dock" ? t("settings.monitors.dockDesc") : t("settings.monitors.notchDesc")}
		>
			<select
				className="settings-select"
				value={value}
				disabled={busy}
				onChange={(e) => void choose(e.target.value)}
			>
				<option value={PRIMARY}>{t("settings.monitors.primary")}</option>
				<option value={ALL}>{t("settings.monitors.all")}</option>
				{monitors.map((m) => (
					<option key={m.id} value={SPECIFIC + m.id}>
						{m.primary ? t("settings.monitors.primaryDisplay", { name: m.name }) : m.name}
					</option>
				))}
				{savedMonitorMissing && (
					<option value={value}>{t("settings.monitors.disconnected")}</option>
				)}
			</select>
		</SettingRow>
	);
}
