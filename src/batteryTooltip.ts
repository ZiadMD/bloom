import { t } from "./i18n";
/** Mirrors `BatteryDetails` in src-tauri/src/battery.rs. */
export interface BatteryDetails {
	present: boolean;
	percent: number | null;
	plugged_in: boolean;
	charging: boolean;
	seconds_left: number | null;
	seconds_to_full: number | null;
	health_percent: number | null;
	power_mode: string | null;
}

/** Durations as the taskbar writes them: "3 hr 12 min", "45 min", "1 hr". */
export function formatDuration(seconds: number): string {
	const minutes = Math.max(1, Math.round(seconds / 60));
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	if (hours === 0) return t("ui.batteryTooltip.restMin", { rest: rest });
	return rest === 0
		? t("ui.batteryTooltip.hoursHr", { hours: hours })
		: t("ui.batteryTooltip.hoursHrRestMin", { hours: hours, rest: rest });
}

/**
 * The battery widget's tooltip: the taskbar battery icon's status line, then
 * what Windows' battery settings add. Until the details arrive (or on a
 * device that reports none), the status line comes from the webview's own
 * level and charging state.
 */
export function batteryTooltip(
	details: BatteryDetails | null,
	fallback: { level: number; charging: boolean },
	energySaver: boolean
): string {
	const percent = details?.percent ?? fallback.level;
	const charging = details ? details.charging : fallback.charging;
	const pluggedIn = details ? details.plugged_in : fallback.charging;
	const lines: string[] = [];

	if (pluggedIn && percent >= 100 && !charging) {
		lines.push(t("ui.batteryTooltip.fullyChargedPercent", { percent: percent }));
	} else if (pluggedIn) {
		lines.push(t(charging ? "battery.pluggedCharging" : "battery.pluggedNotCharging", { percent }));
	} else if (details?.seconds_left) {
		lines.push(
			t("ui.batteryTooltip.value1PercentRemaining", {
				value1: formatDuration(details.seconds_left),
				percent: percent
			})
		);
	} else {
		lines.push(t("ui.batteryTooltip.percentRemaining", { percent: percent }));
	}

	if (charging && details?.seconds_to_full) {
		lines.push(
			t("ui.batteryTooltip.aboutValue1UntilFull", {
				value1: formatDuration(details.seconds_to_full)
			})
		);
	}
	if (details?.power_mode)
		lines.push(t("ui.batteryTooltip.powerModeValue1", { value1: details.power_mode }));
	if (energySaver) lines.push(t("ui.batteryTooltip.energySaverIsOn"));
	if (details?.health_percent != null) {
		lines.push(
			t("ui.batteryTooltip.batteryHealthValue1OfOriginalCapacity", {
				value1: details.health_percent
			})
		);
	}
	return lines.join("\n");
}
