// nsvim's entry. Rust owns the editor's state (`nsh/src`): `Workspace` makes the session and
// takes every key, and the modules of `nsvim/native` give the screens the parts of the state
// they show. The screens are here and under `ui/`.

import { mount, signal, View } from "nsh";
import { currentWindow } from "nsh/desktop";
import {
  BufferLine,
  CommandLine,
  Overlays,
  Settings,
  Setup,
  Shell,
  StatusLine,
  TextArea,
  Tree,
  Workspace,
  type ScreenState,
} from "nsvim/native";

/** The editor: the buffer line, the tree beside the text (or Settings), the status line and
 * the command line, with the overlays over them. */
function Editor(props: { screen: () => ScreenState }) {
  return (
    <View class="flex-1 bg-background">
      <BufferLine />
      <View class="flex-row flex-1 min-h-0">
        <If when={props.screen().tree}>
          <Tree />
        </If>
        <If when={props.screen().settings} else={<TextArea />}>
          <Settings />
        </If>
      </View>
      <StatusLine />
      <CommandLine />
      <Overlays />
    </View>
  );
}

function App() {
  const screen = signal<ScreenState>({ setup: false, settings: false, tree: false });
  Shell.onChange((s) => screen.set(s));
  // The window stays open on a close press (Workspace sets closeOnPress false): the app
  // decides whether it quits.
  currentWindow().on("closePress", () => Shell.closeRequested());
  return (
    <Workspace>
      <If when={screen().setup} else={<Editor screen={screen} />}>
        <Setup />
      </If>
    </Workspace>
  );
}

mount(App);
