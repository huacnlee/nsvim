// The menu button at the start of the title bar, on Windows and Linux. nsvim draws its own
// title bar, and there the platform then shows no menu bar. A press on the button shows the menus
// in a row next to it: NsVim, File, Edit, View, Go and Help, the same menus as macOS's menu bar.
// A press on a menu opens its items under it. A picked item, or a second press on the button,
// hides the row. Rust owns the menu (`MainMenu` in `ext/nsvim-native/src/exports/main_menu.rs`).

import { signal, View, type PressEvent } from "nsh";
import { Icon, MainMenu, Shell } from "nsvim-native";
import { Label, type Font } from "./panes";

/** The height of the title bar (`h-9`): a menu opens under it. */
const BAR = 36;

/** Whether the row of menus shows. */
export const menusShown = signal(false);

/** The menu button, or nothing where the platform shows the menu: macOS, the terminal and the
 * web. `size` is the interface's font size. */
export function MainMenuButton(props: { size: () => number }) {
  return (
    <If when={MainMenu.shown()}>
      <View
        class="shrink-0 self-stretch w-9 items-center justify-center cursor-default hover:bg-surface-raised/50"
        classList={{ "bg-surface-raised/50": menusShown() }}
        role="button"
        accessibilityLabel="Main menu"
        accessibilityState={{ expanded: menusShown() }}
        testID="main-menu"
        onPress={() => {
          menusShown.set(!menusShown());
          Shell.focusKeys();
        }}
      >
        <Icon name="menu" size={props.size() + 3} class="text-muted-foreground" />
      </View>
    </If>
  );
}

/** The row of menus, while the menu button shows it. */
export function MainMenuBar(props: { font: () => Font }) {
  const open = signal(-1);
  return (
    <View class="flex-row shrink-0 self-stretch" role="menubar" testID="main-menu-bar">
      {MainMenu.menus().map((name, i) => (
          <View
            class="shrink-0 self-stretch justify-center px-2 cursor-default hover:bg-surface-raised/50"
            classList={{ "bg-surface-raised/50": open() === i }}
            role="button"
            accessibilityLabel={name}
            accessibilityState={{ expanded: open() === i }}
            testID={`menu-${name}`}
            onPress={(e: PressEvent) => {
              open.set(i);
              // The menu's top left is the name's bottom left, in the window.
              MainMenu.openMenu(i, e.pagePosition.x - e.position.x, e.pagePosition.y - e.position.y + BAR).then((picked) => {
                open.set(-1);
                if (picked) menusShown.set(false);
              });
              Shell.focusKeys();
            }}
          >
            <Label font={props.font} on={open() === i} text={name} />
          </View>
      ))}
    </View>
  );
}
