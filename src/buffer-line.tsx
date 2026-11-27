// The title bar of the editor has no row of its own: the tabs of the open files are the top of
// the window. Over the tree, the top shows the menu button and the project folder. With no
// tree, the window controls and the menu button sit at the start of the tab bar (`Corner`). The
// empty end of the top tab bar drags the window and keeps the controls at the right clear
// (`BarEnd`). The menu button (`./main-menu.tsx`) shows only on Windows and Linux. The tabs are
// in the panes (`./panes.tsx`). Rust owns the state (`Tabs` in
// `ext/nsvim-native/src/exports/buffer_line.rs`).

import { signal, View } from "nsh";
import { MainMenu, Tabs, type BufferLineState } from "nsvim-native";
import { MainMenuButton } from "./main-menu";
import { Label, type Font } from "./panes";
import { Controls, controlsLeft, DRAG } from "./title-bar";

/** The top of the tree: the menu button and the project folder, as wide as the tree. The path
 * keeps its end, which tells folders apart. Nothing while the tree does not show. */
export function ProjectBar(props: { font: () => Font }) {
  const project = signal<BufferLineState["project"]>(undefined);
  // Rust posts the state only when it changed. The project is set only when it changed.
  Tabs.onChange((s) => {
    const p = project();
    if (!s.project || !p || p.path !== s.project.path || p.width !== s.project.width) project.set(s.project);
  });
  return (
    <If when={project()}>
      {(p) => (
        <View
          class="flex-row shrink-0 h-9 items-center pr-3 overflow-hidden bg-muted border-r border-b border-border"
          style={{ width: p().width, paddingLeft: controlsLeft() > 0 ? controlsLeft() + 4 : MainMenu.shown() ? 0 : 12 }}
          accessibilityLabel={p().path}
          testID="project"
          desktop={DRAG}
        >
          <MainMenuButton size={() => props.font().size} />
          <Label font={props.font} text={p().path} class="flex-1 min-w-0" cutStart />
        </View>
      )}
    </If>
  );
}

/** The start of the top bar while the tree does not show: the space of the window controls at
 * the left and the menu button. It lies over the start of the tab bar, and `onWidth` gives its
 * width, which the tab bar keeps clear. */
export function Corner(props: { font: () => Font; onWidth: (width: number) => void }) {
  return (
    <View
      class="absolute top-0 left-0 h-9 flex-row items-center"
      testID="corner"
      desktop={DRAG}
      onLayout={(e) => props.onWidth(e.bounds.size.width)}
    >
      <Controls side="left" />
      <MainMenuButton size={() => props.font().size} />
    </View>
  );
}

/** The empty end of a tab bar. It drags the window. The bar at the top right also keeps the
 * window controls at the right clear (`controls`). */
export function BarEnd(props: { controls: boolean }) {
  return (
    <View class="flex-1 self-stretch flex-row" desktop={DRAG} testID="bar-end">
      <View class="flex-1" />
      <If when={props.controls}>
        <Controls side="right" />
      </If>
    </View>
  );
}

/** The top bar while no file is open: there are no tabs. */
export function EmptyBar(props: { lead: number }) {
  return (
    <View class="flex-row shrink-0 h-9 bg-muted border-b border-border" style={{ paddingLeft: props.lead }} testID="empty-bar">
      <BarEnd controls />
    </View>
  );
}
