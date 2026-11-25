// The leader key menu: a grid of keys and what they do, at the bottom of the window.

import { signal, Text, View } from "nsh";
import { Icon, Overlays, Shell, type LeaderState } from "nsvim-native";

import { SIZE, TERM, Titled } from "./Titled";

/** The menu, once Rust sent it. */
export function Leader() {
  const leader = signal<LeaderState | null>(null);
  Overlays.onLeader((p) => leader.set(p));
  return <If when={leader()}>{(p) => <Menu part={p} />}</If>;
}

function Menu(props: { part: () => LeaderState }) {
  const part = props.part;
  return (
    <View class={TERM ? "absolute left-0 right-0" : "absolute left-[12px] right-[12px]"} style={{ bottom: TERM ? Math.ceil(part().bottom / 16) * 16 : part().bottom }} onPress={() => Shell.focusKeys()}>
      <Titled title={part().title} class={TERM ? "w-full" : "w-full pt-[14px] pb-[10px] px-[8px] shadow-lg"} role="menu" testID="leader">
        <View class={TERM ? "flex-row flex-wrap w-full px-[8px]" : "flex-row flex-wrap w-full"}>
          <Each of={part().items} key="index">
            {(item) => (
              <View
                class={`flex-row ${TERM ? "w-[248px] h-[16px]" : "w-[250px] h-[22px]"} px-[8px] gap-[8px] items-center cursor-pointer hover:bg-surface-raised`}
                role="menuitem"
                accessibilityLabel={item().label}
                testID={`leader-${item().key}`}
                onPress={() => {
                  Shell.focusKeys();
                  Overlays.leaderPress(item().key);
                }}
              >
                <Text class="font-bold text-primary" style={{ fontSize: SIZE }}>
                  {item().key}
                </Text>
                <Icon name="arrowRight" size={SIZE} class="text-text-faint" />
                <Text class={item().group ? "text-mode-visual" : "text-foreground"} style={{ fontSize: SIZE }}>
                  {item().label}
                </Text>
              </View>
            )}
          </Each>
        </View>
      </Titled>
    </View>
  );
}
