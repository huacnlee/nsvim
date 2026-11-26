// The last two steps of setup: the terminal, and the summary before the editor opens.

import { Text, View } from "nsh";
import { Button } from "nsh/base";
import { type SetupState, SetupFlow } from "nsvim-native";
import { act, Kbd, Titled } from "../controls";
import { Heading, OptionBox, CheckRow } from "./parts";

type Step = { st: () => SetupState };

export function ShellStep(props: Step) {
  const st = props.st;
  return (
    <View class="gap-5.5" testID="setup-step-4">
      <Heading title="Use nsvim from the terminal" sub="Optional. Everything here can be undone by hand later." />
      <View class="flex-row gap-4 flex-wrap items-start">
        <OptionBox title="Integrations" class="flex-1 min-w-90">
          <CheckRow
            st={st}
            choice="toggle:vim"
            label="Open nsvim with nvim, vim and vi"
            sub={"Shell aliases to the full path of nsvim. \\nvim still runs Neovim."}
          />
          <CheckRow st={st} choice="toggle:env" label="Set as $EDITOR and $VISUAL" sub="For tools that open an editor, like crontab -e" />
          <CheckRow st={st} choice="toggle:git" label="Use for git commit messages" sub="git waits until you close the buffer" />
          <If when={st().desktopEntry}>
            <CheckRow
              st={st}
              choice="toggle:default"
              label="Make nsvim the default for text files"
              sub="A double-click opens code and text files"
            />
          </If>
        </OptionBox>
        <Titled title="What runs" class="flex-1 min-w-90" bodyClass="pt-4 pb-3.5 px-4 gap-1" testID="setup-commands">
          <If when={st().commands.length === 0}>
            <Text class="text-xs text-muted-foreground">Nothing. nsvim stays inside its window.</Text>
          </If>
          <Each of={st().commands} key="index">
            {(line) => {
              // A `#` line says what a step writes, as a comment in a shell script does.
              const note = () => line().startsWith("# ");
              return (
                <View class="flex-row w-full items-start">
                  <Text class="text-xs text-text-faint">{note() ? "" : "$ "}</Text>
                  <Text
                    class="flex-1 min-w-0 text-xs"
                    classList={{ "text-muted-foreground": note(), "text-foreground": !note() }}
                  >
                    {line()}
                  </Text>
                </View>
              );
            }}
          </Each>
        </Titled>
      </View>
    </View>
  );
}

/** The names of the terminal options, as the summary says them. */
const SHELL: [string, string][] = [
  ["vim", "nvim, vim and vi"],
  ["env", "$EDITOR"],
  ["git", "Git editor"],
  ["default", "Default app"],
];

/** The theme modes, as the summary says them. */
const LOOK: Record<string, string> = { system: "System", light: "Light", dark: "Dark" };

export function ReadyStep(props: Step) {
  const st = props.st;
  const md = () => (st().mac ? "⌘" : "Ctrl");
  const space = () => st().on.includes("leader");
  const rows = (): [string, string][] => {
    const s = st();
    const shell = SHELL.filter(([k]) => s.on.includes(k) && (k !== "default" || s.desktopEntry)).map(([, v]) => v);
    const theme = s.omarchy && s.themeMode === "system" ? `Omarchy (${s.omarchy} right now)` : (LOOK[s.themeMode] ?? s.themeMode);
    return [
      ["Keys", s.vim ? `Neovim motions, ${space() ? "Space" : "backslash"} leader` : `Classic shortcuts (${md()})`],
      ["Mouse", s.mouse ? "On" : "Keyboard only"],
      ["Theme", theme],
      ["Font", `${s.font}, ${s.fontSize} pt`],
      // Windows has no terminal step, so it has no row for it.
      ...(terminal() ? [["Terminal", shell.length ? shell.join(", ") : "None"] as [string, string]] : []),
    ];
  };
  const terminal = () => st().steps.includes("Terminal");
  // How to start nsvim after this window: the aliases, or the launcher and the full path.
  const start = () => {
    const s = st();
    if (s.devBuild) return `This is a development build. The terminal step is off. Run ${s.program} in a terminal.`;
    if (terminal() && s.on.includes("vim")) return "Then, in a new terminal, nvim . opens a folder. \\nvim still runs Neovim.";
    const launcher = terminal() ? "the app launcher" : "the Start menu";
    return `Then, open nsvim from ${launcher}, or run ${s.program} in a terminal.`;
  };
  const tips = (): [string, string[]][] => {
    const l = space() ? "SPC" : "\\";
    return st().vim
      ? [
          ["Find a file", [l, "f"]],
          ["Every key, in a popup", [l]],
          ["Command palette", [l, "p"]],
          ["Settings", [":Settings"]],
        ]
      : [
          ["Find a file", [md(), "P"]],
          ["Command palette", [md(), "⇧", "P"]],
          ["Settings", [md(), ","]],
        ];
  };
  return (
    <View class="gap-5.5" testID="setup-step-5">
      <Heading title="Ready" sub="Saved to ~/.config/nsvim/config.toml when you open nsvim." />
      <Text class="text-sm text-foreground" testID="setup-start">
        {start()}
      </Text>
      <View class="flex-row gap-4 flex-wrap items-start">
        <Titled title="Your setup" class="flex-1 min-w-105" bodyClass="pt-3 pb-1.5 px-4" testID="setup-summary">
          <Each of={rows()} key="index">
            {(row, i) => {
              const focus = () => st().focus === `change:${i()}`;
              return (
                <View class="flex-row min-h-8.5 w-full gap-4 items-center border-b border-border">
                  <Text class="w-21 text-sm text-muted-foreground">{row()[0]}</Text>
                  <Text class="flex-1 text-sm text-foreground">{row()[1]}</Text>
                  <Button
                    accessibilityLabel={`Change ${row()[0].toLowerCase()}`}
                    onPress={act(() => SetupFlow.choose(`change:${i()}`))}
                    class="px-1 rounded-(--radius) hover:opacity-80"
                    classList={{ "bg-surface-raised": focus(), "bg-background": !focus() }}
                  >
                    <Text class="text-xs text-primary">Change</Text>
                  </Button>
                </View>
              );
            }}
          </Each>
        </Titled>
        <Titled title="Try first" class="flex-1 min-w-75" bodyClass="pt-3 pb-1.5 px-4">
          <Each of={tips()} key="index">
            {(tip) => (
              <View class="flex-row min-h-8.5 w-full items-center border-b border-border">
                <Text class="flex-1 text-sm text-foreground">{tip()[0]}</Text>
                <View class="flex-row gap-1">
                  <Each of={tip()[1]} key="index">
                    {(k) => <Kbd>{k()}</Kbd>}
                  </Each>
                </View>
              </View>
            )}
          </Each>
        </Titled>
      </View>
    </View>
  );
}
