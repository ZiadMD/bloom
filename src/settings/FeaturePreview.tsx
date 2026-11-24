import { t, useTranslation, getLocale } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import {
	ArrowUpDown,
	BatteryFull,
	BatteryWarning,
	ChevronDown,
	CloudSun,
	Cpu,
	HardDrive,
	MemoryStick,
	Music2,
	RotateCcw,
	Sun,
	Volume2
} from "lucide-react";
import { EdgeSurface } from "../components/EdgeSurface";
import { DeviceArt, DeviceArtDefs } from "../devices/DeviceArt";
import type { useSettings } from "./useSettings";
import type { SettingsTab } from "./types";
import { PreviewMediaPlayer } from "./PreviewMediaPlayer";
import { PreviewDock } from "./PreviewDock";
import "../App.css";
import "../Dock.css";
import "../Overlay.css";

type Settings = ReturnType<typeof useSettings>;
type Feature =
	| "clock"
	| "battery"
	| "theme"
	| "resting"
	| "device"
	| "music"
	| "dock"
	| "volume"
	| "brightness";

function getFEATURES(): Partial<
	Record<SettingsTab, { id: Feature; label: string; description: string }[]>
> {
	return {
		general: [
			{
				id: "clock",
				label: t("ui.featurePreview.clock"),
				description: t("ui.featurePreview.timeFormatAndUpdateStatusAsTheyAppearInThe")
			},
			{
				id: "battery",
				label: t("ui.featurePreview.batteryAlert"),
				description: t("ui.featurePreview.theAlertThresholdControlsWhenTheLowBatteryPulseAppears")
			}
		],
		appearance: [
			{
				id: "theme",
				label: t("settings.groups.theme"),
				description: t("ui.featurePreview.yourCurrentThemeAppliedToBloomSActualNotchAnd")
			}
		],
		notch: [
			{
				id: "resting",
				label: t("ui.featurePreview.resting"),
				description: t("ui.featurePreview.theSameCompactNotchShownOnYourDesktop")
			},
			{
				id: "device",
				label: t("ui.featurePreview.deviceAlert"),
				description: t("ui.featurePreview.bloomSUsbAlertWakesTheNotchEvenWhenIt")
			},
			{
				id: "music",
				label: t("ui.dockExtras.music"),
				description: t("ui.featurePreview.musicControlsAppearWhenAPlayerIsActive")
			}
		],
		dock: [
			{
				id: "dock",
				label: t("settings.tabs.dock"),
				description: t("ui.featurePreview.theBloomDockRisesFromTheBottomEdge")
			}
		],
		overlays: [
			{
				id: "volume",
				label: t("notch.music.volume"),
				description: t("ui.featurePreview.bloomSVolumeHudSlidesInFromTheLeftEdge")
			},
			{
				id: "brightness",
				label: t("notch.commandCenter.brightness"),
				description: t("ui.featurePreview.theBrightnessHudAppearsOnTheRightEdge")
			}
		]
	};
}

interface FeaturePreviewProps {
	tab: SettingsTab;
	settings: Settings;
	focus: string;
}

function NotchPreview({
	feature,
	settings,
	runId,
	revealed
}: {
	feature: Feature;
	settings: Settings;
	runId: number;
	revealed: boolean;
}) {
	useTranslation();
	const [musicOpen, setMusicOpen] = useState(feature === "music");
	const [previewLayout, setPreviewLayout] = useState(settings.mediaLayout);
	const [volumeExpanded, setVolumeExpanded] = useState(false);
	useEffect(() => setPreviewLayout(settings.mediaLayout), [settings.mediaLayout]);
	const isDevice = feature === "device";
	const isBattery = feature === "battery";
	const isMusic = feature === "music";
	const visualizerEnabled =
		(localStorage.getItem("bloom-media-visualizer-enabled") ??
			localStorage.getItem("bloom-visualizer-enabled")) !== "false";
	const albumArtEnabled = localStorage.getItem("bloom-media-album-art-enabled") !== "false";
	const isPeek = feature === "resting" && settings.notchMode === "peek";
	const widgets =
		feature === "resting" ? [...settings.statusWidgets.left, ...settings.statusWidgets.right] : [];
	const width = isDevice
		? 330
		: isBattery
			? 235
			: isMusic
				? musicOpen
					? previewLayout === "compact"
						? 300
						: 340
					: settings.musicCompactNotch
						? 230
						: 270
				: feature === "resting"
					? Math.min(
							420,
							Math.max(300, 200 + widgets.length * 50 - (settings.timeFormat24h ? 16 : 0))
						)
					: 180;
	const lowBattery = 15 <= settings.lowBatteryThreshold;
	const clock = new Date(2024, 0, 1, 15, 42).toLocaleTimeString(getLocale(), {
		hour: "2-digit",
		minute: "2-digit",
		hour12: !settings.timeFormat24h
	});
	const renderWidget = (id: string) => {
		if (id === "weather")
			return settings.weatherEnabled ? (
				<span className="passive-feature settings-preview-widget" key={id}>
					<CloudSun size={12} />
					{settings.tempUnitFahrenheit ? "72°F" : "22°C"}
				</span>
			) : null;
		if (id === "battery")
			return (
				<span className="passive-feature settings-preview-widget" key={id}>
					<BatteryFull size={12} />
					85%
				</span>
			);
		if (id === "cpu")
			return (
				<span className="passive-feature settings-preview-widget" key={id}>
					<Cpu size={12} />
					24%
				</span>
			);
		if (id === "ram")
			return (
				<span className="passive-feature settings-preview-widget" key={id}>
					<MemoryStick size={12} />
					48%
				</span>
			);
		if (id === "disk")
			return (
				<span className="passive-feature settings-preview-widget" key={id}>
					<HardDrive size={12} />
					{"120GB"}
				</span>
			);
		if (id === "net")
			return (
				<span className="passive-feature settings-preview-widget" key={id}>
					<ArrowUpDown size={12} />
					{t("ui.featurePreview.1mb")}
				</span>
			);
		return null;
	};
	if (isMusic && !settings.musicModeEnabled)
		return <div className="settings-preview-empty">{t("ui.featurePreview.musicModeIsOff")}</div>;
	return (
		<>
			<motion.div
				className={`bloom settings-preview-notch${musicOpen && isMusic ? " expanded" : ""}`}
				initial={{
					y: settings.notchMode === "fixed" ? -12 : -80,
					opacity: 0,
					width: 40,
					height: 36
				}}
				animate={{
					y: isPeek && !revealed ? -27 : 0,
					opacity: 1,
					width,
					height: isDevice
						? 76
						: isMusic && musicOpen
							? previewLayout === "compact"
								? 132 + (volumeExpanded ? 36 : 0)
								: 120
							: 36,
					scale: feature === "theme" ? settings.scale : 1
				}}
				transition={{ type: "spring", stiffness: 450, damping: 32 }}
				onClick={isMusic && !musicOpen ? () => setMusicOpen(true) : undefined}
			>
				<EdgeSurface edge="top" flares />
				{isMusic && musicOpen ? (
					<PreviewMediaPlayer
						layout={previewLayout}
						onLayoutChange={setPreviewLayout}
						volumeExpanded={volumeExpanded}
						onVolumeExpandedChange={setVolumeExpanded}
					/>
				) : isDevice ? (
					<motion.div
						className="device-alert"
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
						transition={{ delay: 0.18, duration: 0.2 }}
					>
						<DeviceArt art="flash-drive" phase="play" runId={runId} size={60} />
						<div className="device-alert-text">
							<span className="device-alert-name">{t("ui.featurePreview.flashDrive")}</span>
							<span className="device-alert-status">{t("ui.featurePreview.pluggedIn")}</span>
						</div>
					</motion.div>
				) : isBattery ? (
					<div className="main-row">
						<div className="main-row-inner">
							<div className="power-pulse-content">
								<BatteryWarning size={16} color={lowBattery ? "#ff453a" : "currentColor"} />
								<span className="label" style={{ color: lowBattery ? "#ff453a" : "inherit" }}>
									{lowBattery ? t("notch.status.lowBattery") : t("notch.status.onBattery")} · 15%
								</span>
							</div>
						</div>
					</div>
				) : isMusic ? (
					<div className="main-row">
						<div className="main-row-inner settings-preview-clock">
							<div className="side-content left">
								{visualizerEnabled && (
									<div
										className="visualizer-horizontal settings-preview-static-visualizer"
										aria-label={t("ui.featurePreview.sampleAudioVisualizer")}
									>
										<i className="bar-horizontal" />
										<i className="bar-horizontal" />
										<i className="bar-horizontal" />
										<i className="bar-horizontal" />
										<i className="bar-horizontal" />
									</div>
								)}
							</div>
							<div className="time-center settings-preview-time">{clock}</div>
							<div className="side-content right">
								{albumArtEnabled && (
									<div className="album-art-glow-wrapper">
										{settings.mediaCompactGlowEnabled && (
											<span className="settings-preview-album-glow" />
										)}
										<span className="album-art">
											<span className="album-art-inner">
												<span className="album-art-placeholder">
													<Music2 className="music-placeholder-svg-small" />
												</span>
											</span>
										</span>
									</div>
								)}
							</div>
						</div>
					</div>
				) : (
					<div className="main-row">
						<div className="main-row-inner settings-preview-clock">
							<div className="side-content left">
								{feature === "resting" && (
									<div className="passive-features-group">
										{settings.statusWidgets.left.map(renderWidget)}
									</div>
								)}
							</div>
							<div className="time-center settings-preview-time">
								<span>{clock}</span>
								{settings.showUpdateIndicator && (
									<span
										className="settings-preview-update-dot"
										aria-label={t("ui.featurePreview.updateIndicator")}
									/>
								)}
							</div>
							<div className="side-content right">
								{feature === "resting" && (
									<div className="passive-features-group">
										{settings.statusWidgets.right.map(renderWidget)}
									</div>
								)}
							</div>
						</div>
					</div>
				)}
			</motion.div>
			{isMusic && (
				<button
					type="button"
					className="settings-preview-player-toggle"
					onClick={() => {
						setMusicOpen((open) => !open);
						setVolumeExpanded(false);
					}}
				>
					{musicOpen ? t("ui.featurePreview.collapseNotch") : t("ui.featurePreview.openPlayer")}
				</button>
			)}
		</>
	);
}

function OverlayPreview({ feature, settings }: { feature: Feature; settings: Settings }) {
	useTranslation();
	const [value, setValue] = useState(feature === "volume" ? 64 : 78);
	const [mixerOpen, setMixerOpen] = useState(false);
	const [appVolume, setAppVolume] = useState(52);
	const [appMuted, setAppMuted] = useState(false);
	const volume = feature === "volume";
	const enabled = volume ? settings.volumeOverlayEnabled : settings.brightnessOverlayEnabled;
	const edgeEnabled = volume ? settings.volumeEdgeEnabled : settings.brightnessEdgeEnabled;
	const changeBar = (event: React.PointerEvent<HTMLDivElement>) => {
		const rect = event.currentTarget.getBoundingClientRect();
		setValue(
			Math.max(0, Math.min(100, Math.round((1 - (event.clientY - rect.top) / rect.height) * 100)))
		);
	};
	if (!enabled)
		return (
			<div className="settings-preview-empty">
				{t(volume ? "preview.volumeOff" : "preview.brightnessOff")}
			</div>
		);
	return (
		<div
			className={
				"settings-preview-overlay-frame " +
				(volume ? "left" : "right") +
				(edgeEnabled ? " edge-on" : "")
			}
		>
			<motion.div
				className={`settings-preview-overlay ${volume ? "volume-notch-wrapper" : "brightness-notch-wrapper"}`}
				initial={{ x: volume ? -55 : 55, opacity: 0 }}
				animate={{ x: 0, opacity: 1 }}
				transition={{ type: "spring", stiffness: 430, damping: 30 }}
			>
				<EdgeSurface edge={volume ? "left" : "right"} />
				<div
					className={volume ? "volume-notch" : "brightness-notch"}
					style={volume && mixerOpen ? { width: 102 } : undefined}
				>
					<div className={volume ? "volume-notch-content-group" : "brightness-notch-content-group"}>
						<div
							className={`${volume ? "volume-notch-bar" : "brightness-notch-bar"} settings-preview-hud-bar`}
							role="slider"
							aria-label={t(volume ? "preview.sampleVolume" : "preview.sampleBrightness")}
							aria-valuemin={0}
							aria-valuemax={100}
							aria-valuenow={value}
							tabIndex={0}
							onPointerDown={changeBar}
							onPointerMove={(event) => {
								if (event.buttons) changeBar(event);
							}}
							onKeyDown={(event) => {
								if (event.key === "ArrowUp" || event.key === "ArrowRight")
									setValue((v) => Math.min(100, v + 5));
								if (event.key === "ArrowDown" || event.key === "ArrowLeft")
									setValue((v) => Math.max(0, v - 5));
							}}
						>
							<div
								className={volume ? "volume-notch-fill" : "brightness-notch-fill"}
								style={{ height: `${value}%` }}
							/>
						</div>
						{volume ? (
							<button
								type="button"
								className="volume-notch-action"
								aria-label={t("ui.featurePreview.sampleVolumeMixer")}
								onClick={() => setMixerOpen((open) => !open)}
							>
								<Volume2 size={16} />
							</button>
						) : (
							<div className="brightness-notch-icon">
								<Sun size={18} />
							</div>
						)}
					</div>
					{volume && mixerOpen && (
						<div className="volume-mixer-panel" style={{ width: 60 }}>
							<div className="volume-mixer-list" style={{ gridTemplateColumns: "1fr" }}>
								<div className="volume-mixer-tile">
									<div
										className="volume-mixer-bar settings-preview-hud-bar"
										role="slider"
										tabIndex={0}
										aria-label={t("ui.featurePreview.sampleAppVolume")}
										aria-valuemin={0}
										aria-valuemax={100}
										aria-valuenow={appMuted ? 0 : appVolume}
										onPointerDown={(event) => {
											const rect = event.currentTarget.getBoundingClientRect();
											setAppVolume(
												Math.max(
													0,
													Math.min(
														100,
														Math.round((1 - (event.clientY - rect.top) / rect.height) * 100)
													)
												)
											);
											setAppMuted(false);
										}}
										onPointerMove={(event) => {
											if (event.buttons) {
												const rect = event.currentTarget.getBoundingClientRect();
												setAppVolume(
													Math.max(
														0,
														Math.min(
															100,
															Math.round((1 - (event.clientY - rect.top) / rect.height) * 100)
														)
													)
												);
											}
										}}
										onKeyDown={(event) => {
											if (event.key === "ArrowUp") setAppVolume((v) => Math.min(100, v + 5));
											if (event.key === "ArrowDown") setAppVolume((v) => Math.max(0, v - 5));
										}}
									>
										<div
											className="volume-mixer-fill"
											style={{ height: `${appMuted ? 0 : appVolume}%` }}
										/>
									</div>
									<button
										type="button"
										className={`volume-mixer-action${appMuted ? " muted" : ""}`}
										aria-label={
											appMuted
												? t("ui.featurePreview.unmuteSampleApp")
												: t("ui.featurePreview.muteSampleApp")
										}
										onClick={() => setAppMuted((muted) => !muted)}
									>
										<span className="volume-mixer-action-fallback">{"M"}</span>
									</button>
								</div>
							</div>
						</div>
					)}
				</div>
			</motion.div>
		</div>
	);
}

function previewDescription(feature: Feature, settings: Settings, fallback: string) {
	const flag = (value: boolean) => t(value ? "preview.on" : "preview.off");
	const mode = (value: string) => t("common.behavior." + value);
	switch (feature) {
		case "clock":
			return t("preview.clock", {
				format: t(
					settings.timeFormat24h ? "ui.featurePreview.24HourTime" : "ui.featurePreview.12HourTime"
				),
				autostart: flag(settings.autostart),
				indicator: flag(settings.showUpdateIndicator)
			});
		case "battery":
			return t("preview.battery", { threshold: settings.lowBatteryThreshold });
		case "theme":
			return t("preview.theme", {
				theme: t("preview.themes." + settings.themeMode),
				opacity: Math.round(settings.themeOpacity * 100),
				scale: Math.round(settings.scale * 100),
				corners: flag(settings.cornersEnabled)
			});
		case "resting":
			return t("preview.resting", {
				mode: mode(settings.notchMode),
				visibility:
					settings.notchMode === "fixed"
						? t("ui.featurePreview.alwaysVisible")
						: t("preview.edgeDelay", { delay: settings.notchEdgeDelay }),
				weather: settings.weatherEnabled
					? settings.tempUnitFahrenheit
						? "°F"
						: "°C"
					: flag(false),
				panel: settings.notchRememberPanel
					? t("preview.remembered")
					: t("preview.panels." + settings.notchDefaultPanel)
			});
		case "music":
			return t("preview.music", {
				enabled: flag(settings.musicModeEnabled),
				compact: flag(settings.musicCompactNotch),
				layout: t("common.mediaLayout." + settings.mediaLayout),
				glow: flag(settings.mediaCompactGlowEnabled)
			});
		case "dock":
			return t("preview.dock", {
				mode: settings.dockEnabled
					? t("preview.dockMode", { mode: mode(settings.dockMode) })
					: t("ui.featurePreview.dockOff"),
				icons: flag(settings.dockIconOnly),
				position:
					settings.dockExtrasPosition === "off"
						? t("preview.off")
						: t("common." + settings.dockExtrasPosition),
				previews: flag(settings.dockPreviewEnabled)
			});
		case "volume":
			return t("preview.volume", {
				enabled: flag(settings.volumeOverlayEnabled),
				edge: flag(settings.volumeEdgeEnabled)
			});
		case "brightness":
			return t("preview.brightness", {
				enabled: flag(settings.brightnessOverlayEnabled),
				edge: flag(settings.brightnessEdgeEnabled)
			});
		default:
			return fallback;
	}
}

export function FeaturePreview({ tab, settings, focus }: FeaturePreviewProps) {
	useTranslation();
	const choices = getFEATURES()[tab];
	const [feature, setFeature] = useState<Feature>("clock");
	const [replay, setReplay] = useState(0);
	const [expanded, setExpanded] = useState(false);
	const [revealed, setRevealed] = useState(false);
	const previousWidgets = useRef(settings.statusWidgets);

	useEffect(() => {
		const first = getFEATURES()[tab]?.[0];
		if (first) setFeature(first.id);
		setReplay((n) => n + 1);
		setRevealed(false);
	}, [tab]);

	useEffect(() => {
		if (!focus.startsWith(tab + ":")) return;
		const label = focus.split(":")[1];
		const next: Partial<Record<SettingsTab, Record<string, Feature>>> = {
			general: {
				"launch-at-login": "clock",
				"24-hour-time": "clock",
				"update-indicator": "clock",
				"low-battery-alert": "battery"
			},
			appearance: {
				"theme-mode": "theme",
				"custom-theme-color": "theme",
				"background-opacity": "theme",
				"color-saturation": "theme",
				"background-brightness": "theme",
				"screen-corners": "theme",
				"edge-shine": "theme",
				"ui-font-scale": "theme"
			},
			notch: {
				"notch-behavior": "resting",
				"notch-monitor": "resting",
				"peek-delay": "resting",
				"remember-last-panel": "resting",
				"default-panel": "resting",
				"calendar-timer": "resting",
				"monday-first-calendar": "resting",
				"command-center": "resting",
				"connected-devices": "device",
				"timer-sound": "resting",
				"keep-items": "resting",
				"weather-status": "resting",
				"status-widgets": "resting",
				"music-mode": "music",
				"compact-mode": "music",
				"media-layout": "music",
				"ambient-glow": "music",
				"compact-glow": "music"
			},
			dock: {
				"bloom-dock": "dock",
				behavior: "dock",
				"dock-monitor": "dock",
				"peek-delay": "dock",
				"show-app-previews": "dock",
				"show-background-apps": "dock",
				"icon-only": "dock",
				"task-view": "dock",
				"app-separator": "dock",
				"win-number-shortcuts": "dock",
				"adaptive-mode": "dock",
				"start-menu-icon": "dock",
				drives: "dock",
				"recycle-bin": "dock",
				"dock-extras": "dock"
			},
			overlays: {
				"volume-hud": "volume",
				"brightness-hud": "brightness",
				"volume-edge-hover": "volume",
				"brightness-edge-hover": "brightness"
			}
		};
		const scene = next[tab]?.[label];
		if (scene) {
			setFeature(scene);
			setReplay((n) => n + 1);
		}
	}, [focus, tab]);

	useEffect(() => {
		if (previousWidgets.current !== settings.statusWidgets && tab === "notch") {
			setFeature("resting");
			setReplay((n) => n + 1);
		}
		previousWidgets.current = settings.statusWidgets;
	}, [settings.statusWidgets, tab]);

	if (!choices) return null;
	const selected = choices.find((choice) => choice.id === feature) ?? choices[0];

	return (
		<section
			className={`feature-preview${expanded ? "" : " collapsed"}`}
			aria-label={t("ui.featurePreview.featurePreview")}
		>
			<div className="feature-preview-heading">
				<h2>{t("ui.featurePreview.preview")}</h2>
				<div className="feature-preview-actions">
					{expanded && (
						<button
							type="button"
							onClick={() => setReplay((n) => n + 1)}
							aria-label={t("ui.featurePreview.replayPreview")}
							title={t("ui.featurePreview.replayPreview")}
						>
							<RotateCcw size={14} />
						</button>
					)}
					<button
						type="button"
						onClick={() => setExpanded((open) => !open)}
						aria-expanded={expanded}
						aria-label={
							expanded ? t("ui.featurePreview.hidePreview") : t("ui.featurePreview.showPreview")
						}
						title={
							expanded ? t("ui.featurePreview.hidePreview") : t("ui.featurePreview.showPreview")
						}
					>
						<ChevronDown size={16} />
					</button>
				</div>
			</div>
			{expanded && (
				<>
					{choices.length > 1 && (
						<div
							className="feature-preview-switcher"
							role="group"
							aria-label={t("ui.featurePreview.tabPreviews", { tab: tab })}
						>
							{choices.map((choice) => (
								<button
									key={choice.id}
									type="button"
									className={selected.id === choice.id ? "active" : ""}
									aria-pressed={selected.id === choice.id}
									onClick={() => {
										setFeature(choice.id);
										setReplay((n) => n + 1);
									}}
								>
									{choice.label}
								</button>
							))}
						</div>
					)}
					<div
						key={`${selected.id}-${replay}`}
						className={`feature-preview-stage real-${tab}`}
						onMouseEnter={() => setRevealed(true)}
						onMouseLeave={() => setRevealed(false)}
						onFocus={() => setRevealed(true)}
						onBlur={() => setRevealed(false)}
						tabIndex={0}
					>
						{tab === "appearance" && settings.cornersEnabled && (
							<>
								<span className="screen-corner top-left" />
								<span className="screen-corner top-right" />
							</>
						)}
						{(tab === "general" || tab === "notch" || tab === "appearance") && (
							<>
								<DeviceArtDefs />
								<NotchPreview
									feature={selected.id}
									settings={settings}
									runId={replay}
									revealed={revealed}
								/>
							</>
						)}
						{tab === "appearance" && <PreviewDock settings={settings} revealed={revealed} />}
						{tab === "dock" && <PreviewDock settings={settings} revealed={revealed} />}
						{tab === "overlays" && <OverlayPreview feature={selected.id} settings={settings} />}
					</div>
					<p className="feature-preview-description">
						{previewDescription(selected.id, settings, selected.description)}
					</p>
				</>
			)}
		</section>
	);
}
