// The title bar of the editor has no row of its own: the tabs of the open files are the top of
// the window. Over the tree, the top shows the menu button and the project folder. With no
// tree, the window controls and the menu button sit at the start of the tab bar (`Corner`). The
// tab bar at the top right keeps the controls at the right clear (`BarEnd`). The empty part of
// the tab bars drags the window (`./panes.tsx`). The menu button (`./main-menu.tsx`) shows only
// on Windows and Linux. Rust owns the state (`Tabs` in `ext/nsvim-native/src/exports/buffer_line.rs`).

import { signal, View } from "nsh";
import { MainMenu, Tabs, type BufferLineState } from "nsvim-native";
import { MainMenuBar, MainMenuButton, menusShown } from "./main-menu";
import { Label, type Font } from "./panes";
import { Controls, controlsLeft, DRAG } from "./title-bar";

/** The top of the tree: the menu button and the project folder, as wide as the tree. The path
 * keeps its end, which tells folders apart. The row of menus takes the place of the path while
 * it shows. Nothing while the tree does not show. */
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
          <If when={menusShown()} else={<Label font={props.font} text={p().path} class="flex-1 min-w-0" cutStart />}>
            <MainMenuBar font={props.font} />
          </If>
        </View>
      )}
    </If>
  );
}

/** The start of the top bar while the tree does not show: the space of the window controls at
 * the left and the menu button, with the row of menus while it shows. It lies over the start of the tab bar, and `onWidth` gives its
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
      <If when={menusShown()}>
        <MainMenuBar font={props.font} />
      </If>
    </View>
  );
}

/** The end of the tab bar at the top right: the space of the window controls at the right. */
export function BarEnd() {
  return <Controls side="right" />;
}

/** The top bar while no file is open: there are no tabs. */
export function EmptyBar(props: { lead: number }) {
  return (
    <View class="flex-row shrink-0 h-9 bg-muted border-b border-border" style={{ paddingLeft: props.lead }} testID="empty-bar">
      <View class="flex-1" />
      <BarEnd />
    </View>
  );
}
