import { useState } from "react";
import { Button } from "@/ui/Button";
import {
	Select,
	SelectTrigger,
	SelectValue,
	SelectContent,
	SelectItem,
} from "@/ui/Select";
import type { ConversationSettings, ConversationDefaultsResponse } from "@/api/types";

interface ConversationSettingsPanelProps {
	defaults: ConversationDefaultsResponse;
	currentSettings: ConversationSettings;
	onChange: (settings: ConversationSettings) => void;
	onSave: () => void;
}

const PRESETS: Array<{ id: string; name: string; settings: ConversationSettings }> = [
	{ id: "chat", name: "Chat", settings: { memory: "full", delegation: "standard", worker_context: { history: "none", memory: "none" } } },
	{ id: "focus", name: "Focus", settings: { memory: "ambient", delegation: "standard", worker_context: { history: "none", memory: "none" } } },
	{ id: "hands-on", name: "Hands-on", settings: { memory: "off", delegation: "direct", worker_context: { history: "recent", memory: "tools" } } },
	{ id: "quick", name: "Quick", settings: { memory: "off", delegation: "standard", worker_context: { history: "none", memory: "none" } } },
];

export function ConversationSettingsPanel({ defaults, currentSettings, onChange, onSave }: ConversationSettingsPanelProps) {
	const [isExpanded, setIsExpanded] = useState(false);

	const applyPreset = (preset: (typeof PRESETS)[0]) => {
		onChange({ ...currentSettings, ...preset.settings });
	};

	return (
		<div className="rounded-lg border border-app-line bg-app-box p-4">
			<div className="mb-4 flex items-center justify-between">
				<h3 className="text-sm font-medium">Conversation Settings</h3>
				<Button variant="ghost" size="sm" onClick={() => setIsExpanded(!isExpanded)}>
					{isExpanded ? "▼" : "▶"} Advanced
				</Button>
			</div>

			{/* Presets */}
			<div className="mb-4 flex flex-wrap gap-2">
				{PRESETS.map((preset) => (
					<Button
						key={preset.id}
						variant="outline"
						size="sm"
						onClick={() => applyPreset(preset)}
						className="text-xs"
					>
						{preset.name}
					</Button>
				))}
			</div>

			{/* Basic Settings */}
			<div className="space-y-3">
				<div>
					<label className="mb-1 block text-xs text-ink-faint">Model</label>
					<Select
						value={currentSettings.model || defaults.model}
						onValueChange={(value) => onChange({ ...currentSettings, model: value })}
					>
						<SelectTrigger>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{defaults.available_models.map((m) => (
								<SelectItem key={m.id} value={m.id}>{m.name}</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>

				<div>
					<label className="mb-1 block text-xs text-ink-faint">Memory</label>
					<Select
						value={currentSettings.memory || defaults.memory}
						onValueChange={(value) => onChange({ ...currentSettings, memory: value as any })}
					>
						<SelectTrigger>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="full">On</SelectItem>
							<SelectItem value="ambient">Context Only</SelectItem>
							<SelectItem value="off">Off</SelectItem>
						</SelectContent>
					</Select>
				</div>

				<div>
					<label className="mb-1 block text-xs text-ink-faint">Mode</label>
					<Select
						value={currentSettings.delegation || defaults.delegation}
						onValueChange={(value) => onChange({ ...currentSettings, delegation: value as any })}
					>
						<SelectTrigger>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="standard">Standard</SelectItem>
							<SelectItem value="direct">Direct (cortex chat)</SelectItem>
						</SelectContent>
					</Select>
				</div>
			</div>

			{/* Advanced Settings */}
			{isExpanded && (
				<div className="mt-4 space-y-3 border-t border-app-line pt-4">
					<h4 className="text-xs font-medium text-ink-faint">Worker Context</h4>
					
					<div>
						<label className="mb-1 block text-xs text-ink-faint">History</label>
						<Select
							value={currentSettings.worker_context?.history || defaults.worker_context.history}
							onValueChange={(value) => onChange({
								...currentSettings,
								worker_context: { ...currentSettings.worker_context, history: value as any }
							})}
						>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="none">None</SelectItem>
								<SelectItem value="summary">Summary</SelectItem>
								<SelectItem value="recent">Recent (20)</SelectItem>
								<SelectItem value="full">Full</SelectItem>
							</SelectContent>
						</Select>
					</div>

					<div>
						<label className="mb-1 block text-xs text-ink-faint">Worker Memory</label>
						<Select
							value={currentSettings.worker_context?.memory || defaults.worker_context.memory}
							onValueChange={(value) => onChange({
								...currentSettings,
								worker_context: { ...currentSettings.worker_context, memory: value as any }
							})}
						>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="none">None</SelectItem>
								<SelectItem value="ambient">Ambient</SelectItem>
								<SelectItem value="tools">Tools</SelectItem>
								<SelectItem value="full">Full</SelectItem>
							</SelectContent>
						</Select>
					</div>
				</div>
			)}

			<div className="mt-4 flex justify-end">
				<Button onClick={onSave} variant="default">Apply Settings</Button>
			</div>
		</div>
	);
}
