// The overlays over the editor: the leader key menu, the finder with its pickers, and the
// in-app context menu. Rust owns their state (`Overlays` in `nsvim-native`) and takes their
// keys. These views show the state and send clicks. A press gives the focus back to the view
// that takes the keys.

import { platform, signal, View } from "nsh";
import { Overlays as State, Shell, type OverlayKind } from "nsvim-native";

import { ContextMenu } from "./ContextMenu";
import { Finder } from "./Finder";
import { Leader } from "./Leader";

/** The overlay that shows now, over the whole editor screen, on a scrim that closes it. */
export function Overlays() {
  const kind = signal<OverlayKind>("none");
  State.onChange((k) => kind.set(k));
  return (
    <If when={kind() !== "none"}>
      <View class="absolute inset-0">
        <View
          class="absolute inset-0"
          // A terminal draws the label of a view with no content as text. The scrim stays blank there.
          accessibilityLabel={platform.idiom === "terminal" ? undefined : "Close"}
          testID="overlay-scrim"
          onPress={() => {
            Shell.focusKeys();
            State.close();
          }}
        />
        <If when={kind() === "leader"}>
          <Leader />
        </If>
        <If when={kind() === "pick"}>
          <Finder />
        </If>
        <If when={kind() === "menu"}>
          <ContextMenu />
        </If>
      </View>
    </If>
  );
}
