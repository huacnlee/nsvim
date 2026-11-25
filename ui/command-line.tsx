// The command line under the status line: the `:`, `/` or `?` input with its caret, else a
// message, an error or the mode, and the pending keys or a button that shows the keys. Rust
// gives what it shows (`Cmdline` in nsvim/native).

import { signal, View } from "nsh";
import { Cmdline, type CommandState, Shell } from "nsvim/native";
import { type FontState, Line, useFont } from "./font";

/** The size of the command line's text, in points. */
const SIZE = 12.5;

/** The colour and weight of each kind of message. */
const TONE = {
  quiet: { class: "shrink truncate text-muted-foreground", id: "message" },
  error: { class: "shrink truncate text-destructive", id: "error" },
  mode: { class: "shrink truncate font-bold text-foreground", id: "message" },
} as const;

/** The command line. It shows always in vim, and in the classic keymap only for a command or
 * a message. */
export function CommandLine() {
  const font = useFont();
  // Vim is the default keymap: the line shows from the first frame, so the text area keeps
  // its height.
  const command = signal<CommandState>({
    shows: true,
    typing: false,
    before: "",
    after: "",
    text: "",
    tone: "quiet",
    hint: "none",
    pending: "",
  });
  Cmdline.onChange((c) => command.set(c));
  return (
    <If when={command().shows}>
      <View class="h-[22px] shrink-0 flex-row items-center overflow-hidden px-2 bg-background" testID="command-line">
        <If when={command().typing} else={<Message font={font} command={command()} />}>
          <View class="flex-1 flex-row items-center">
            <Line font={font} size={SIZE} mono class="text-foreground" testID="command-input">
              {command().before}
            </Line>
            <View class="bg-primary" style={{ width: 1.5, height: 15 }} testID="command-caret" />
            <Line font={font} size={SIZE} mono class="text-foreground" testID="command-after">
              {command().after}
            </Line>
          </View>
        </If>
      </View>
    </If>
  );
}

/** A message, an error or the mode, then the hint at the end. */
function Message(props: { font: () => FontState; command: CommandState }) {
  return (
    <View class="flex-1 flex-row items-center">
      <Line font={props.font} size={SIZE} mono class={TONE[props.command.tone].class} testID={TONE[props.command.tone].id}>
        {props.command.text}
      </Line>
      <View class="flex-1" />
      <If when={props.command.hint === "leader"}>
        <Line font={props.font} size={SIZE} mono class="text-foreground" testID="pending">
          {"<space>"}
        </Line>
      </If>
      <If when={props.command.hint === "pending"}>
        <Line font={props.font} size={SIZE} mono class="text-foreground" testID="pending">
          {props.command.pending}
        </Line>
      </If>
      <If when={props.command.hint === "keys"}>
        <View
          class="justify-center self-stretch px-1 cursor-pointer hover:bg-surface-raised"
          role="button"
          accessibilityLabel="Show the keys"
          onPress={() => {
            Cmdline.showKeys();
            Shell.focusKeys();
          }}
        >
          <Line font={props.font} size={12} mono class="text-text-faint">
            {"<space> for keys"}
          </Line>
        </View>
      </If>
    </View>
  );
}
