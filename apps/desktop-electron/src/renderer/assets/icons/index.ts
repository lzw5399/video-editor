import deleteIconUrl from "./delete.svg";
import pauseIconUrl from "./pause.svg";
import playIconUrl from "./play.svg";
import redoIconUrl from "./redo.svg";
import splitIconUrl from "./split.svg";
import undoIconUrl from "./undo.svg";
import zoomInIconUrl from "./zoom-in.svg";
import zoomOutIconUrl from "./zoom-out.svg";

export const appIconUrls = {
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
