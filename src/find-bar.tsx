// The find bar under the panes: find and replace in the active file, with no Vim syntax. Rust
// gives what it shows (`Find` in nsvim-native) and takes its keys (`App::find_key`). A press
// asks Rust for a change and gives the keys back to the editor's key view.

import { type JSX, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Find, type FindState, Icon } from "nsvim-native";
import { act, Check } from "./controls";

/** One text field: its text and a caret while it has the keys, else a hint. */
function Field(props: { name: string; text: string; on: boolean; testID: string; onPress: () => void }) {
  return (
    <View
      class="flex-row flex-1 min-w-0 h-6 px-2 items-center rounded-(--radius) border bg-background cursor-text"
      classList={{ "border-mode-insert": props.on, "border-border": !props.on }}
      role="textbox"
      accessibilityLabel={props.name}
      testID={props.testID}
      onPress={act(props.onPress)}
    >
      <Text class="shrink truncate text-[13px] text-foreground" testID={`${props.testID}-text`}>
        {props.text}
      </Text>
      <If when={props.on}>
        <View class="w-[1.5px] h-[15px] bg-primary" testID={`${props.testID}-caret`} />
      </If>
      <If when={!props.text}>
        <Text class="text-[13px] text-text-faint">{props.on ? ` ${props.name}` : props.name}</Text>
      </If>
    </View>
  );
}

/** A small square button with an icon or a label. */
function Tool(props: { label: string; testID: string; onPress: () => void; children?: JSX.Element }) {
  return (
    <Button
      accessibilityLabel={props.label}
      testID={props.testID}
      onPress={act(props.onPress)}
      class="h-6 min-w-6 px-1.5 shrink-0 items-center justify-center rounded-(--radius) hover:bg-surface-raised active:bg-muted"
    >
      {props.children}
    </Button>
  );
}

/** The bar, while it shows. */
export function FindBar() {
  const find = signal<FindState | null>(null);
  Find.onChange((f) => find.set(f.shows ? f : null));
  return <If when={find()}>{(f) => <Bar find={f} />}</If>;
}

function Bar(props: { find: () => FindState }) {
  const f = props.find;
  return (
    <View class="shrink-0 gap-1.5 px-2 py-1.5 bg-muted border-t border-border" role="search" testID="find-bar">
      <View class="flex-row items-center gap-1.5">
        <Tool
          label={f().replace ? "Hide replace" : "Show replace"}
          testID="find-toggle-replace"
          onPress={() => Find.showReplace(!f().replace)}
        >
          <Icon name={f().replace ? "chevronDown" : "chevronRight"} size={14} class="text-muted-foreground" />
        </Tool>
        <Field name="Find" text={f().query} on={f().onFind} testID="find-query" onPress={() => Find.focus(false)} />
        <Check label="Match case" on={f().case} onPress={() => Find.toggle("case")} class="shrink-0 px-1 h-6">
          <Text class="text-xs text-muted-foreground">Match case</Text>
        </Check>
        <Check label="Whole word" on={f().word} onPress={() => Find.toggle("word")} class="shrink-0 px-1 h-6">
          <Text class="text-xs text-muted-foreground">Whole word</Text>
        </Check>
        <Check label="Regex" on={f().regex} onPress={() => Find.toggle("regex")} class="shrink-0 px-1 h-6">
          <Text class="text-xs text-muted-foreground">Regex</Text>
        </Check>
        <View class="w-[84px] shrink-0 items-end">
          <Text
            class="text-xs"
            classList={{ "text-destructive": f().none, "text-muted-foreground": !f().none }}
            testID="find-count"
          >
            {f().count}
          </Text>
        </View>
        <Tool label="Previous match" testID="find-prev" onPress={() => Find.step(true)}>
          <Icon name="chevronUp" size={14} class="text-foreground" />
        </Tool>
        <Tool label="Next match" testID="find-next" onPress={() => Find.step(false)}>
          <Icon name="chevronDown" size={14} class="text-foreground" />
        </Tool>
        <Tool label="Close" testID="find-close" onPress={() => Find.close()}>
          <Icon name="x" size={14} class="text-muted-foreground" />
        </Tool>
      </View>
      <If when={f().replace}>
        <View class="flex-row items-center gap-1.5">
          <View class="w-6 shrink-0" />
          <Field name="Replace" text={f().with} on={f().onReplace} testID="find-with" onPress={() => Find.focus(true)} />
          <Tool label="Replace" testID="find-replace" onPress={() => Find.replace(false)}>
            <Text class="text-xs text-foreground">Replace</Text>
          </Tool>
          <Tool label="Replace all" testID="find-replace-all" onPress={() => Find.replace(true)}>
            <Text class="text-xs text-foreground">Replace all</Text>
          </Tool>
        </View>
      </If>
    </View>
  );
}
