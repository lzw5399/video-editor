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
import pauseIconUrl from "./pause.svg";
import playIconUrl from "./play.svg";
import redoIconUrl from "./redo.svg";
import splitIconUrl from "./split.svg";
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
  pause: pauseIconUrl,
  play: playIconUrl,
  redo: redoIconUrl,
  split: splitIconUrl,
  undo: undoIconUrl,
  zoomIn: zoomInIconUrl,
  zoomOut: zoomOutIconUrl
} as const;

export type AppIconName = keyof typeof appIconUrls;
