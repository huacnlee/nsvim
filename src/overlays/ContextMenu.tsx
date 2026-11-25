// The in-app context menu at the pointer, kept inside the window. Linux shows it. Other
// systems show their own menu (`native_menu` in Rust).

import { signal, Text, View, windowSize } from "nsh";
import { TERM } from "./Titled";
import { Overlays, Shell, type MenuState } from "nsvim-native";

/** The menu's width, in points: whole cells in a terminal. */
const WIDTH = TERM ? 256 : 260;
/** A row's height, and a separator's: one cell each in a terminal. */
const ROW = TERM ? 16 : 24;
const SEPARATOR = TERM ? 16 : 9;
/** The padding above and below the rows: the border's cell in a terminal. */
const PAD = TERM ? 32 : 8;

/** The menu's height: its rows, separators and padding. */
function height(m: MenuState) {
  return PAD + m.items.reduce((h, i) => h + (i.separator ? SEPARATOR : ROW), 0);
}

/** `n` down to whole cells of `cell` points, in a terminal. */
function snap(n: number, cell: number) {
  return TERM ? Math.floor(n / cell) * cell : n;
}

/** The menu, once Rust sent it. */
export function ContextMenu() {
  const menu = signal<MenuState | null>(null);
  Overlays.onMenu((m) => menu.set(m));
  return <If when={menu()}>{(m) => <Menu part={m} />}</If>;
}

function Menu(props: { part: () => MenuState }) {
  const part = props.part;
  const left = () => snap(Math.max(Math.min(part().x, windowSize().width - WIDTH - 8), 4), 8);
  const top = () => {
    const h = height(part());
    const y = part().y;
    return snap(y + h > windowSize().height ? Math.max(y - h, 4) : y, 16);
  };
  return (
    <View class="absolute" style={{ left: left(), top: top() }} onPress={() => Shell.focusKeys()}>
      <View
        class={TERM ? "bg-background border border-border" : "py-[4px] rounded-(--radius) bg-muted border border-border shadow-lg"}
        style={{ width: WIDTH }}
        role="menu"
        testID="context-menu"
      >
        <Each of={part().items} key="index">
          {(item, index) => (
            <If
              when={!item().separator}
              else={<View class={TERM ? "h-[16px] w-full" : "h-px w-full my-[4px] bg-border"} />}
            >
              <View
                class={TERM ? "flex-row h-[16px] px-[8px] items-center" : "flex-row h-[24px] px-[12px] items-center"}
                classList={{
                  "bg-surface-raised": part().sel === index(),
                  "hover:bg-surface-raised": part().sel !== index() && item().enabled,
                }}
                role="menuitem"
                accessibilityLabel={item().title}
                accessibilityState={{ selected: part().sel === index(), disabled: !item().enabled }}
                disabled={!item().enabled}
                onPress={() => {
                  Shell.focusKeys();
                  if (item().enabled) Overlays.menuPress(item().id);
                }}
              >
                <Text class={`text-sm flex-1 ${item().enabled ? "text-foreground" : "text-text-faint"}`}>{item().title}</Text>
                <Text class="text-xs text-text-faint">{item().hint}</Text>
              </View>
            </If>
          )}
        </Each>
      </View>
    </View>
  );
}
