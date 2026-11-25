// The status line under the text: the mode, the git branch, the file, the diagnostics, the
// language, the encoding and the position. Rust gives each part (`Status` in nsvim/native).

import { signal, View } from "nsh";
import { Icon, type ModeColor, type PositionState, Shell, Status, type StatusState } from "nsvim/native";
import { type FontState, Line, useFont } from "./font";

/** The fill of each mode colour. */
const FILL: Record<ModeColor, string> = {
  primary: "bg-primary",
  modeInsert: "bg-mode-insert",
  modeVisual: "bg-mode-visual",
  modeCommand: "bg-mode-command",
  destructive: "bg-destructive",
};

/** The text of the status line: half a point smaller than the interface text. */
const DELTA = -0.5;

/** A part at the end of the line. It shows while its text is not empty. */
function Tail(props: {
  font: () => FontState;
  text: string;
  color: string;
  id: string;
  icon?: "gitCompareArrows" | "diff";
}) {
  return (
    <If when={props.text !== ""}>
      <View class="flex-row items-center gap-[5px] px-3">
        <If when={props.icon}>
          {(name) => <Icon name={name()} size={props.font().size - 1} class={props.color} testID={`${props.id}-icon`} />}
        </If>
        <Line font={props.font} delta={DELTA} class={props.color} testID={props.id}>
          {props.text}
        </Line>
      </View>
    </If>
  );
}

/** The end of the line on the Settings tab: the theme, and `:w` to write config.toml. */
function SettingsEnd(props: { font: () => FontState; theme: string }) {
  return (
    <View class="flex-row items-center self-stretch">
      <Line font={props.font} delta={DELTA} class="px-3 text-muted-foreground">
        {props.theme}
      </Line>
      <View
        class="justify-center self-stretch px-3 bg-primary cursor-pointer"
        role="button"
        accessibilityLabel="Write config.toml"
        onPress={() => {
          Status.writeConfig();
          Shell.focusKeys();
        }}
      >
        <Line font={props.font} delta={DELTA} class="font-bold text-primary-foreground">
          :w
        </Line>
      </View>
    </View>
  );
}

/** The cursor's position, in the mode's colour in vim. */
function Position(props: { font: () => FontState; status: StatusState }) {
  const position = signal<PositionState>({ scroll: "", pos: "" });
  Status.onPosition((p) => position.set(p));
  return (
    <>
      <Tail font={props.font} text={props.status.lang === "" ? "" : position().scroll} color="text-muted-foreground" id="scroll" />
      <If when={position().pos !== ""}>
        <View
          class={`justify-center self-stretch px-3 ${props.status.vim ? FILL[props.status.modeColor] : "bg-surface-raised"}`}
          testID="position-fill"
        >
          <Line
            font={props.font}
            delta={DELTA}
            class={props.status.vim ? "font-bold text-primary-foreground" : "text-foreground"}
            testID="position"
          >
            {position().pos}
          </Line>
        </View>
      </If>
    </>
  );
}

/** The status line under the text. */
export function StatusLine() {
  const font = useFont();
  const status = signal<StatusState | null>(null);
  Status.onChange((s) => status.set(s));
  return (
    <View class="h-6 shrink-0 flex-row items-center overflow-hidden bg-muted" testID="status-line">
      <If when={status()}>
        {(st) => (
          <>
            <If when={st().badge}>
              {(badge) => (
                <View class={`justify-center self-stretch px-3 ${FILL[st().badgeColor]}`} testID="mode-badge">
                  <Line font={font} delta={DELTA} class="font-bold text-primary-foreground" testID="mode">
                    {badge()}
                  </Line>
                </View>
              )}
            </If>
            <If when={st().branch}>
              {(branch) => (
                <View class="flex-row items-center gap-1.5 self-stretch px-3 bg-surface-raised">
                  <Icon name="gitBranch" size={font().size - 1} class="text-muted-foreground" testID="branch-icon" />
                  <Line font={font} delta={DELTA} class="text-foreground" testID="branch">
                    {branch()}
                  </Line>
                </View>
              )}
            </If>
            <Line font={font} delta={DELTA} class="shrink truncate px-3 text-foreground" testID="file">
              {st().file}
            </Line>
            <View class="flex-1" />
            <If when={st().settings}>{(theme) => <SettingsEnd font={font} theme={theme()} />}</If>
            <Tail font={font} text={st().errors > 0 ? `E ${st().errors}` : ""} color="text-destructive" id="errors" />
            <Tail font={font} text={st().warnings > 0 ? `W ${st().warnings}` : ""} color="text-warning" id="warnings" />
            <Tail
              font={font}
              text={proposals(st().proposals)}
              color="text-mode-visual"
              id="proposals"
              icon="gitCompareArrows"
            />
            <Tail font={font} text={st().outside > 0 ? `${st().outside}` : ""} color="text-mode-visual" id="outside" icon="diff" />
            <Tail font={font} text={st().lang} color="text-muted-foreground" id="language" />
            <Tail font={font} text={st().lang === "" ? "" : st().encoding} color="text-muted-foreground" id="encoding" />
            <Position font={font} status={st()} />
          </>
        )}
      </If>
    </View>
  );
}

/** The count of proposed edits, in words. Empty for none. */
function proposals(n: number): string {
  if (n === 0) return "";
  return n === 1 ? "1 proposal" : `${n} proposals`;
}
