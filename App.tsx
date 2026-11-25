// nsvim's entry. Rust owns the editor's state (`ext/nsvim-native`): `Workspace` makes the
// session and takes every key, and the modules of `nsvim-native` give the screens the parts of
// the state they show. The screens are here and under `src/`.

import { mount, signal, View } from "nsh";
import { currentWindow, isAvailable as hasWindows } from "nsh/desktop";
import { Shell, TextArea, Workspace, type ScreenState } from "nsvim-native";
import { Tree } from "./src/tree";
import { BufferLine } from "./src/buffer-line";
import { CommandLine } from "./src/command-line";
import { StatusLine } from "./src/status-line";
import { Settings } from "./src/settings/Settings";
import { Setup } from "./src/setup/Setup";
import { Overlays } from "./src/overlays/Overlays";

/** The editor: the buffer line, the tree beside the text, the status line and the command
 * line, with the overlays over them. Settings covers all of it, the overlays too. */
function Editor(props: { screen: () => ScreenState }) {
  return (
    <View class="flex-1 bg-background">
      <BufferLine />
      <View class="flex-row flex-1 min-h-0">
        <If when={props.screen().tree}>
          <Tree />
        </If>
        <TextArea />
      </View>
      <StatusLine />
      <CommandLine />
      <Overlays />
      <If when={props.screen().settings}>
        <Settings />
      </If>
    </View>
  );
}

function App() {
  const screen = signal<ScreenState>({ setup: false, settings: false, tree: false });
  Shell.onChange((s) => screen.set(s));
  // The window stays open on a close press (Workspace sets closeOnPress false): the app
  // decides whether it quits. The web and the terminal have no window, and currentWindow()
  // throws there.
  if (hasWindows) currentWindow().on("closePress", () => Shell.closeRequested());
  return (
    <Workspace>
      <If when={screen().setup} else={<Editor screen={screen} />}>
        <Setup />
      </If>
    </Workspace>
  );
}

mount(App);
