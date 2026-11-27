// The welcome page: what a window with no project shows in place of the text, as Vim and Zed
// do. It lists the first things to do and the recent projects. Rust gives the rows
// (`Welcome` in nsvim-native). A press runs the same item as the menu, then the keys go back
// to the editor.

import { Text, View } from "nsh";
import { Button } from "nsh/base";
import { Welcome, type WelcomeRow, type WelcomeState } from "nsvim-native";
import { act } from "./controls";
import { Line, useFont, type FontState } from "./font";

/** One row: its label on the left and its keys, muted, on the right. */
function Row(props: { font: () => FontState; row: WelcomeRow }) {
  return (
    <Button
      accessibilityLabel={props.row.label}
      testID="welcome-row"
      onPress={act(() => Welcome.pick(props.row.id))}
      class="flex-row items-center h-7 px-2 gap-4 rounded-(--radius) hover:bg-surface-raised active:bg-muted"
      style={{ cursor: "default" }}
    >
      <Line font={props.font} class="flex-1 shrink truncate text-foreground">
        {props.row.label}
      </Line>
      <Line font={props.font} delta={-0.5} mono class="shrink-0 text-muted-foreground">
        {props.row.keys}
      </Line>
    </Button>
  );
}

/** A titled list of rows. */
function Section(props: { font: () => FontState; title: string; rows: WelcomeRow[]; empty?: string }) {
  return (
    <View class="gap-1">
      <Line font={props.font} delta={-1} class="px-2 pb-1 text-muted-foreground">
        {props.title}
      </Line>
      <If when={props.rows.length > 0} else={<Line font={props.font} class="px-2 h-7 text-muted-foreground">{props.empty ?? ""}</Line>}>
        {props.rows.map((r) => (
          <Row font={props.font} row={r} />
        ))}
      </If>
    </View>
  );
}

/** The welcome page, over the space of the text. */
export function WelcomePage(props: { state: () => WelcomeState }) {
  const font = useFont();
  return (
    <View testID="welcome" class="absolute left-0 right-0 top-9 bottom-0 items-center justify-center bg-background">
      <View class="w-100 max-w-full gap-6 px-4">
        <View class="gap-1 px-2">
          <Text class="text-2xl font-bold text-foreground">NsVim</Text>
          <Line font={font} class="text-muted-foreground">
            {`Version ${props.state().version}`}
          </Line>
        </View>
        <Section font={font} title="Start" rows={props.state().actions} />
        <Section font={font} title="Recent projects" rows={props.state().recent} empty="No recent projects" />
      </View>
    </View>
  );
}
