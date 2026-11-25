// The title bar that the app draws. Rust hides the system's title bar (`window_attributes`), so
// the top bar of each screen is the title bar: it is `h-9` (`TITLE_BAR` in Rust), a press on it
// drags the window, and it keeps the system's window controls clear. Views in it with a press
// handler stay clickable.

import { View } from "nsh";
import { titleBar } from "nsh/desktop";

/** The props that make a view drag the window, as a title bar does. */
export const DRAG = { windowDrag: true } as const;

/** The space that the system's window controls take at one end of the title bar: macOS's
 * traffic lights at the left, the buttons of Windows and GTK at the right. It is 0 wide where
 * the system draws no controls in the bar, as on the web. */
export function Controls(props: { side: "left" | "right" }) {
  const width = () => titleBar()[props.side];
  return (
    <If when={width() > 0}>
      <View class="shrink-0 self-stretch" style={{ width: width() }} />
    </If>
  );
}

/** The width of the window controls at the left end of the title bar. */
export function controlsLeft(): number {
  return titleBar().left;
}
