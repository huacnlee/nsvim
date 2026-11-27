// nsvim's entry. Rust owns the editor's state (`ext/nsvim-native`): `Workspace` makes the
// session and takes every key, and the modules of `nsvim-native` give the screens the parts of
// the state they show. The screens are here and under `src/`.

import { mount, signal, View } from "nsh";
import { currentWindow, isAvailable as hasWindows } from "nsh/desktop";
import { Shell, Tabs, Welcome, Workspace, type ScreenState, type WelcomeState } from "nsvim-native";
import { Tree } from "./src/tree";
import { Corner, ProjectBar } from "./src/buffer-line";
import { CommandLine } from "./src/command-line";
import { FindBar } from "./src/find-bar";
import { StatusLine } from "./src/status-line";
import { Settings } from "./src/settings/Settings";
import { Setup } from "./src/setup/Setup";
import { Overlays } from "./src/overlays/Overlays";
import { WelcomePage } from "./src/welcome";
import { Panes, type Font } from "./src/panes";

/** The editor: the tree beside the panes of the open files, the status line and the command
 * line, with the overlays over them. There is no title bar row: the tree's project bar and the
 * panes' tab bars are the top of the window. Settings covers all of it, the overlays too. A
 * window with no project and no file open shows the welcome page in place of the panes. */
function Editor(props: { screen: () => ScreenState }) {
  const welcome = signal<WelcomeState>({ show: false, version: "", actions: [], recent: [] });
  Welcome.onChange((w) => welcome.set(w));
  const font = signal<Font>({ family: "system-ui", size: 13 });
  Tabs.onChange((s) => {
    if (font().family !== s.fontFamily || font().size !== s.fontSize) font.set({ family: s.fontFamily, size: s.fontSize });
  });
  // The width of the corner at the start of the tab bar, while the tree does not show.
  const corner = signal(0);
  return (
    <View class="flex-1 bg-background">
      <View class="flex-row flex-1 min-h-0">
        <If when={props.screen().tree}>
          <View class="shrink-0 min-h-0">
            <ProjectBar font={font} />
            <View class="flex-row flex-1 min-h-0">
              <Tree />
            </View>
          </View>
        </If>
        {/* The welcome page covers the panes under the top bar. */}
        <View class="flex-1 min-w-0">
          <Panes font={font} lead={() => (props.screen().tree ? 0 : corner())} />
          <FindBar />
          <If when={welcome().show}>
            <WelcomePage state={welcome} />
          </If>
          <If when={!props.screen().tree}>
            <Corner font={font} onWidth={(w) => corner.set(w)} />
          </If>
        </View>
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
