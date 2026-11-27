// The menu button at the start of the title bar, on Windows and Linux. nsvim draws its own
// title bar, and there the platform then shows no menu bar. A press opens the application
// menu under the button: NsVim, File, Edit, View, Go and Help, each a submenu, the same items as
// macOS's menu bar. Rust owns the menu (`MainMenu` in `ext/nsvim-native/src/exports/main_menu.rs`).

import { View, type PressEvent } from "nsh";
import { Icon, MainMenu, Shell } from "nsvim-native";

/** The height of the title bar (`h-9`): the menu opens under it. */
const BAR = 36;

/** The menu button, or nothing where the platform shows the menu: macOS, the terminal and the
 * web. `size` is the interface's font size. */
export function MainMenuButton(props: { size: () => number }) {
  return (
    <If when={MainMenu.shown()}>
      <View
        class="shrink-0 self-stretch w-9 items-center justify-center cursor-default hover:bg-surface-raised/50"
        role="button"
        accessibilityLabel="Main menu"
        testID="main-menu"
        onPress={(e: PressEvent) => {
          // The menu's top left is the button's bottom left, in the window.
          MainMenu.open(e.pagePosition.x - e.position.x, e.pagePosition.y - e.position.y + BAR);
          Shell.focusKeys();
        }}
      >
        <Icon name="menu" size={props.size() + 3} class="text-muted-foreground" />
      </View>
    </If>
  );
}
