import { Volume2, Sun, ArrowLeftToLine, ArrowRightToLine } from "lucide-react";
import { SettingRow } from "./SettingRow";
import { t, useTranslation } from "../i18n";

interface OverlaysTabProps {
	volumeOverlayEnabled: boolean;
	toggleVolumeOverlay: () => void;
	volumeEdgeEnabled: boolean;
	toggleVolumeEdge: () => void;
	brightnessOverlayEnabled: boolean;
	toggleBrightnessOverlay: () => void;
	brightnessEdgeEnabled: boolean;
	toggleBrightnessEdge: () => void;
}

export function OverlaysTab({
	volumeOverlayEnabled,
	toggleVolumeOverlay,
	volumeEdgeEnabled,
	toggleVolumeEdge,
	brightnessOverlayEnabled,
	toggleBrightnessOverlay,
	brightnessEdgeEnabled,
	toggleBrightnessEdge
}: OverlaysTabProps) {
	useTranslation();

	return (
		<>
			<div className="setting-group-label">{t("settings.groups.overlays")}</div>
			<div className="setting-group">
				<SettingRow
					previewId="volume-hud"
					icon={Volume2}
					label={t("settings.overlays.volume")}
					desc={t("settings.overlays.volumeDesc")}
				>
					<label className="toggle-switch">
						<input type="checkbox" checked={volumeOverlayEnabled} onChange={toggleVolumeOverlay} />
						<span className="slider"></span>
					</label>
				</SettingRow>

				{volumeOverlayEnabled && (
					<SettingRow
						previewId="volume-edge-hover"
						icon={ArrowLeftToLine}
						label={t("settings.overlays.volumeEdge")}
						desc={t("settings.overlays.volumeEdgeDesc")}
					>
						<label className="toggle-switch">
							<input type="checkbox" checked={volumeEdgeEnabled} onChange={toggleVolumeEdge} />
							<span className="slider"></span>
						</label>
					</SettingRow>
				)}

				<SettingRow
					previewId="brightness-hud"
					icon={Sun}
					label={t("settings.overlays.brightness")}
					desc={t("settings.overlays.brightnessDesc")}
				>
					<label className="toggle-switch">
						<input
							type="checkbox"
							checked={brightnessOverlayEnabled}
							onChange={toggleBrightnessOverlay}
						/>
						<span className="slider"></span>
					</label>
				</SettingRow>

				{brightnessOverlayEnabled && (
					<SettingRow
						previewId="brightness-edge-hover"
						icon={ArrowRightToLine}
						label={t("settings.overlays.brightnessEdge")}
						desc={t("settings.overlays.brightnessEdgeDesc")}
						divider={false}
					>
						<label className="toggle-switch">
							<input
								type="checkbox"
								checked={brightnessEdgeEnabled}
								onChange={toggleBrightnessEdge}
							/>
							<span className="slider"></span>
						</label>
					</SettingRow>
				)}
			</div>
		</>
	);
}
