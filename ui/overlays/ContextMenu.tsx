// The in-app context menu at the pointer, kept inside the window. Linux shows it. Other
// systems show their own menu (`native_menu` in Rust).

import { signal, Text, View, windowSize } from "nsh";
import { Overlays, Shell, type MenuState } from "nsvim/native";

/** The menu's width, in points. */
const WIDTH = 260;

/** The menu's height: rows of 24 points, separators of 9, and 8 of padding. */
function height(m: MenuState) {
  return 8 + m.items.reduce((h, i) => h + (i.separator ? 9 : 24), 0);
}

/** The menu, once Rust sent it. */
export function ContextMenu() {
  const menu = signal<MenuState | null>(null);
  Overlays.onMenu((m) => menu.set(m));
  return <If when={menu()}>{(m) => <Menu part={m} />}</If>;
}

function Menu(props: { part: () => MenuState }) {
  const part = props.part;
  const left = () => Math.max(Math.min(part().x, windowSize().width - WIDTH - 8), 4);
  const top = () => {
    const h = height(part());
    const y = part().y;
    return y + h > windowSize().height ? Math.max(y - h, 4) : y;
  };
  return (
    <View class="absolute" style={{ left: left(), top: top() }} onPress={() => Shell.focusKeys()}>
      <View
        class="py-[4px] rounded-(--radius) bg-muted border border-border shadow-lg"
        style={{ width: WIDTH }}
        role="menu"
        testID="context-menu"
      >
        <Each of={part().items} key="index">
          {(item, index) => (
            <If
              when={!item().separator}
              else={<View class="h-px w-full my-[4px] bg-border" />}
            >
              <View
                class="flex-row h-[24px] px-[12px] items-center"
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
