import categoryAdjustIconUrl from "./category-adjust.svg";
import categoryAudioIconUrl from "./category-audio.svg";
import categoryCaptionIconUrl from "./category-caption.svg";
import categoryDigitalHumanIconUrl from "./category-digital-human.svg";
import categoryEffectIconUrl from "./category-effect.svg";
import categoryFilterIconUrl from "./category-filter.svg";
import categoryMediaIconUrl from "./category-media.svg";
import categoryStickerIconUrl from "./category-sticker.svg";
import categoryTemplateIconUrl from "./category-template.svg";
import categoryTextIconUrl from "./category-text.svg";
import categoryTransitionIconUrl from "./category-transition.svg";
import deleteIconUrl from "./delete.svg";
import mediaFilterIconUrl from "./media-filter.svg";
import mediaImportIconUrl from "./media-import.svg";
import mediaListIconUrl from "./media-list.svg";
import pauseIconUrl from "./pause.svg";
import playIconUrl from "./play.svg";
import previewFitIconUrl from "./preview-fit.svg";
import previewNextFrameIconUrl from "./preview-next-frame.svg";
import previewPreviousFrameIconUrl from "./preview-previous-frame.svg";
import previewStopIconUrl from "./preview-stop.svg";
import redoIconUrl from "./redo.svg";
import splitIconUrl from "./split.svg";
import titlebarMenuIconUrl from "./titlebar-menu.svg";
import timelineAddIconUrl from "./timeline-add.svg";
import timelineSnapOffIconUrl from "./timeline-snap-off.svg";
import timelineSnapOnIconUrl from "./timeline-snap-on.svg";
import topExportIconUrl from "./top-export.svg";
import trackHideOffIconUrl from "./track-hide-off.svg";
import trackHideOnIconUrl from "./track-hide-on.svg";
import trackLockOffIconUrl from "./track-lock-off.svg";
import trackLockOnIconUrl from "./track-lock-on.svg";
import trackMuteOffIconUrl from "./track-mute-off.svg";
import trackMuteOnIconUrl from "./track-mute-on.svg";
import undoIconUrl from "./undo.svg";
import zoomInIconUrl from "./zoom-in.svg";
import zoomOutIconUrl from "./zoom-out.svg";

export const appIconUrls = {
  categoryAdjust: categoryAdjustIconUrl,
  categoryAudio: categoryAudioIconUrl,
  categoryCaption: categoryCaptionIconUrl,
  categoryDigitalHuman: categoryDigitalHumanIconUrl,
  categoryEffect: categoryEffectIconUrl,
  categoryFilter: categoryFilterIconUrl,
  categoryMedia: categoryMediaIconUrl,
  categorySticker: categoryStickerIconUrl,
  categoryTemplate: categoryTemplateIconUrl,
  categoryText: categoryTextIconUrl,
  categoryTransition: categoryTransitionIconUrl,
  delete: deleteIconUrl,
  mediaFilter: mediaFilterIconUrl,
  mediaImport: mediaImportIconUrl,
  mediaList: mediaListIconUrl,
  pause: pauseIconUrl,
  play: playIconUrl,
  previewFit: previewFitIconUrl,
  previewNextFrame: previewNextFrameIconUrl,
  previewPreviousFrame: previewPreviousFrameIconUrl,
  previewStop: previewStopIconUrl,
  redo: redoIconUrl,
  split: splitIconUrl,
  titlebarMenu: titlebarMenuIconUrl,
  timelineAdd: timelineAddIconUrl,
  timelineSnapOff: timelineSnapOffIconUrl,
  timelineSnapOn: timelineSnapOnIconUrl,
  topExport: topExportIconUrl,
  trackHideOff: trackHideOffIconUrl,
  trackHideOn: trackHideOnIconUrl,
  trackLockOff: trackLockOffIconUrl,
  trackLockOn: trackLockOnIconUrl,
  trackMuteOff: trackMuteOffIconUrl,
  trackMuteOn: trackMuteOnIconUrl,
  undo: undoIconUrl,
  zoomIn: zoomInIconUrl,
  zoomOut: zoomOutIconUrl
} as const;

export type AppIconName = keyof typeof appIconUrls;
