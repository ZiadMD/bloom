import type { EventCallback, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

/**
 * Listens for an event addressed to this window (or broadcast to every
 * window). `listen` from `@tauri-apps/api/event` hears events sent to *any*
 * window, so with a notch or dock on several monitors one screen would react
 * to another's overlap, edge hover or fullscreen state. Per-surface events
 * (see monitors.rs) are sent to the window they concern; listen for them here.
 */
export function listenHere<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
	return getCurrentWebviewWindow().listen<T>(event, handler);
}
