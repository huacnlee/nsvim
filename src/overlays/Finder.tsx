// The finder panel, near the top of the window: the query line, the rows, and a preview of
// the picked file. Files, project search, buffers, commands, locations, code actions and the
// key list all share it. Its keys stay in Rust (`App::key`).

import { signal, Text, View, windowSize } from "nsh";
import { Icon, Overlays, Shell, type PickHead } from "nsvim-native";

import { PickList } from "./PickList";
import { Preview } from "./Preview";
import { SIZE, TERM, Titled } from "./Titled";

/** The query line: a chevron, the query and its caret, a hint while it is empty, and the
 * count. */
function Query(props: { head: PickHead }) {
  if (TERM)
    // A terminal draws no frame around one row of text: the query line is the title, then
    // the query, one row high.
    return (
      <View class="flex-row h-[16px] w-full px-[8px] items-center" testID="picker-input">
        <Text class="text-primary">{`${props.head.title} › `}</Text>
        <Text testID="picker-query">{props.head.query}</Text>
        <View class="w-[8px] h-[16px] bg-primary" testID="picker-caret" />
        <If when={!props.head.query}>
          <Text class="text-text-faint">{" Type to filter"}</Text>
        </If>
        <View class="flex-1" />
        <Text class="text-muted-foreground" testID="picker-count">
          {props.head.count}
        </Text>
      </View>
    );
  return (
      <Titled title={props.head.title} class="h-[64px] w-full">
        <View class="flex-row flex-1 px-[8px] items-center" testID="picker-input">
          <Text class="text-primary">{"› "}</Text>
          <Text testID="picker-query">{props.head.query}</Text>
          <View class="w-[8px] h-[16px] bg-primary" testID="picker-caret" />
          <If when={!props.head.query}>
            <Text class="text-text-faint">{" Type to filter"}</Text>
          </If>
          <View class="flex-1" />
          <Text class="text-muted-foreground" testID="picker-count">
            {props.head.count}
          </Text>
        </View>
      </Titled>
    );
  return (
    <Titled title={props.head.title} class="h-[40px] w-full">
      <View class="flex-row flex-1 px-[8px] items-center" testID="picker-input">
        <View class="w-[18px]">
          <Icon name="chevronRight" size={14} class="text-primary" />
        </View>
        <Text class="text-[13px]" testID="picker-query">
          {props.head.query}
        </Text>
        <View class="w-[1.5px] h-[16px] bg-primary" testID="picker-caret" />
        <If when={!props.head.query}>
          <Text class="text-[13px] text-text-faint">{"  Type to filter"}</Text>
        </If>
        <View class="flex-1" />
        <Text class="text-muted-foreground" style={{ fontSize: SIZE }} testID="picker-count">
          {props.head.count}
        </Text>
      </View>
    </Titled>
  );
}

/** The panel, once Rust sent its query line. */
export function Finder() {
  const head = signal<PickHead | null>(null);
  Overlays.onPickHead((h) => head.set(h));
  return <If when={head()}>{(h) => <Panel head={h} />}</If>;
}

function Panel(props: { head: () => PickHead }) {
  const head = props.head;
  const width = () => Math.min(Math.max(windowSize().width - 32, 320), 1040);
  const height = () => Math.max(Math.min(560, windowSize().height - 100), 200);
  const left = () => Math.max((windowSize().width - width()) / 2, 16);
  const top = () => Math.min(80, windowSize().height * 0.1 + 20);
  const listWidth = () => (head().wide ? width() - 20 : Math.max(width() * 0.42, 300));
  if (TERM)
    // A terminal lays out in cells: the panel takes the editor's rows below the buffer line
    // and above the status line, edge to edge, so no half cell of code shows beside it.
    return (
      <View
        class="absolute left-0 right-0 top-[16px] bottom-[32px] flex-row gap-[8px] px-[8px] bg-background"
        role="dialog"
        testID="picker"
        onPress={() => Shell.focusKeys()}
      >
        <View class="h-full" style={{ width: head().wide ? "100%" : Math.floor((windowSize().width * 0.42) / 8) * 8 }}>
          <Query head={head()} />
          <Titled title="Results" class="w-full flex-1 min-h-0">
            <PickList empty={head().empty} />
          </Titled>
        </View>
        <If when={!head().wide}>
          <Preview />
        </If>
      </View>
    );
  return (
    <View class="absolute" style={{ left: left(), top: top() }}>
      <View
        class="flex-row gap-[14px] p-[10px] rounded-(--radius) bg-background border border-border shadow-xl"
        style={{ width: width(), height: height() }}
        role="dialog"
        testID="picker"
        onPress={() => Shell.focusKeys()}
      >
        <View class="h-full gap-[14px]" style={{ width: listWidth() }}>
          <Query head={head()} />
          <Titled title="Results" class="w-full flex-1 min-h-0">
            <PickList empty={head().empty} />
          </Titled>
        </View>
        <If when={!head().wide}>
          <Preview />
        </If>
      </View>
    </View>
  );
}
