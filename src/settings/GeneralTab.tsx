import { t, useTranslation, LOCALES, type LanguageSetting } from "../i18n";
import { useState } from "react";
import {
	Power,
	Download,
	Clock,
	BatteryWarning,
	RefreshCw,
	RotateCcw,
	LogOut,
	Compass
} from "lucide-react";
import { SettingRow } from "./SettingRow";

interface GeneralTabProps {
	language: LanguageSetting;
	handleLanguageChange: (code: string) => void;
	autostart: boolean;
	toggleAutostart: () => void;
	timeFormat24h: boolean;
	toggleTimeFormat24h: () => void;
	showUpdateIndicator: boolean;
	toggleUpdateIndicator: () => void;
	lowBatteryThreshold: number;
	handleThresholdChange: (val: number) => void;
	restartBloom: () => void;
	resetToDefaults: () => void;
	quitBloom: () => void;
	openOnboarding: () => Promise<void>;
}

export function GeneralTab({
	autostart,
	toggleAutostart,
	language,
	handleLanguageChange,
	timeFormat24h,
	toggleTimeFormat24h,
	showUpdateIndicator,
	toggleUpdateIndicator,
	lowBatteryThreshold,
	handleThresholdChange,
	restartBloom,
	resetToDefaults,
	quitBloom,
	openOnboarding
}: GeneralTabProps) {
	useTranslation();
	const [setupError, setSetupError] = useState("");
	const [openingSetup, setOpeningSetup] = useState(false);
	return (
		<>
			<div className="setting-group-label">{t("settings.groups.system")}</div>
			<div className="setting-group">
				<SettingRow
					previewId="language"
					icon={Clock}
					label={t("settings.general.language")}
					desc={t("settings.general.languageFallbackDesc")}
				>
					<select
						className="settings-select"
						value={language}
						onChange={(e) => handleLanguageChange(e.target.value)}
					>
						<option value="system">{t("settings.general.languageSystem")}</option>
						{LOCALES.map((locale) => (
							<option key={locale.code} value={locale.code}>
								{locale.nativeName}
							</option>
						))}
					</select>
				</SettingRow>
				<SettingRow
					previewId="launch-at-login"
					icon={Power}
					label={t("settings.general.launchAtLogin")}
					desc={t("settings.general.launchAtLoginDesc")}
				>
					<label className="toggle-switch">
						<input type="checkbox" checked={autostart} onChange={toggleAutostart} />
						<span className="slider"></span>
					</label>
				</SettingRow>

				<SettingRow
					previewId="update-indicator"
					icon={Download}
					label={t("settings.general.updateIndicator")}
					desc={t("settings.general.updateIndicatorDesc")}
				>
					<label className="toggle-switch">
						<input type="checkbox" checked={showUpdateIndicator} onChange={toggleUpdateIndicator} />
						<span className="slider"></span>
					</label>
				</SettingRow>

				<SettingRow
					previewId="24-hour-time"
					icon={Clock}
					label={t("settings.general.time24h")}
					desc={t("settings.general.time24hDesc")}
				>
					<label className="toggle-switch">
						<input type="checkbox" checked={timeFormat24h} onChange={toggleTimeFormat24h} />
						<span className="slider"></span>
					</label>
				</SettingRow>

				<SettingRow
					previewId="low-battery-alert"
					icon={BatteryWarning}
					label={t("settings.general.lowBattery")}
					desc={t("settings.general.lowBatteryDesc", { percent: lowBatteryThreshold })}
					divider={false}
				>
					<input
						type="range"
						min="5"
						max="50"
						step="5"
						value={lowBatteryThreshold}
						onChange={(e) => handleThresholdChange(parseInt(e.target.value))}
						className="settings-slider"
					/>
				</SettingRow>
			</div>

			<div className="setting-group-label">{t("settings.groups.app")}</div>
			<div className="setting-group">
				<SettingRow
					previewId="welcome-to-bloom"
					icon={Compass}
					label={t("ui.generalTab.welcomeToBloom")}
					desc={t("ui.generalTab.replaySetupAndTheQuickGuide")}
				>
					<button
						type="button"
						className="onboarding-settings-launch"
						disabled={openingSetup}
						onClick={async () => {
							setOpeningSetup(true);
							setSetupError("");
							try {
								await openOnboarding();
							} catch (error) {
								setSetupError(String(error));
							} finally {
								setOpeningSetup(false);
							}
						}}
					>
						{t("ui.generalTab.openSetup")}
					</button>
				</SettingRow>
				{setupError && (
					<p className="onboarding-settings-error" role="alert">
						{setupError}
					</p>
				)}
				<SettingRow
					previewId="reset-to-defaults"
					icon={RotateCcw}
					label={t("settings.general.reset")}
					desc={t("settings.general.resetDesc")}
					action
					danger
					onClick={resetToDefaults}
				/>
				<SettingRow
					previewId="restart-bloom"
					icon={RefreshCw}
					label={t("settings.general.restart")}
					desc={t("settings.general.restartDesc")}
					action
					onClick={restartBloom}
				/>
				<SettingRow
					previewId="quit-bloom"
					icon={LogOut}
					label={t("settings.general.quit")}
					desc={t("settings.general.quitDesc")}
					action
					danger
					onClick={quitBloom}
					divider={false}
				/>
			</div>
		</>
	);
}
