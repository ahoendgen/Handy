import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { Select, type SelectOption } from "../ui/Select";
import type { TriggerWord, TriggerActionType } from "@/bindings";
import { commands } from "@/bindings";

interface TriggerWordsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

const ACTION_TYPE_OPTIONS: SelectOption[] = [
  { value: "key_press", label: "Key Press" },
  { value: "text_replacement", label: "Text Replacement" },
];

const KEY_OPTIONS: SelectOption[] = [
  { value: "enter", label: "Enter" },
  { value: "tab", label: "Tab" },
  { value: "backspace", label: "Backspace" },
  { value: "escape", label: "Escape" },
  { value: "space", label: "Space" },
  { value: "delete", label: "Delete" },
];

export const TriggerWords: React.FC<TriggerWordsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, isUpdating, refreshSettings } = useSettings();

    // Add new trigger state
    const [newPhrase, setNewPhrase] = useState("");
    const [actionType, setActionType] = useState<TriggerActionType>("key_press");
    const [actionValue, setActionValue] = useState("enter");
    const [isAdding, setIsAdding] = useState(false);

    // Edit trigger state
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editPhrase, setEditPhrase] = useState("");
    const [editActionType, setEditActionType] = useState<TriggerActionType>("key_press");
    const [editActionValue, setEditActionValue] = useState("");
    const [isSaving, setIsSaving] = useState(false);

    const triggerWordsEnabled = getSetting("trigger_words_enabled") ?? false;
    const triggerWords = getSetting("trigger_words") || [];

    const handleToggle = async (enabled: boolean) => {
      try {
        await commands.changeTriggerWordsEnabledSetting(enabled);
        await refreshSettings();
      } catch (error) {
        console.error("Failed to toggle trigger words:", error);
      }
    };

    const handleAddTrigger = async () => {
      const trimmedPhrase = newPhrase.trim().toLowerCase();
      if (!trimmedPhrase || trimmedPhrase.length > 50) return;

      if (
        triggerWords.some(
          (t: TriggerWord) =>
            t.trigger_phrase.toLowerCase() === trimmedPhrase
        )
      ) {
        return;
      }

      setIsAdding(true);
      try {
        const result = await commands.addTriggerWord(
          trimmedPhrase,
          actionType,
          actionValue
        );
        if (result.status === "ok") {
          setNewPhrase("");
          setActionType("key_press");
          setActionValue("enter");
          await refreshSettings();
        }
      } catch (error) {
        console.error("Failed to add trigger word:", error);
      } finally {
        setIsAdding(false);
      }
    };

    const handleToggleTrigger = async (trigger: TriggerWord) => {
      try {
        const result = await commands.updateTriggerWord(
          trigger.id,
          trigger.trigger_phrase,
          trigger.action_type,
          trigger.action_value,
          !trigger.enabled
        );
        if (result.status === "ok") {
          await refreshSettings();
        }
      } catch (error) {
        console.error("Failed to toggle trigger word:", error);
      }
    };

    const startEditing = (trigger: TriggerWord) => {
      setEditingId(trigger.id);
      setEditPhrase(trigger.trigger_phrase);
      setEditActionType(trigger.action_type);
      setEditActionValue(trigger.action_value);
    };

    const cancelEditing = () => {
      setEditingId(null);
      setEditPhrase("");
      setEditActionType("key_press");
      setEditActionValue("");
    };

    const saveEdit = async (trigger: TriggerWord) => {
      const trimmedPhrase = editPhrase.trim().toLowerCase();
      if (!trimmedPhrase) return;

      setIsSaving(true);
      try {
        const result = await commands.updateTriggerWord(
          trigger.id,
          trimmedPhrase,
          editActionType,
          editActionValue,
          trigger.enabled
        );
        if (result.status === "ok") {
          await refreshSettings();
          cancelEditing();
        }
      } catch (error) {
        console.error("Failed to update trigger word:", error);
      } finally {
        setIsSaving(false);
      }
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddTrigger();
      }
    };

    const getActionLabel = (trigger: TriggerWord) => {
      if (trigger.action_type === "key_press") {
        const key = KEY_OPTIONS.find((k) => k.value === trigger.action_value);
        return key?.label || trigger.action_value;
      }
      // Show special characters in a readable way
      if (trigger.action_value === "\n") return "↵ (newline)";
      if (trigger.action_value === "\t") return "⇥ (tab)";
      return `"${trigger.action_value}"`;
    };

    return (
      <>
        <ToggleSwitch
          checked={triggerWordsEnabled}
          onChange={handleToggle}
          isUpdating={isUpdating("trigger_words_enabled")}
          label={t("settings.advanced.triggerWords.title")}
          description={t("settings.advanced.triggerWords.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />

        {triggerWordsEnabled && (
          <>
            <SettingContainer
              title={t("settings.advanced.triggerWords.addNew")}
              description=""
              descriptionMode={descriptionMode}
              grouped={grouped}
            >
              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2">
                  <Input
                    type="text"
                    className="flex-1 max-w-40"
                    value={newPhrase}
                    onChange={(e) => setNewPhrase(e.target.value)}
                    onKeyDown={handleKeyPress}
                    placeholder={t(
                      "settings.advanced.triggerWords.phrasePlaceholder"
                    )}
                    variant="compact"
                    disabled={isAdding}
                  />
                  <Select
                    value={actionType}
                    options={ACTION_TYPE_OPTIONS}
                    onChange={(value) =>
                      setActionType((value as TriggerActionType) || "key_press")
                    }
                    placeholder={t("settings.advanced.triggerWords.actionType")}
                    disabled={isAdding}
                    isClearable={false}
                    className="w-40"
                  />
                </div>
                <div className="flex items-center gap-2">
                  {actionType === "key_press" ? (
                    <Select
                      value={actionValue}
                      options={KEY_OPTIONS}
                      onChange={(value) => setActionValue(value || "enter")}
                      placeholder={t("settings.advanced.triggerWords.selectKey")}
                      disabled={isAdding}
                      isClearable={false}
                      className="w-40"
                    />
                  ) : (
                    <Input
                      type="text"
                      className="max-w-40"
                      value={actionValue}
                      onChange={(e) => setActionValue(e.target.value)}
                      placeholder={t(
                        "settings.advanced.triggerWords.textPlaceholder"
                      )}
                      variant="compact"
                      disabled={isAdding}
                    />
                  )}
                  <Button
                    onClick={handleAddTrigger}
                    disabled={
                      !newPhrase.trim() ||
                      newPhrase.trim().length > 50 ||
                      isAdding
                    }
                    variant="primary"
                    size="md"
                  >
                    {t("settings.advanced.triggerWords.add")}
                  </Button>
                </div>
              </div>
            </SettingContainer>

            {triggerWords.length > 0 && (
              <div
                className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"}`}
              >
                <div className="space-y-2">
                  {triggerWords.map((trigger: TriggerWord) => (
                    <div key={trigger.id}>
                      {editingId === trigger.id ? (
                        // Edit mode
                        <div className="flex flex-col gap-2 py-2 px-2 rounded bg-mid-gray/10">
                          <div className="flex items-center gap-2">
                            <Input
                              type="text"
                              className="flex-1 max-w-32"
                              value={editPhrase}
                              onChange={(e) => setEditPhrase(e.target.value)}
                              placeholder="Phrase"
                              variant="compact"
                              disabled={isSaving}
                            />
                            <Select
                              value={editActionType}
                              options={ACTION_TYPE_OPTIONS}
                              onChange={(value) =>
                                setEditActionType(
                                  (value as TriggerActionType) || "key_press"
                                )
                              }
                              disabled={isSaving}
                              isClearable={false}
                              className="w-36"
                            />
                          </div>
                          <div className="flex items-center gap-2">
                            {editActionType === "key_press" ? (
                              <Select
                                value={editActionValue}
                                options={KEY_OPTIONS}
                                onChange={(value) =>
                                  setEditActionValue(value || "enter")
                                }
                                disabled={isSaving}
                                isClearable={false}
                                className="w-32"
                              />
                            ) : (
                              <Input
                                type="text"
                                className="max-w-32"
                                value={editActionValue}
                                onChange={(e) => setEditActionValue(e.target.value)}
                                placeholder="Text"
                                variant="compact"
                                disabled={isSaving}
                              />
                            )}
                            <Button
                              onClick={() => saveEdit(trigger)}
                              disabled={!editPhrase.trim() || isSaving}
                              variant="primary"
                              size="sm"
                            >
                              {t("common.save")}
                            </Button>
                            <Button
                              onClick={cancelEditing}
                              disabled={isSaving}
                              variant="secondary"
                              size="sm"
                            >
                              {t("common.cancel")}
                            </Button>
                          </div>
                        </div>
                      ) : (
                        // View mode
                        <div
                          className={`flex items-center justify-between py-1 px-2 rounded hover:bg-mid-gray/5 ${
                            trigger.enabled ? "" : "opacity-50"
                          }`}
                        >
                          <div className="flex items-center gap-2">
                            <button
                              onClick={() => handleToggleTrigger(trigger)}
                              className={`w-4 h-4 rounded border flex-shrink-0 ${
                                trigger.enabled
                                  ? "bg-logo-primary border-logo-primary"
                                  : "border-mid-gray/50"
                              }`}
                              aria-label={
                                trigger.enabled
                                  ? "Disable trigger"
                                  : "Enable trigger"
                              }
                            >
                              {trigger.enabled && (
                                <svg
                                  className="w-full h-full text-white"
                                  fill="none"
                                  stroke="currentColor"
                                  viewBox="0 0 24 24"
                                >
                                  <path
                                    strokeLinecap="round"
                                    strokeLinejoin="round"
                                    strokeWidth={3}
                                    d="M5 13l4 4L19 7"
                                  />
                                </svg>
                              )}
                            </button>
                            <span className="font-medium">
                              "{trigger.trigger_phrase}"
                            </span>
                            <span className="text-mid-gray">→</span>
                            <span
                              className={`text-sm ${
                                trigger.action_type === "key_press"
                                  ? "text-logo-primary"
                                  : "text-green-500"
                              }`}
                            >
                              {getActionLabel(trigger)}
                            </span>
                            {trigger.is_builtin && (
                              <span className="text-xs text-mid-gray/60 ml-1">
                                ({t("settings.advanced.triggerWords.builtin")})
                              </span>
                            )}
                          </div>
                          <Button
                            onClick={() => startEditing(trigger)}
                            variant="ghost"
                            size="sm"
                            className="opacity-50 hover:opacity-100"
                            aria-label={t("common.edit")}
                          >
                            <svg
                              className="w-4 h-4"
                              fill="none"
                              stroke="currentColor"
                              viewBox="0 0 24 24"
                            >
                              <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                              />
                            </svg>
                          </Button>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </>
    );
  }
);
