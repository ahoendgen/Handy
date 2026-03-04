import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { VolumeSlider } from "./VolumeSlider";
import { SoundPicker } from "./SoundPicker";

interface AudioFeedbackProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const AudioFeedback: React.FC<AudioFeedbackProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    return (
      <div className="flex flex-col">
        <ToggleSwitch
          checked={getSetting("audio_feedback_start") || false}
          onChange={(enabled) => updateSetting("audio_feedback_start", enabled)}
          isUpdating={isUpdating("audio_feedback_start")}
          label={t("settings.sound.audioFeedback.startSound.label")}
          description={t(
            "settings.sound.audioFeedback.startSound.description",
          )}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
        <ToggleSwitch
          checked={getSetting("audio_feedback_stop") || false}
          onChange={(enabled) => updateSetting("audio_feedback_stop", enabled)}
          isUpdating={isUpdating("audio_feedback_stop")}
          label={t("settings.sound.audioFeedback.stopSound.label")}
          description={t("settings.sound.audioFeedback.stopSound.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
      </div>
    );
  },
);
