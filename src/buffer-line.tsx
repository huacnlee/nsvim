// The title bar of the editor: the menu button and the project folder over the tree, then the name
// of the file that has the keys. A press on it drags the window, and it keeps the system's window
// controls clear (`./title-bar.tsx`). The menu button (`./main-menu.tsx`) shows only on Windows and
// Linux. The tabs of the open files are in the panes below it (`./panes.tsx`).
// Rust owns the state (`Tabs` in `ext/nsvim-native/src/exports/buffer_line.rs`).

import { signal, View } from "nsh";
import { MainMenu, Tabs, type BufferLineState } from "nsvim-native";
import { MainMenuBar, MainMenuButton, menusShown } from "./main-menu";
import { Label, type Font } from "./panes";
import { Controls, controlsLeft, DRAG } from "./title-bar";

/** The window's title bar, over the tree and the panes. */
export function BufferLine(props: { font: () => Font }) {
  const project = signal<BufferLineState["project"]>(undefined);
  const title = signal("");
  // Rust posts the state only when it changed. The project and the title are set only when
  // their own part changed.
  Tabs.onChange((s) => {
    const p = project();
    if (!s.project || !p || p.path !== s.project.path || p.width !== s.project.width) project.set(s.project);
    const name = s.tabs.find((t) => t.on)?.name ?? "";
    if (name !== title()) title.set(name);
  });
  return (
    <View class="flex-row items-center h-9 shrink-0 overflow-hidden bg-muted border-b border-border" testID="buffer-line" desktop={DRAG}>
      <If
        when={project()}
        else={
          <>
            <Controls side="left" />
            <MainMenuButton size={() => props.font().size} />
            <If when={menusShown()}>
              <MainMenuBar font={props.font} />
            </If>
          </>
        }
      >
        {(p) => (
          // As wide as the tree, with the window controls or the menu button at its start. The
          // path keeps its end, which tells folders apart. The row of menus takes the place of
          // the path while it shows, and the folder grows when the tree is too narrow for it.
          <View
            class="flex-row shrink-0 self-stretch items-center pr-3 overflow-hidden border-r border-border"
            style={{ width: menusShown() ? undefined : p().width, minWidth: p().width, paddingLeft: controlsLeft() > 0 ? controlsLeft() + 4 : MainMenu.shown() ? 0 : 12 }}
            accessibilityLabel={p().path}
            testID="project"
          >
            <MainMenuButton size={() => props.font().size} />
            <If when={menusShown()} else={<Label font={props.font} text={p().path} class="flex-1 min-w-0" cutStart />}>
              <MainMenuBar font={props.font} />
            </If>
          </View>
        )}
      </If>
      <View class="flex-1 min-w-0 self-stretch flex-row items-center justify-center px-3" desktop={DRAG}>
        <Label font={props.font} text={title()} />
      </View>
      <Controls side="right" />
    </View>
  );
}
