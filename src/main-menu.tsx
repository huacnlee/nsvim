// The menu button at the start of the title bar, on Windows and Linux. nsvim draws its own
// title bar, and there the platform then shows no menu bar. A press on the button shows the menus
// in a row next to it: NsVim, File, Edit, View, Go and Help, the same menus as macOS's menu bar.
// The row is a menu bar (`nsh/base`'s `Menu`s in a `Toolbar`): a press on a name opens its menu
// under it, and while one is open the pointer over another name opens that one. A submenu opens
// beside its item. The row stays until a second press on the button hides it; an item that turns
// something on or off keeps its menu open, so the check mark changes in place. Rust owns the menu
// (`MainMenu` in `ext/nsvim-native/src/exports/main_menu.rs`).

import { signal, Text, View } from "nsh";
import { Menu, MenuItem, MenuSeparator, Submenu, Toolbar } from "nsh/base";
import { Icon, MainMenu, Shell, type MainMenuItem, type MainMenuList, type MainMenuSubItem } from "nsvim-native";
import { Label, type Font } from "./panes";

/** Whether the row of menus shows. */
export const menusShown = signal(false);

/** The index of the open menu of the row, or -1. It is kept outside the row: the row moves
 * between the top bar and the folder over the tree when the tree shows or hides (View › File
 * Tree), and the menu stays open there. */
const openMenu = signal(-1);

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
          openMenu.set(-1);
          Shell.focusKeys();
        }}
      >
        <Icon name="menu" size={props.size() + 3} class="text-muted-foreground" />
      </View>
    </If>
  );
}

/** The popup of a menu and of a submenu, as the in-app context menu (`./overlays/ContextMenu.tsx`). */
const POPUP = "min-w-56 p-[4px] rounded-(--radius) bg-muted border border-border shadow-lg";
/** A row of a menu. The keys and the pointer give it the focus. */
const ROW = "flex-row h-[24px] px-[8px] gap-[6px] rounded-(--radius) items-center focus:bg-surface-raised hover:bg-surface-raised disabled:opacity-50";
const SEPARATOR = "h-px w-full my-[4px] bg-border";

/** The row of menus, while the menu button shows it. */
export function MainMenuBar(props: { font: () => Font }) {
  const menus = signal<MainMenuList[]>([]);
  // Asked again as a menu opens and after a pick, for the check marks and the recent projects.
  const load = () => void MainMenu.menus().then((m) => menus.set(m));
  load();
  const open = openMenu;
  const size = () => Math.max(9, props.font().size);
  return (
    <Toolbar accessibilityLabel="Menu bar" class="flex-row shrink-0 self-stretch" testID="main-menu-bar">
      <Each of={menus()} key={(m: MainMenuList) => m.name}>
        {(menu, i) => (
          <Menu
            open={open() === i()}
            offset={0}
            margin={4}
            popupClass={POPUP}
            onOpenChange={(e: { open: boolean }) => {
              if (e.open) {
                open.set(i());
                load();
              } else if (open() === i()) {
                open.set(-1);
                Shell.focusKeys();
              }
            }}
            trigger={
              <View
                class="shrink-0 h-9 justify-center px-2 cursor-default hover:bg-surface-raised/50"
                classList={{ "bg-surface-raised/50": open() === i() }}
                accessibilityLabel={menu().name}
                testID={`menu-${menu().name}`}
              >
                <Label font={props.font} on={open() === i()} text={menu().name} />
              </View>
            }
          >
            <Each of={menu().items} key="index">
              {(item) => <Row item={item} size={size} load={load} />}
            </Each>
          </Menu>
        )}
      </Each>
    </Toolbar>
  );
}

/** An item, a separator or a submenu of a menu. Its kind stays: the menus keep their shape. */
function Row(props: { item: () => MainMenuItem; size: () => number; load: () => void }) {
  const item = props.item;
  if (item().separator) return <MenuSeparator class={SEPARATOR} />;
  if (item().items.length === 0) return <Item item={item} size={props.size} load={props.load} />;
  return (
    <Submenu
      trigger={
        <View class={ROW} testID={`menu-item-${item().label}`}>
          <View class="w-[14px]" />
          <Text class="flex-1 text-foreground" style={{ fontSize: props.size(), numberOfLines: 1 }}>
            {item().label}
          </Text>
          <Icon name="chevronRight" size={props.size()} class="text-muted-foreground" />
        </View>
      }
    >
      <Each of={item().items} key="index">
        {(sub, i) => <Item item={sub} size={props.size} load={props.load} testID={`menu-sub-item-${i()}`} />}
      </Each>
    </Submenu>
  );
}

/** An item that runs when picked, as the same item of the menu bar. One with a check mark keeps
 * the menu open, so the mark shows its new state. */
function Item(props: { item: () => MainMenuItem | MainMenuSubItem; size: () => number; load: () => void; testID?: string }) {
  const item = props.item;
  if (item().separator) return <MenuSeparator class={SEPARATOR} />;
  const checked = () => {
    const i = item();
    return "checkable" in i && i.checkable ? i.checked : undefined;
  };
  return (
    <MenuItem
      class={ROW}
      disabled={!item().enabled}
      checked={checked()}
      closeOnSelect={checked() === undefined}
      accessibilityLabel={item().label}
      testID={props.testID ?? `menu-item-${item().label}`}
      onSelect={() => {
        MainMenu.pick(item().id);
        if (checked() !== undefined) props.load();
      }}
    >
      <View class="w-[14px] items-center">
        <If when={checked()}>
          <Icon name="check" size={props.size()} class="text-foreground" />
        </If>
      </View>
      <Text class="flex-1 text-foreground" style={{ fontSize: props.size(), numberOfLines: 1 }}>
        {item().label}
      </Text>
    </MenuItem>
  );
}
