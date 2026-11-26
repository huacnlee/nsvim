// The buffer line: the project folder over the tree, then the tabs that Rust lists. It is the
// window's title bar (`./title-bar.tsx`): a press on it outside a tab drags the window. A click on
// a tab shows it. A click on its close button or a middle click closes it. A right-click opens
// the tab's menu (close the others, the tabs to its left or right, the clean tabs, all of them,
// copy its path). A tab with unsaved changes shows a small square in place of the close mark. Rust owns the state (`Tabs` in
// `ext/nsvim-native/src/exports/buffer_line.rs`).

import { effect, platform, Scroll, signal, Text, View, type ContextMenuEvent, type MouseEvent, type ViewRef } from "nsh";
import { Icon, Shell, Tabs, type BufferLineState, type TabState } from "nsvim-native";
import { Controls, controlsLeft, DRAG } from "./title-bar";

/** The font of the interface: the chrome has fixed heights, so its text follows config.toml's
 * size and not the system text size. */
type Font = { family: string; size: number };

/** Runs `f`, then gives the keys back to the editor. */
function then(f: () => void) {
  f();
  Shell.focusKeys();
}

/** The frame of a tab. The tab that shows has a raised background. The other tabs have the
 * background of the line, and a softer step of the raised background under the pointer. */
function frame(on: boolean) {
  return `flex-row shrink-0 self-stretch items-center cursor-pointer pl-3 pr-1.5 ${on ? "bg-surface-raised" : "hover:bg-surface-raised/50"}`;
}

/** One line of interface text. With `cutStart`, text that does not fit loses its start, not
 * its end. */
function Label(props: { font: () => Font; on?: boolean; text: string; class?: string; cutStart?: boolean }) {
  return (
    <Text
      allowFontScaling={false}
      class={`${props.on ? "text-foreground" : "text-muted-foreground"} ${props.class ?? ""}`}
      style={{
        fontFamily: props.font().family,
        fontSize: Math.max(9, props.font().size),
        numberOfLines: 1,
        textOverflow: props.cutStart ? "ellipsis-start" : "ellipsis",
      }}
    >
      {props.text}
    </Text>
  );
}

/** The mark of a tab: a small solid square while it has unsaved changes, else an X. The square sits in a
 * box as large as the X, so the tab keeps its width. */
function Mark(props: { font: () => Font; index: number; dirty: boolean }) {
  // A terminal has no box smaller than a cell: the square is a char there, or the button
  // would show its label ("Close") as its text.
  if (props.dirty && platform.idiom === "terminal")
    return (
      <Text class="text-warning" accessibilityLabel="Unsaved changes" testID={`tab-mark-${props.index}`}>
        ●
      </Text>
    );
  return (
    <If
      when={props.dirty}
      else={
        <Icon name="x" size={props.font().size + 1} label="Close" class="text-muted-foreground" testID={`tab-mark-${props.index}`} />
      }
    >
      <View class="items-center justify-center" style={{ width: props.font().size + 1, height: props.font().size + 1 }}>
        <View
          class="size-2 rounded-[1px] bg-warning"
          accessible
          role="image"
          accessibilityLabel="Unsaved changes"
          testID={`tab-mark-${props.index}`}
        />
      </View>
    </If>
  );
}

/** Tab `index`. */
function Tab(props: { tab: () => TabState; index: number; font: () => Font }) {
  const i = props.index;
  let node: ViewRef | undefined;
  // The tab that shows always stays in sight: when it becomes the one that shows, the line
  // scrolls just enough to bring it in.
  effect(() => {
    if (props.tab().on) node?.reveal({ horizontal: "nearest", animated: false });
  });
  return (
    <View
      ref={(n) => {
        node = n;
        if (props.tab().on) n?.reveal({ horizontal: "nearest", animated: false });
      }}
      class={frame(props.tab().on)}
      role="tab"
      accessibilityLabel={props.tab().name}
      accessibilityState={{ selected: props.tab().on }}
      testID={`tab-${i}`}
      onPress={() => then(() => Tabs.show(i))}
      onMouseUp={(e: MouseEvent) => {
        if (e.button === "middle") then(() => Tabs.close(i));
      }}
      onContextMenu={(e: ContextMenuEvent) => then(() => Tabs.openMenu(i, e.pagePosition.x, e.pagePosition.y))}
    >
      <Label font={props.font} on={props.tab().on} text={props.tab().name} />
      {/* Only the mark's square takes the click, not the tab's full height. */}
      <View
        class="ml-1.5 items-center justify-center rounded cursor-pointer hover:bg-border"
        style={{ width: props.font().size + 7, height: props.font().size + 7 }}
        role="button"
        accessibilityLabel="Close"
        testID={`tab-close-${i}`}
        onPress={() => then(() => Tabs.close(i))}
      >
        {/* Each mark is its own icon, made again only when the tab becomes dirty or clean. */}
        <If when={props.tab().dirty} else={<Mark font={props.font} index={i} dirty={false} />}>
          <Mark font={props.font} index={i} dirty />
        </If>
      </View>
    </View>
  );
}

/** The line of open files above the text, and the window's title bar. */
export function BufferLine() {
  const project = signal<BufferLineState["project"]>(undefined);
  const tabs = signal<TabState[]>([]);
  const font = signal<Font>({ family: "system-ui", size: 13 });
  // Rust posts the state only when it changed. The project and the font are set only when
  // their own part changed, so a change to a tab redraws only the tabs.
  Tabs.onChange((s) => {
    const p = project();
    if (!s.project || !p || p.path !== s.project.path || p.width !== s.project.width) project.set(s.project);
    tabs.set(s.tabs);
    if (font().family !== s.fontFamily || font().size !== s.fontSize) font.set({ family: s.fontFamily, size: s.fontSize });
  });
  return (
    <View class="flex-row items-center h-9 shrink-0 overflow-hidden bg-muted" role="tablist" testID="buffer-line" desktop={DRAG}>
      <If when={project()} else={<Controls side="left" />}>
        {(p) => (
          // As wide as the tree, with the window controls at its start. The path keeps its end,
          // which tells folders apart.
          <View
            class="flex-row shrink-0 self-stretch items-center pr-3 overflow-hidden border-r border-border"
            style={{ width: p().width, paddingLeft: controlsLeft() > 0 ? controlsLeft() + 4 : 12 }}
            accessibilityLabel={p().path}
            testID="project"
          >
            <Label font={font} text={p().path} class="flex-1 min-w-0" cutStart />
          </View>
        )}
      </If>
      {/* The tabs scroll sideways when they do not fit, and stop before the window controls.
          The space after the last tab still drags the window. */}
      <Scroll
        horizontal
        scrollbar={false}
        class="flex-1 min-w-0 self-stretch"
        contentClass="flex-row grow items-stretch"
        testID="tabs"
        desktop={DRAG}
      >
        <Each of={tabs()} key="index">
          {(tab, index) => <Tab tab={tab} index={index()} font={font} />}
        </Each>
        <View class="flex-1 self-stretch" desktop={DRAG} />
      </Scroll>
      <Controls side="right" />
    </View>
  );
}
