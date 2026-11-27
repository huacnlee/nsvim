// The panes: the open files in NSH's `DockArea`. Each tab group is a pane with its own tab bar,
// and its displayed file shows below it. A tab dragged along a tab bar moves there. Dropped on
// another pane, it joins that pane. Dropped on an edge of a pane, it splits the pane: a new pane
// on that side. The area runs the drags, the drop indicator and the handles between the panes.
// The file that has the keys is Rust's active file, and its pane draws the cursor. A click on
// another pane or tab makes its file the active one.
//
// A click on a tab's close button or a middle click closes it. A right-click opens the tab's
// menu. A tab with unsaved changes shows a small square in place of the close mark. Rust owns
// the open files (`Tabs` in `ext/nsvim-native/src/exports/buffer_line.rs`). The layout of the
// panes is this screen's: `./dock-layout.ts` keeps it in step with Rust's tabs.
//
// The pane commands (Split Right, `Ctrl w v`, `:vsplit`, Close Pane...) come from Rust as a
// numbered ask in the tabs' state. This screen carries out each number once, on the pane of
// the active file: DockArea has no focused pane, so the active file's pane is the focused one.

import { effect, platform, signal, Text, View, type ContextMenuEvent, type MouseEvent, type ViewRef } from "nsh";
import { DockArea, type DockAreaLayout, type DockTab } from "nsh/base";
import { Icon, Shell, Tabs, TextArea, type TabState } from "nsvim-native";
import { BarEnd, EmptyBar } from "./buffer-line";
import { closeGroup, groups, neighbour, panelsOf, sameLayout, split, sync, topRight } from "./dock-layout";

/** The font of the interface: the chrome has fixed heights, so its text follows config.toml's
 * size and not the system text size. */
export type Font = { family: string; size: number };

/** Runs `f`, then gives the keys back to the editor. */
function then(f: () => void) {
  f();
  Shell.focusKeys();
}

/** One line of interface text. With `cutStart`, text that does not fit loses its start, not
 * its end. */
export function Label(props: { font: () => Font; on?: boolean; text: string; class?: string; cutStart?: boolean }) {
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

/** What a tab shows: the file's name and its close button. The area wraps it: its press
 * displays the tab, and its drag moves it. `index` is the file's place in Rust's tabs. */
function Tab(props: { tab: () => TabState | undefined; index: () => number; font: () => Font }) {
  let node: ViewRef | undefined;
  // The tab of the active file stays in sight: when its file becomes the active one, the tab bar
  // scrolls just enough to bring it in. The area's own reveal runs before the first layout.
  effect(() => {
    if (props.tab()?.on) node?.reveal({ horizontal: "nearest", animated: false });
  });
  return (
    <If when={props.tab()}>
      {(tab) => (
        <View
          ref={(n) => {
            node = n;
            if (tab().on) n?.reveal({ horizontal: "nearest", animated: false });
          }}
          class="flex-row h-9 items-center pl-3 pr-1.5"
          accessibilityLabel={tab().name}
          testID={`tab-${props.index()}`}
          // A press shows the file and gives it the keys, also in a pane that already displays it.
          onPress={() => then(() => Tabs.showFile(tab().id))}
          onMouseUp={(e: MouseEvent) => {
            if (e.button === "middle") then(() => Tabs.close(props.index()));
          }}
          onContextMenu={(e: ContextMenuEvent) => then(() => Tabs.openMenu(props.index(), e.pagePosition.x, e.pagePosition.y))}
        >
          <Label font={props.font} on={tab().on} text={tab().name} />
          {/* Only the mark's square takes the click, not the tab's full height. */}
          <View
            class="ml-1.5 items-center justify-center rounded cursor-pointer hover:bg-border"
            style={{ width: props.font().size + 7, height: props.font().size + 7 }}
            role="button"
            accessibilityLabel="Close"
            testID={`tab-close-${props.index()}`}
            onPress={() => then(() => Tabs.close(props.index()))}
          >
            {/* Each mark is its own icon, made again only when the tab becomes dirty or clean. */}
            <If when={tab().dirty} else={<Mark font={props.font} index={props.index()} dirty={false} />}>
              <Mark font={props.font} index={props.index()} dirty />
            </If>
          </View>
        </View>
      )}
    </If>
  );
}

/** The panes of the open files, at the top of the window: their tab bars are the title bar.
 * `lead` is the width at the start of each tab bar that the window controls and the menu button
 * take. With no file open, the one text area shows what to do, under an empty bar. */
export function Panes(props: { font: () => Font; lead: () => number }) {
  const tabs = signal<TabState[]>([]);
  const layout = signal<DockAreaLayout>({ center: { type: "tabs", panels: [] } });
  // The panels that their groups display, so a hidden tab draws nothing.
  const shown = () => new Set(groups(layout().center).map((g) => g.panels[g.activeIndex ?? 0]));
  // The file that had the keys at the last change: a file that opens joins its pane.
  let near: string | undefined;
  const keyOf = (t: TabState) => String(t.id);
  const indexOf = (panel: string) => tabs().findIndex((t) => keyOf(t) === panel);

  /** Rust's tabs follow the order of the panes, so the next and previous file follow what
   * shows. */
  const order = (l: DockAreaLayout) => {
    const keys = panelsOf(l);
    if (keys.join(",") !== tabs().map(keyOf).join(",")) Tabs.order(keys.map(Number));
  };

  // The number of the last pane ask carried out. The ask that shows at the start is old.
  let asked: number | undefined;

  /** Carries out pane command `cmd` on the pane of `active`, from `l`. */
  const pane = (l: DockAreaLayout, cmd: string, active: string): DockAreaLayout => {
    if (cmd === "pane-next" || cmd === "pane-prev") {
      const to = neighbour(l, active, cmd === "pane-next" ? 1 : -1);
      if (to === undefined) Tabs.say("only one pane");
      // The press of a pane's tab does the same: its file gets the keys.
      else Tabs.showFile(Number(to));
      return l;
    }
    const next = cmd === "pane-close" ? closeGroup(l, active) : split(l, active, cmd === "split-down" ? "vertical" : "horizontal");
    if (!next) Tabs.say(cmd === "pane-close" ? "only one pane" : "one tab in this pane: open another file to split");
    return next ?? l;
  };

  Tabs.onChange((s) => {
    tabs.set(s.tabs);
    const keys = s.tabs.map(keyOf);
    const on = s.tabs.find((t) => t.on);
    const active = on ? keyOf(on) : undefined;
    let next = sync(layout(), keys, active, near);
    near = active;
    const ask = s.pane;
    if (ask && asked !== undefined && ask.seq !== asked && active !== undefined) next = pane(next, ask.cmd, active);
    asked = ask?.seq ?? 0;
    if (!sameLayout(next, layout())) {
      layout.set(next);
      order(next);
    }
  });

  return (
    <View class="flex-1 min-w-0 min-h-0">
      <If
        when={tabs().length > 0}
        else={
          <>
            <EmptyBar lead={props.lead()} />
            <TextArea />
          </>
        }
      >
        <DockArea
          class="flex-1 bg-background"
          layout={layout()}
          onLayoutChange={(e: { layout: DockAreaLayout }) => {
            layout.set(e.layout);
            order(e.layout);
            // A tab that its group comes to display in place of the active file has the keys:
            // a press on the tab, or a tab dropped on that group. A tab dropped elsewhere keeps
            // them.
            const on = tabs().find((t) => t.on);
            if (!on || shown().has(keyOf(on))) return;
            const group = groups(e.layout.center).find((g) => g.panels.includes(keyOf(on)));
            const next = group?.panels[group.activeIndex ?? 0];
            if (next !== undefined) then(() => Tabs.showFile(Number(next)));
          }}
          panels={tabs().map((t) => ({ id: keyOf(t), zoomable: false }))}
          renderPanel={(p) => <TextArea doc={Number(p.id)} shown={shown().has(p.id)} />}
          renderTab={(t: DockTab) => (
            <Tab tab={() => tabs()[indexOf(t.panel)]} index={() => indexOf(t.panel)} font={props.font} />
          )}
          renderToolbar={(t: DockTab) => <BarEnd controls={topRight(layout().center)?.panels.includes(t.panel) ?? false} />}
          renderDragPreview={(t: DockTab) => (
            <View class="flex-row items-center h-9 px-3 bg-surface-raised border border-border opacity-90">
              <Label font={props.font} on text={tabs()[indexOf(t.panel)]?.name ?? ""} />
            </View>
          )}
          groupClass="bg-background"
          tabBarClass="flex-row h-9 bg-muted border-b border-border"
          tabBarStyle={{ paddingLeft: props.lead() }}
          tabClass="h-9 cursor-pointer selected:bg-surface-raised hover:bg-surface-raised/50"
          contentClass="bg-background"
          dropIndicatorClass="bg-primary/15 border border-primary"
          handleClass="bg-border"
          handleAccessibilityLabel="Resize panes"
        />
      </If>
    </View>
  );
}
